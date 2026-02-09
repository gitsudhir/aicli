mod build_prompt;
mod chunk_text;
mod config;
mod embed_chunks;
mod embed_query;
mod generate;
mod http;
mod retrieve_chunks;
mod scan_files;
mod store_qdrant;

pub use build_prompt::{build_prompt_with_context, Message};
pub use config::Config;

use chunk_text::chunk_text;
use embed_chunks::embed_texts;
use embed_query::embed_query;
use generate::generate_answer;
use retrieve_chunks::retrieve_top;
use scan_files::scan_files;
use store_qdrant::{ensure_collection, store_points, Point, PointPayload};

pub fn index_corpus(cfg: &Config, source: Option<&str>) -> Result<(), String> {
    let files = scan_files(cfg, source);
    if files.is_empty() {
        return Ok(());
    }

    let mut next_id: i64 = 1;
    let mut collection_ready = false;

    for (path, text) in files {
        let chunks = chunk_text(&text, cfg);
        if chunks.is_empty() {
            continue;
        }
        let vectors = embed_texts(cfg, &chunks)?;
        if vectors.is_empty() {
            continue;
        }
        if !collection_ready {
            ensure_collection(cfg, vectors[0].len())?;
            collection_ready = true;
        }

        let mut points = Vec::new();
        for (idx, (chunk, vector)) in chunks.iter().cloned().zip(vectors).enumerate() {
            points.push(Point {
                id: next_id,
                vector,
                payload: PointPayload {
                    path: path.clone(),
                    index: idx,
                    chunk,
                },
            });
            next_id += 1;
        }
        store_points(cfg, &points)?;
    }

    Ok(())
}

pub fn answer_query(cfg: &Config, question: &str) -> Result<(String, String), String> {
    let query_vec = embed_query(cfg, question)?;
    let hits = retrieve_top(cfg, &query_vec)?;
    let (messages, context) = build_prompt_with_context(cfg, question, &hits);
    let answer = generate_answer(cfg, &messages)?;
    Ok((context, answer))
}
