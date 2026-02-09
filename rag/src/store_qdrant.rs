use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::http::{get_json, put_json};

#[derive(Serialize)]
struct CreateCollection {
    vectors: VectorParams,
}

#[derive(Serialize)]
struct VectorParams {
    size: usize,
    distance: String,
}

#[derive(Serialize)]
pub struct PointPayload {
    pub path: String,
    pub index: usize,
    pub chunk: String,
}

#[derive(Serialize)]
pub struct Point {
    pub id: i64,
    pub vector: Vec<f32>,
    pub payload: PointPayload,
}

#[derive(Serialize)]
struct UpsertPoints<'a> {
    points: &'a [Point],
}

#[derive(Deserialize)]
struct QdrantResponse {
    _result: Option<serde_json::Value>,
}

pub fn ensure_collection(cfg: &Config, vector_size: usize) -> Result<(), String> {
    let url = format!("{}/collections/{}", cfg.qdrant_url, cfg.collection);
    let exists = get_json::<serde_json::Value>(&url).is_ok();
    if exists {
        return Ok(());
    }
    let body = CreateCollection {
        vectors: VectorParams {
            size: vector_size,
            distance: cfg.distance.clone(),
        },
    };
    let _ = put_json::<QdrantResponse, _>(&url, &body)?;
    Ok(())
}

pub fn store_points(cfg: &Config, points: &[Point]) -> Result<(), String> {
    if points.is_empty() {
        return Ok(());
    }
    let url = format!("{}/collections/{}/points", cfg.qdrant_url, cfg.collection);
    let body = UpsertPoints { points };
    let _ = put_json::<QdrantResponse, _>(&url, &body)?;
    Ok(())
}
