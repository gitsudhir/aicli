use crate::config::Config;

pub fn chunk_text(text: &str, cfg: &Config) -> Vec<String> {
    let size = cfg.chunk_size;
    let mut overlap = cfg.chunk_overlap;

    if size == 0 {
        return vec![text.to_string()];
    }
    if overlap >= size {
        overlap = size / 4;
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    let chars: Vec<char> = text.chars().collect();
    let len_chars = chars.len();

    while start < len_chars {
        let end = (start + size).min(len_chars);
        let chunk_str: String = chars[start..end].iter().collect();
        let trimmed = chunk_str.trim();
        if !trimmed.is_empty() {
            chunks.push(trimmed.to_string());
        }
        if end == len_chars {
            break;
        }
        start = end.saturating_sub(overlap);
    }

    chunks
}
