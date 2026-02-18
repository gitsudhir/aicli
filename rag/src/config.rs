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
    pub hybrid_system_prompt: String,
    pub mcp_url: String,
    pub mcp_command: String,
    pub mcp_args: Vec<String>,
    pub agent_max_steps: usize,
}

impl Config {
    pub fn from_env() -> Self {
        // Load .env if present so MCP and model config work without manual `source .env`.
        let _ = dotenvy::dotenv();
        let include_exts = env::var("RAG_INCLUDE_EXTS").unwrap_or_else(|_| {
            ".rs,.md,.txt,.toml,.json,.yaml,.yml,.py,.js,.ts,.tsx,.html,.css".to_string()
        });
        Self {
            source_dir: env::var("RAG_SOURCE_DIR").unwrap_or_else(|_| "./".to_string()),
            include_exts: include_exts.split(',').map(|s| s.trim().to_string()).collect(),
            exclude_dirs: env::var("RAG_EXCLUDE_DIRS")
                .unwrap_or_else(|_| ".git,target,node_modules,.idea,.vscode,dist,build,qdrant_storage,.qoder".to_string())
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
            collection: env::var("QDRANT_COLLECTION").unwrap_or_else(|_| {
                let repo = current_folder_name().unwrap_or_else(|| "default".to_string());
                format!("{}_rag_chunks", sanitize_collection_name(&repo))
            }),
            distance: env::var("QDRANT_DISTANCE").unwrap_or_else(|_| "Cosine".to_string()),
            top_k: env::var("RAG_TOP_K").ok().and_then(|v| v.parse().ok()).unwrap_or(5),
            system_prompt: env::var("RAG_SYSTEM_PROMPT").unwrap_or_else(|_| {
                "You are a helpful coding assistant. Use only the provided context.".to_string()
            }),
            hybrid_system_prompt: env::var("RAG_HYBRID_SYSTEM_PROMPT").unwrap_or_else(|_| {
                "You are a hybrid AI agent.\n\nYou can:\n- Retrieve knowledge from documents.\n- Call MCP tools.\n- Fetch MCP prompts.\n- Read MCP resources.\n- Answer directly if no external action is required.\n\nAlways respond in valid JSON with one action:\nretrieve | tool | prompt | resource | final\n\nDo not output plain text.".to_string()
            }),
            mcp_url: env::var("MCP_URL").unwrap_or_default(),
            mcp_command: env::var("MCP_COMMAND").unwrap_or_default(),
            mcp_args: env::var("MCP_ARGS")
                .unwrap_or_default()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect(),
            agent_max_steps: env::var("RAG_AGENT_MAX_STEPS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }
}

fn current_folder_name() -> Option<String> {
    let cwd = env::current_dir().ok()?;
    cwd.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

fn sanitize_collection_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
        } else if c.is_ascii_whitespace() || c == '.' {
            out.push('_');
        }
    }
    if out.is_empty() { "default".to_string() } else { out }
}
