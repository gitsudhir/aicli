use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::config::Config;

pub fn scan_files(cfg: &Config, source_dir: Option<&str>) -> Vec<(String, String)> {
    let base = source_dir.unwrap_or(&cfg.source_dir);
    let mut results = Vec::new();

    let walker = WalkDir::new(base).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !cfg.exclude_dirs.iter().any(|d| d == &name)
    });

    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_text_file(path, &cfg.include_exts) {
            continue;
        }
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() > cfg.max_file_bytes {
                continue;
            }
        }
        let text = fs::read_to_string(path).unwrap_or_default();
        if text.trim().is_empty() {
            continue;
        }
        results.push((path.to_string_lossy().to_string(), text));
    }

    results
}

fn is_text_file(path: &Path, exts: &[String]) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    exts.iter().any(|ext| lower.ends_with(ext))
}
