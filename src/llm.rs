use crate::git;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;

fn print_banner(title: &str) {
    let max_width = 100;
    let min_width = 60;
    let padding = 4;
    let raw_width = title.len() + padding * 2;
    let total_width = std::cmp::min(max_width, std::cmp::max(min_width, raw_width));
    let banner_line = "=".repeat(total_width);

    let title_padding = (total_width - title.len()) / 2;

    println!("{}", banner_line);
    println!(
        "{}{}{}",
        " ".repeat(title_padding),
        title,
        " ".repeat(total_width - title_padding - title.len())
    );
    println!("{}", banner_line);
}

pub fn ai_commit(apply: bool) -> Result<(), Box<dyn Error>> {
    let path = env::current_dir().map_err(|_| "Failed to get current directory")?;
    let repo = git::Repo::new(&path);
    let changes = repo.get_staged_git_changes()?;
    let config = crate::config::load_config()?;
    let messages = build_prompt_messages(&changes, config.deepseek.prompt);

    print_banner("AI Suggested Commit Message");

    let rt = tokio::runtime::Runtime::new()?;
    let mut full_message = String::new();
    let mut current_line = String::new();

    rt.block_on(async {
        stream_commit_message(
            &config.deepseek.api_key,
            messages,
            config.deepseek.temperature,
            |content| {
                for ch in content.chars() {
                    current_line.push(ch);
                    if ch == '\n' {
                        print!("| {}\n", current_line.trim_end());
                        full_message.push_str(&current_line);
                        current_line.clear();
                    }
                }
            },
        )
        .await
    })?;

    if !current_line.trim().is_empty() {
        println!("| {}", current_line.trim_end());
        full_message.push_str(&current_line);
    }

    if apply {
        let commit_id = repo.commit(&full_message.trim())?;
        print_banner("✅ Commit Successful");
        println!("Commit ID: {}\n", commit_id);
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
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    mut callback: impl FnMut(String),
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let request_body = ChatRequest {
        model: "deepseek-chat".to_string(),
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
        return Err(format!("API failed: {}", err_msg).into());
    }

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_str = String::from_utf8_lossy(&chunk);

        for line in chunk_str.lines() {
            if line.starts_with("data:") && line != "data: [DONE]" {
                let json_str = &line[5..].trim();
                if let Ok(data) = serde_json::from_str::<StreamResponseChunk>(json_str) {
                    for choice in data.choices {
                        if let Some(content) = choice.delta.content {
                            callback(content);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
