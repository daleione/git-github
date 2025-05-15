use crate::git;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::env;

pub fn ai_commit(apply: bool) -> Result<(), Box<dyn Error>> {
    let path = env::current_dir().map_err(|_| "Failed to get current directory")?;
    let repo = git::Repo::new(&path);
    let changes = repo.get_staged_git_changes()?;
    let config = crate::config::load_config()?;
    let messages = build_prompt_messages(&changes, config.deepseek.prompt);

    println!("\nAI suggested commit message:");

    let rt = tokio::runtime::Runtime::new()?;

    let mut full_message = String::new();

    rt.block_on(async {
        stream_commit_message(
            &config.deepseek.api_key,
            messages,
            config.deepseek.temperature,
            |content| {
                print!("{}", content);
                full_message.push_str(&content);
            },
        )
        .await
    })?;

    if apply {
        println!("\n\nExecuting commit...");
        let commit_id = repo.commit(&full_message)?;
        println!("✅ Commit successful, ID: {}", commit_id);
    }
    Ok(())
}

fn build_prompt_messages(changes: &str, prompt_opt: Option<String>) -> Vec<ChatMessage> {
    let mut prompt = "You are a helpful assistant that generates clear and concise Git commit messages based on the changes provided.
Follow these rules:
1. Use imperative mood (e.g., 'Fix bug' not 'Fixed bug')
2. Keep it short (50 chars or less) for the title
3. Optionally add a longer description after a blank line
4. Focus on what changed, not why".to_string();
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
