use crate::error::{Error, Result};
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// One message in a chat completion request.
#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }
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

/// Stream a chat completion live and return the full collected message.
///
/// A spinner animates `title` while we wait for the model; the first token
/// settles it into a static header, then the body is revealed character by
/// character under a `│` gutter. Runs the async request on a local runtime so
/// callers stay synchronous.
pub fn stream_and_collect(
    title: &str,
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
) -> Result<String> {
    use std::io::Write;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut full_message = String::new();
    // `Some` until the first token settles the header; `take`n exactly once.
    let mut spinner = Some(crate::style::Spinner::start(title));
    let mut at_line_start = true;

    rt.block_on(async {
        stream_chat(api_key, model, messages, temperature, |content| {
            // First content arriving stops the spinner and prints the header.
            if let Some(spinner) = spinner.take() {
                spinner.finish();
            }
            for ch in content.chars() {
                full_message.push(ch);
                if at_line_start {
                    print!("{} ", crate::style::gutter());
                    at_line_start = false;
                }
                if ch == '\n' {
                    println!();
                    at_line_start = true;
                } else {
                    print!("{ch}");
                }
                // Flush and pace each character so it reveals one at a time.
                let _ = std::io::stdout().flush();
                crate::style::tick();
            }
        })
        .await
    })?;

    // An empty response never settled the spinner; do it so the header shows.
    if let Some(spinner) = spinner.take() {
        spinner.finish();
    }
    // Close the final line when the body didn't end with a newline.
    if !at_line_start {
        println!();
    }

    Ok(full_message)
}

async fn stream_chat(
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
