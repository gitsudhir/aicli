use serde::{Deserialize, Serialize};

use crate::build_prompt::Message;
use crate::config::Config;
use crate::http::post_json;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    stream: bool,
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
    let url = format!("{}/api/chat", cfg.ollama_url);
    let req = ChatRequest {
        model: &cfg.chat_model,
        messages,
        stream: false,
    };
    let res = post_json::<ChatResponse, _>(&url, &req)?;
    Ok(res.message.and_then(|m| m.content).unwrap_or_default())
}
