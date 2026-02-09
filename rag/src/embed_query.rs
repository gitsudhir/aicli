use crate::config::Config;
use crate::embed_chunks::embed_texts;

pub fn embed_query(cfg: &Config, text: &str) -> Result<Vec<f32>, String> {
    let vecs = embed_texts(cfg, &[text.to_string()])?;
    Ok(vecs.into_iter().next().unwrap_or_default())
}
