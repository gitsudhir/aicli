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
    let length = text.len();

    while start < length {
        let end = (start + size).min(length);
        let chunk = text[start..end].trim();
        if !chunk.is_empty() {
            chunks.push(chunk.to_string());
        }
        if end == length {
            break;
        }
        start = end.saturating_sub(overlap);
    }

    chunks
}
