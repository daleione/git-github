use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::repo::Repo;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::Command;
use std::time::Duration;

fn print_banner(title: &str) {
    let max_width = 100;
    let min_width = 60;
    let padding = 4;
    // Count characters, not bytes, so multibyte titles (e.g. "✅ …") still
    // size and center the banner correctly.
    let title_width = title.chars().count();
    let raw_width = title_width + padding * 2;
    let total_width = std::cmp::min(max_width, std::cmp::max(min_width, raw_width));
    let banner_line = "=".repeat(total_width);

    let title_padding = (total_width - title_width) / 2;

    println!("{}", banner_line);
    println!(
        "{}{}{}",
        " ".repeat(title_padding),
        title,
        " ".repeat(total_width - title_padding - title_width)
    );
    println!("{}", banner_line);
}

/// Shared setup for every commit entry point: open the repo, optionally stage,
/// and load a config with a usable API key.
fn prepare(stage: bool) -> Result<(Repo, String, AppConfig)> {
    let path = env::current_dir().map_err(|_| Error::NoCurrentDir)?;
    let repo = Repo::new(&path)?;

    if stage {
        repo.stage_all()?;
    }

    let changes = repo.get_staged_git_changes()?;
    let config = crate::config::load_config()?;
    if config.deepseek.api_key.is_empty() {
        return Err(Error::NoApiKey);
    }

    Ok((repo, changes, config))
}

/// What to do with the AI-generated message once it has been produced.
pub enum CommitMode {
    Preview,
    Apply,
    Editor,
}

/// Stream the AI response, echoing each completed line as `| ...`, and return
/// the full message.
fn stream_and_collect(
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
) -> Result<String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut full_message = String::new();
    let mut current_line = String::new();

    rt.block_on(async {
        stream_commit_message(api_key, model, messages, temperature, |content| {
            for ch in content.chars() {
                current_line.push(ch);
                if ch == '\n' {
                    println!("| {}", current_line.trim_end());
                    full_message.push_str(&current_line);
                    current_line.clear();
                }
            }
        })
        .await
    })?;

    if !current_line.trim().is_empty() {
        println!("| {}", current_line.trim_end());
        full_message.push_str(&current_line);
    }

    Ok(full_message)
}

/// Generate a commit message from the staged changes and act on it per `mode`.
pub fn run(stage: bool, mode: CommitMode) -> Result<()> {
    let (repo, changes, config) = prepare(stage)?;
    let messages = build_prompt_messages(&changes, config.deepseek.prompt);

    print_banner(match mode {
        CommitMode::Editor => "AI Generating Commit Message",
        _ => "AI Suggested Commit Message",
    });

    let model = config.deepseek.model.as_deref().unwrap_or("deepseek-chat");
    let message = stream_and_collect(
        &config.deepseek.api_key,
        model,
        messages,
        config.deepseek.temperature,
    )?;

    if !matches!(mode, CommitMode::Preview) && message.trim().is_empty() {
        return Err(Error::EmptyMessage);
    }

    match mode {
        CommitMode::Preview => {}
        CommitMode::Apply => {
            commit_via_git(&message, false)?;
            print_banner("✅ Commit Successful");
            println!("Commit ID: {}\n", repo.head_commit_id()?);
        }
        CommitMode::Editor => {
            commit_via_git(&message, true)?;
            print_banner("✅ Commit Completed");
        }
    }

    Ok(())
}

/// Commit the staged changes via `git commit -F`, so pre-commit/commit-msg
/// hooks and signing run (libgit2 would skip them). When `edit` is set, `-e`
/// opens the editor on the seeded message first.
fn commit_via_git(message: &str, edit: bool) -> Result<()> {
    // Process-unique name so concurrent runs don't clobber each other.
    let temp_file = env::temp_dir().join(format!("git-github-commit-{}.txt", std::process::id()));
    fs::write(&temp_file, message.trim())?;

    let mut command = Command::new("git");
    command.arg("commit");
    if edit {
        print_banner("Opening Editor for Review");
        // `-e` opens the editor to edit the seeded message; `-F` (below) does
        // not abort when the message is left unchanged, unlike `--template`.
        command.args(["-e", "-v"]);
    }
    let status = command
        .arg("-F")
        .arg(&temp_file)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status();

    // Clean up regardless of how git exited.
    let _ = fs::remove_file(&temp_file);

    if !status?.success() {
        return Err(Error::CommitCancelled);
    }

    Ok(())
}

fn build_prompt_messages(changes: &str, prompt_opt: Option<String>) -> Vec<ChatMessage> {
    let mut prompt = r#"
You are an AI commit message assistant.

Please generate a commit message with the following format:
1. Title (one short sentence, 50-72 characters max).
2. A clear bullet-point list of changes (start each line with "- ").
3. Each line, including bullets, should be under 100 characters.
4. Keep it concise, consistent, and professional.

Example:

Improve error handling in user authentication

- Add detailed error messages for login failures
- Handle timeout errors gracefully
- Refactor error propagation logic for clarity
"#
    .to_string();
    if let Some(config_prompt) = prompt_opt {
        prompt = config_prompt;
    }

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Here are my current Git changes:\n{}", changes),
        },
    ]
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct StreamResponseChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: DeltaMessage,
}

#[derive(Debug, Deserialize)]
struct DeltaMessage {
    content: Option<String>,
}

async fn stream_commit_message(
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    mut callback: impl FnMut(String),
) -> Result<()> {
    // `connect_timeout` bounds reaching the API; `read_timeout` is an idle
    // timeout between stream reads, so a long-but-active stream is not cut off
    // while a stalled connection still fails fast.
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(60))
        .build()?;
    let request_body = ChatRequest {
        model: model.to_string(),
        messages,
        stream: true,
        temperature,
    };

    let response = client
        .post("https://api.deepseek.com/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let err_msg = response.text().await?;
        return Err(Error::ApiError(err_msg));
    }

    // SSE events are newline-delimited, but `bytes_stream` yields arbitrary
    // network chunks: a single `data:` line (or a multibyte char) may straddle
    // two chunks. Buffer bytes and only parse whole lines so nothing is lost.
    let mut stream = response.bytes_stream();
    let mut buffer: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        buffer.extend_from_slice(&chunk?);
        drain_sse_lines(&mut buffer, &mut callback);
    }

    Ok(())
}

/// Parse every complete (newline-terminated) SSE line in `buffer`, invoking
/// `callback` for each content delta. Any trailing partial line is left in
/// `buffer` for the next chunk.
fn drain_sse_lines(buffer: &mut Vec<u8>, callback: &mut impl FnMut(String)) {
    while let Some(newline) = buffer.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buffer.drain(..=newline).collect();
        let line = String::from_utf8_lossy(&line);
        let line = line.trim();

        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim();
            if data == "[DONE]" {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<StreamResponseChunk>(data) {
                for choice in parsed.choices {
                    if let Some(content) = choice.delta.content {
                        callback(content);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::drain_sse_lines;

    /// Feed an SSE stream one byte at a time and confirm every content delta is
    /// recovered — i.e. lines and multibyte chars split across chunks are not
    /// lost. Also checks the trailing partial line stays buffered.
    fn collect_byte_by_byte(stream: &[u8]) -> (String, Vec<u8>) {
        let mut buffer = Vec::new();
        let mut out = String::new();
        let mut push = |c: String| out.push_str(&c);
        for &byte in stream {
            buffer.push(byte);
            drain_sse_lines(&mut buffer, &mut push);
        }
        (out, buffer)
    }

    #[test]
    fn reassembles_split_lines_and_multibyte() {
        let stream = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello \"}}]}\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"世界\"}}]}\n",
            "data: [DONE]\n",
        )
        .as_bytes();

        let (out, buffer) = collect_byte_by_byte(stream);
        assert_eq!(out, "Hello 世界");
        assert!(buffer.is_empty());
    }

    #[test]
    fn keeps_trailing_partial_line_buffered() {
        let mut buffer = Vec::new();
        let mut out = String::new();
        let mut push = |c: String| out.push_str(&c);

        // A complete line plus the start of the next one.
        buffer.extend_from_slice(
            b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\ndata: {\"choi",
        );
        drain_sse_lines(&mut buffer, &mut push);

        assert_eq!(out, "hi");
        assert_eq!(buffer, b"data: {\"choi");
    }
}
