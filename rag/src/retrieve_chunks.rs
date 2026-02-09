use serde::Deserialize;

use crate::config::Config;
use crate::http::post_json;

#[derive(Deserialize, Clone)]
pub struct Hit {
    pub payload: Option<Payload>,
}

#[derive(Deserialize, Clone)]
pub struct Payload {
    pub path: Option<String>,
    pub index: Option<usize>,
    pub chunk: Option<String>,
}

#[derive(Deserialize)]
struct QueryResponse {
    result: Option<QueryResult>,
}

#[derive(Deserialize)]
struct QueryResult {
    points: Vec<Hit>,
}

#[derive(serde::Serialize)]
struct QueryRequest<'a> {
    query: &'a [f32],
    limit: usize,
    with_payload: bool,
}

pub fn retrieve_top(cfg: &Config, vector: &[f32]) -> Result<Vec<Hit>, String> {
    if vector.is_empty() {
        return Ok(vec![]);
    }
    let url = format!("{}/collections/{}/points/query", cfg.qdrant_url, cfg.collection);
    let req = QueryRequest {
        query: vector,
        limit: cfg.top_k,
        with_payload: true,
    };
    let res = post_json::<QueryResponse, _>(&url, &req)?;
    Ok(res
        .result
        .map(|r| r.points)
        .unwrap_or_default())
}
