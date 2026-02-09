use serde::Serialize;
use serde_json::Value;

use crate::config::Config;
use crate::http::post_json;

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a [String],
}

#[derive(Serialize)]
struct EmbedLegacyRequest<'a> {
    model: &'a str,
    prompt: &'a [String],
}

pub fn embed_texts(cfg: &Config, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
    if texts.is_empty() {
        return Ok(vec![]);
    }
    let url = format!("{}/api/embed", cfg.ollama_url);
    let req = EmbedRequest {
        model: &cfg.embed_model,
        input: texts,
    };
    match post_json::<Value, _>(&url, &req) {
        Ok(res) => parse_embeddings(res),
        Err(_) => {
            let url = format!("{}/api/embeddings", cfg.ollama_url);
            let req = EmbedLegacyRequest {
                model: &cfg.embed_model,
                prompt: texts,
            };
            let res = post_json::<Value, _>(&url, &req)?;
            parse_embeddings(res)
        }
    }
}

fn parse_embeddings(value: Value) -> Result<Vec<Vec<f32>>, String> {
    if let Some(embeddings) = value.get("embeddings") {
        return parse_embeddings_value(embeddings);
    }
    if let Some(embedding) = value.get("embedding") {
        return parse_embeddings_value(embedding);
    }
    Err("No embeddings in response".to_string())
}

fn parse_embeddings_value(value: &Value) -> Result<Vec<Vec<f32>>, String> {
    if let Some(arr) = value.as_array() {
        if arr.is_empty() {
            return Ok(vec![]);
        }
        if arr[0].is_array() {
            let mut out = Vec::new();
            for row in arr {
                out.push(parse_vec(row)?);
            }
            return Ok(out);
        }
        return Ok(vec![parse_vec(value)?]);
    }
    Err("Invalid embeddings format".to_string())
}

fn parse_vec(value: &Value) -> Result<Vec<f32>, String> {
    let arr = value.as_array().ok_or("Embedding is not an array")?;
    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let n = v.as_f64().ok_or("Embedding value is not a number")?;
        out.push(n as f32);
    }
    Ok(out)
}
