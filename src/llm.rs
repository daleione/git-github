use crate::git;
use std::env;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// 主函数：生成 AI Commit 信息
pub fn ai_commit() -> Result<(), Box<dyn Error>> {
    let path = env::current_dir().map_err(|_| "无法获取当前目录")?;
    let repo = git::Repo::new(&path);
    let changes = repo.get_staged_git_changes()?;

    let messages = build_prompt_messages(&changes);

    // println!("以下是检测到的 Git 改动：\n{}", changes);
    println!("\nAI 建议的 Commit 信息：");

    let rt = tokio::runtime::Runtime::new()?;
    let config = crate::config::load_config()?; // 加载配置
    rt.block_on(async {
        stream_commit_message(
            &config.deepseek.api_key,
            messages,
            config.deepseek.temperature,
            |content| {
                print!("{}", content);
            })
        .await
    })?;

    Ok(())
}

/// 构造 ChatMessage 请求体
fn build_prompt_messages(changes: &str) -> Vec<ChatMessage> {
    vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a helpful assistant that generates clear and concise Git commit messages based on the changes provided. Follow these rules:\n1. Use imperative mood (e.g., 'Fix bug' not 'Fixed bug')\n2. Keep it short (50 chars or less) for the title\n3. Optionally add a longer description after a blank line\n4. Focus on what changed, not why".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Here are my current Git changes:\n{}", changes),
        },
    ]
}

/// Chat 请求结构体
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

/// Chat 响应结构体
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

/// 流式调用 DeepSeek API，返回 Commit 信息
async fn stream_commit_message(
    api_key: &str,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    callback: impl Fn(String),
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
        return Err(format!("API 请求失败: {}", err_msg).into());
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
