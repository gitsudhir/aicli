# AI CLI - Retrieval-Augmented Generation Terminal Interface

AI CLI is a terminal-based application that combines the power of large language models with Retrieval-Augmented Generation (RAG) capabilities. Built with Rust, it provides an interactive terminal interface for querying documents and running commands with contextual awareness.

## Features

### Core Functionality
- **Dual Input Modes**: Switch between RAG queries and direct command execution
- **Interactive Terminal UI**: Built with `ratatui` for a responsive terminal experience
- **Real-time Document Indexing**: Index local files and directories for RAG queries
- **Context-Aware Responses**: LLM responses enhanced with relevant document context
- **Multi-file Support**: Process various file types including text, code, and documentation

### RAG Capabilities
- **Document Scanning**: Automatically scans and processes files from specified directories
- **Text Chunking**: Intelligently breaks down documents into searchable chunks
- **Vector Embeddings**: Generates embeddings using configured embedding models
- **Qdrant Integration**: Uses Qdrant vector database for efficient similarity search
- **Context Retrieval**: Retrieves relevant document chunks based on query similarity

### User Interface
- **Split-screen Layout**: Dedicated areas for context and responses
- **Scrollable Views**: Navigate through long context and answer content
- **Keyboard Navigation**: Full keyboard control with intuitive shortcuts
- **Loading Indicators**: Visual feedback during processing operations
- **Mode Switching**: Toggle between RAG and command modes

## Architecture

The application consists of two main components:

### Main Application (`src/main.rs`)
- Terminal UI management using `ratatui` and `crossterm`
- Input handling and state management
- Thread-safe communication between UI and background operations
- Command execution capabilities

### RAG Library (`rag/`)
- Document processing and indexing pipeline
- Vector embedding generation
- Qdrant database integration
- Query processing and response generation

## Installation

### Prerequisites
- Rust toolchain (latest stable version)
- Qdrant vector database server
- Access to embedding model API (configured via environment)

### Setup

1. **Clone the repository:**
```bash
git clone <repository-url>
cd aicli
```

2. **Start Qdrant server:**
```bash
# Using Docker
docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant
```

3. **Configure environment variables:**
```bash
export QDRANT_URL="http://localhost:6334"
export EMBEDDING_MODEL_API_KEY="your-api-key"
export EMBEDDING_MODEL_URL="your-embedding-model-endpoint"
```

4. **Build the application:**
```bash
cargo build --release
```

5. **Run the application:**
```bash
cargo run
```

## Usage

### Basic Navigation
- **Tab**: Switch between RAG and Command modes
- **Ctrl+O**: Toggle focus between context and answer panels
- **Up/Down/PgUp/PgDn**: Scroll through content
- **Home/End**: Jump to beginning/end of content
- **Esc/Ctrl+C**: Exit the application

### RAG Mode
1. **Index Documents**: Press `Ctrl+R` or `F2` to index files from configured directories
2. **Ask Questions**: Type your query and press Enter to get context-aware responses
3. **View Context**: The top panel shows retrieved document chunks used for generation

### Command Mode
1. **Switch Mode**: Press Tab to enter Command mode
2. **Execute Commands**: Type shell commands and press Enter to execute them directly
3. **View Output**: Command results appear in the answer panel

## Configuration

The application uses environment variables for configuration:

```bash
# Qdrant database connection
export QDRANT_URL="http://localhost:6334"
export QDRANT_COLLECTION="midjourney"  # Default collection name

# Embedding model configuration
export EMBEDDING_MODEL_URL="your-embedding-api-endpoint"
export EMBEDDING_MODEL_API_KEY="your-api-key"

# File scanning configuration
export SCAN_DIRECTORIES="/path/to/documents,/another/path"
export FILE_EXTENSIONS=".txt,.md,.rs,.py,.js"  # Comma-separated list
```

## Project Structure

```
aicli/
├── src/
│   └── main.rs              # Main application entry point
├── rag/
│   ├── src/
│   │   ├── lib.rs           # RAG library main module
│   │   ├── build_prompt.rs  # Prompt construction with context
│   │   ├── chunk_text.rs    # Text chunking utilities
│   │   ├── config.rs        # Configuration management
│   │   ├── embed_chunks.rs  # Text embedding generation
│   │   ├── embed_query.rs   # Query embedding generation
│   │   ├── generate.rs      # Answer generation
│   │   ├── http.rs          # HTTP client utilities
│   │   ├── retrieve_chunks.rs # Similarity search and retrieval
│   │   ├── scan_files.rs    # File system scanning
│   │   └── store_qdrant.rs  # Qdrant database operations
│   └── Cargo.toml           # RAG library dependencies
├── Cargo.toml               # Main project dependencies
├── Cargo.lock               # Dependency lock file
└── README.md                # This file
```

## Dependencies

### Main Application
- `crossterm` - Terminal manipulation and event handling
- `ratatui` - Terminal user interface framework
- `rag` - Local RAG library (path dependency)

### RAG Library
- `reqwest` - HTTP client for API calls
- `serde` - Serialization/deserialization
- `serde_json` - JSON handling
- `walkdir` - File system traversal

## Development

### Building
```bash
# Development build
cargo build

# Release build
cargo build --release
```

### Running Tests
```bash
cargo test
```

### Code Structure Guidelines
- Follow Rust naming conventions
- Use descriptive variable and function names
- Maintain clear separation between UI and business logic
- Handle errors gracefully with meaningful error messages

## Troubleshooting

### Common Issues

1. **Qdrant Connection Failed**
   - Ensure Qdrant server is running on the configured port
   - Check `QDRANT_URL` environment variable
   - Verify network connectivity

2. **Embedding Generation Errors**
   - Check API key and endpoint configuration
   - Verify embedding model service availability
   - Ensure proper rate limiting compliance

3. **File Scanning Issues**
   - Verify directory permissions
   - Check `SCAN_DIRECTORIES` configuration
   - Ensure supported file extensions are configured

### Debugging
Enable debug logging by setting the environment variable:
```bash
export RUST_LOG=debug
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with the excellent `ratatui` terminal framework
- Powered by Qdrant vector database
- Inspired by modern AI-assisted development tools