use serde::{Deserialize, Serialize};

use crate::build_prompt::Message;
use crate::config::Config;
use crate::http::post_json;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: Option<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

pub fn generate_answer(cfg: &Config, messages: &[Message]) -> Result<String, String> {
    generate_chat(cfg, messages, None)
}

pub fn generate_json(cfg: &Config, messages: &[Message]) -> Result<String, String> {
    generate_chat(cfg, messages, Some("json"))
}

fn generate_chat(cfg: &Config, messages: &[Message], format: Option<&str>) -> Result<String, String> {
    let url = format!("{}/api/chat", cfg.ollama_url);
    let req = ChatRequest {
        model: &cfg.chat_model,
        messages,
        stream: false,
        format,
    };
    let res = post_json::<ChatResponse, _>(&url, &req)?;
    Ok(res.message.and_then(|m| m.content).unwrap_or_default())
}
