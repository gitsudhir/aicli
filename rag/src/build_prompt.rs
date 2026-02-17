use crate::config::Config;
use crate::retrieve_chunks::Hit;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub fn build_prompt_with_context(
    cfg: &Config,
    question: &str,
    hits: &[Hit],
) -> (Vec<Message>, String) {
    let context = format_context_from_hits(hits);

    let user_content = format!(
        "Use the context below to answer the question.\n\nContext:\n{}\n\nQuestion: {}",
        context, question
    );

    let messages = vec![
        Message { role: "system".to_string(), content: cfg.system_prompt.clone() },
        Message { role: "user".to_string(), content: user_content },
    ];

    (messages, context)
}

pub fn format_context_from_hits(hits: &[Hit]) -> String {
    let mut context_lines = Vec::new();
    for (i, hit) in hits.iter().enumerate() {
        let payload = hit.payload.as_ref();
        let path = payload
            .and_then(|p| p.path.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let index = payload
            .and_then(|p| p.index)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "?".to_string());
        let chunk = payload.and_then(|p| p.chunk.clone()).unwrap_or_default();
        context_lines.push(format!("[{}] {} (chunk {})\n{}", i + 1, path, index, chunk));
    }

    if context_lines.is_empty() {
        "(no context found)".to_string()
    } else {
        context_lines.join("\n\n")
    }
}
