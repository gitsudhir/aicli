use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub source_dir: String,
    pub include_exts: Vec<String>,
    pub exclude_dirs: Vec<String>,
    pub max_file_bytes: u64,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub ollama_url: String,
    pub embed_model: String,
    pub chat_model: String,
    pub qdrant_url: String,
    pub collection: String,
    pub distance: String,
    pub top_k: usize,
    pub system_prompt: String,
}

impl Config {
    pub fn from_env() -> Self {
        let include_exts = env::var("RAG_INCLUDE_EXTS").unwrap_or_else(|_| {
            ".rs,.md,.txt,.toml,.json,.yaml,.yml,.py,.js,.ts,.tsx,.html,.css".to_string()
        });
        Self {
            source_dir: env::var("RAG_SOURCE_DIR").unwrap_or_else(|_| "./".to_string()),
            include_exts: include_exts.split(',').map(|s| s.trim().to_string()).collect(),
            exclude_dirs: env::var("RAG_EXCLUDE_DIRS")
                .unwrap_or_else(|_| ".git,target,node_modules,.idea,.vscode,dist,build".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            max_file_bytes: env::var("RAG_MAX_FILE_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500_000),
            chunk_size: env::var("RAG_CHUNK_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1200),
            chunk_overlap: env::var("RAG_CHUNK_OVERLAP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(200),
            ollama_url: env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
            embed_model: env::var("OLLAMA_EMBED_MODEL").unwrap_or_else(|_| "nomic-embed-text".to_string()),
            chat_model: env::var("OLLAMA_CHAT_MODEL").unwrap_or_else(|_| "qwen2.5-coder:14b".to_string()),
            qdrant_url: env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6333".to_string()),
            collection: env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "rag_chunks".to_string()),
            distance: env::var("QDRANT_DISTANCE").unwrap_or_else(|_| "Cosine".to_string()),
            top_k: env::var("RAG_TOP_K").ok().and_then(|v| v.parse().ok()).unwrap_or(5),
            system_prompt: env::var("RAG_SYSTEM_PROMPT").unwrap_or_else(|_| {
                "You are a helpful coding assistant. Use only the provided context.".to_string()
            }),
        }
    }
}
