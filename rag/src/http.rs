use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::from_str;
use std::time::Duration;

pub fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client.get(url).send().map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("GET {} failed: {} {}", url, status, text));
    }
    from_str::<T>(&text).map_err(|e| format!("GET {} decode failed: {} | {}", url, e, text))
}

pub fn post_json<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .json(body)
        .send()
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("POST {} failed: {} {}", url, status, text));
    }
    from_str::<T>(&text).map_err(|e| format!("POST {} decode failed: {} | {}", url, e, text))
}

pub fn put_json<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .put(url)
        .header(CONTENT_TYPE, "application/json")
        .json(body)
        .send()
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("PUT {} failed: {} {}", url, status, text));
    }
    from_str::<T>(&text).map_err(|e| format!("PUT {} decode failed: {} | {}", url, e, text))
}
