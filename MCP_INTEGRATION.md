# MCP Integration Guide for AICLI

## 📋 Overview

This document provides a comprehensive guide for integrating Model Context Protocol (MCP) capabilities into your AICLI application. It covers the complete architecture, data flow, and integration patterns for connecting to MCP servers, discovering capabilities, and intelligently routing queries between MCP tools and RAG systems.

## 🏗️ Architecture Overview

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   AICLI Client  │◄──►│  MCP Client     │◄──►│  MCP Servers    │
│                 │    │                 │    │                 │
│ • Query Parser  │    │ • Tool Discovery│    │ • Tools         │
│ • Router        │    │ • Connection Mgmt│    │ • Resources     │
│ • LLM Interface │    │ • Request Handler│    │ • Prompts       │
│ • RAG System    │    │ • Response Parser│    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Ollama LLM    │    │   JSON-RPC 2.0  │    │ Stdio/HTTP Transport│
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## 🔄 Complete Data Flow

### 1. Query Processing Flow

```
User Query
    │
    ▼
┌─────────────────────────────────────┐
│        Query Analysis & Routing      │
│  ─────────────────────────────────  │
│  1. Parse user intent               │
│  2. Identify query type:            │
│     • Tool-based query              │
│     • Knowledge-based query         │
│     • Hybrid query                  │
│  3. Decision Engine:                │
│     • MCP vs RAG routing            │
│     • Tool selection                │
└─────────────────────────────────────┘
    │
    ├── If MCP Route ──────────────────┐
    │                                  │
    ▼                                  ▼
┌─────────────────┐          ┌─────────────────┐
│ Tool Discovery  │          │ Tool Execution  │
│                 │          │                 │
│ • List available│          │ • Validate input│
│   tools         │          │ • Call tool     │
│ • Match intent  │          │ • Process result│
│   to tools      │          │ • Format output │
└─────────────────┘          └─────────────────┘
    │                                  │
    ▼                                  ▼
┌─────────────────────────────────────────────────┐
│              Result Integration                 │
│  ─────────────────────────────────────────────  │
│  • Combine MCP results with context             │
│  • Format for LLM consumption                   │
│  • Add metadata and source information          │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│              LLM Processing                     │
│  ─────────────────────────────────────────────  │
│  • Send enriched context to Ollama              │
│  • Include tool results as additional context   │
│  • Generate final response                      │
└─────────────────────────────────────────────────┘
    │
    ▼
User Response
```

### 2. MCP Client Connection Flow

```
┌─────────────────────────────────────────────────┐
│           MCP Client Initialization             │
│  ─────────────────────────────────────────────  │
│  1. Load server configurations                  │
│  2. Establish transport connections             │
│  3. Send initialization handshake               │
│  4. Discover server capabilities                │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│           Capability Discovery                  │
│  ─────────────────────────────────────────────  │
│  • tools/list - Get available tools             │
│  • resources/list - Get available resources     │
│  • prompts/list - Get available prompts         │
│  • Cache capabilities for performance           │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│           Tool Registration & Metadata          │
│  ─────────────────────────────────────────────  │
│  • Parse tool schemas                           │
│  • Extract tool descriptions                    │
│  • Build tool capability index                  │
│  • Create tool-to-intent mapping                │
└─────────────────────────────────────────────────┘
```

## 🛠️ Core Components

### 1. Query Router & Decision Engine

The router determines whether to use MCP tools, RAG, or both based on:

**Decision Factors:**
- Query intent analysis
- Available tool capabilities
- Knowledge base relevance
- User preferences
- Performance considerations

**Routing Logic:**
```rust
use mcp_client_rust::{MCPClient, ClientInfo};
use serde_json::json;

async fn route_query(query: &str, client: &mut MCPClient) -> RouteResult {
    // Analyze query intent
    let intent = analyze_intent(query).await;
    
    // Check available MCP tools
    let mcp_tools = client.list_tools().await?;
    let matching_tools = find_matching_tools(&intent, &mcp_tools);
    
    // Check RAG relevance
    let rag_relevance = check_rag_relevance(query).await;
    
    // Decision matrix
    if !matching_tools.is_empty() && high_confidence_tool_match(&intent, &matching_tools) {
        Ok(Route::McpTool { tools: matching_tools })
    } else if rag_relevance > RAG_THRESHOLD {
        Ok(Route::Rag)
    } else if !matching_tools.is_empty() && rag_relevance > HYBRID_THRESHOLD {
        Ok(Route::Hybrid { tools: matching_tools })
    } else {
        Ok(Route::LlmDirect)
    }
}
```

### 2. Tool Discovery & Management

**Tool Discovery Process:**
1. **Initial Discovery**: On connection, query all servers for available tools
2. **Schema Parsing**: Extract tool names, descriptions, and input schemas
3. **Capability Indexing**: Build searchable index of tool capabilities
4. **Intent Mapping**: Map natural language intents to specific tools

**Tool Metadata Structure:**
```rust
// Using mcp-client-rust types
use mcp_client_rust::types::{Tool, ToolResult, JsonRpcRequest};

// Example tool definition from mcp-server-rust
let bmi_tool = Tool {
    name: "calculate-bmi".to_string(),
    description: Some("Calculates Body Mass Index from weight and height".to_string()),
    input_schema: json!({
        "type": "object",
        "properties": {
            "weightKg": {
                "type": "number",
                "description": "Weight in kilograms"
            },
            "heightM": {
                "type": "number", 
                "description": "Height in meters"
            }
        },
        "required": ["weightKg", "heightM"]
    }),
};

// Tool result structure
let tool_result = ToolResult {
    content: vec![ToolResultContent::Text { 
        text: "Your BMI is 22.9 (Normal range)".to_string() 
    }],
    is_error: Some(false),
};
```

### 3. MCP Client Integration

**Connection Management:**
- Multiple server support
- Automatic reconnection
- Connection pooling
- Health monitoring

**Request/Response Handling:**
```rust
use mcp_client_rust::{
    MCPClient, ClientInfo, ClientResult,
    transport::StdioTransport,
    types::{Tool, ToolResult}
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AICLIIntegration {
    mcp_client: Arc<Mutex<MCPClient>>,
    server_configs: Vec<ServerConfig>,
}

impl AICLIIntegration {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let configs = load_server_configs().await?;
        let mut clients = Vec::new();
        
        // Connect to all configured MCP servers
        for config in &configs {
            let transport = StdioTransport::new(
                &config.command,
                &config.args
            )?;
            
            let client_info = ClientInfo {
                name: "AICLI".to_string(),
                version: "1.0.0".to_string(),
            };
            
            let mut client = MCPClient::new(Arc::new(transport), client_info);
            client.initialize().await?;
            clients.push(client);
        }
        
        Ok(Self {
            mcp_client: Arc::new(Mutex::new(clients.remove(0))), // Primary client
            server_configs: configs,
        })
    }
    
    pub async fn discover_all_tools(&self) -> ClientResult<Vec<Tool>> {
        let mut all_tools = Vec::new();
        let client = self.mcp_client.lock().await;
        
        // Get tools from primary server
        let tools = client.list_tools().await?;
        all_tools.extend(tools);
        
        // Add tools from other servers if needed
        // This would involve managing multiple client connections
        
        Ok(all_tools)
    }
    
    pub async fn execute_tool(
        &self, 
        tool_name: &str, 
        arguments: serde_json::Value
    ) -> ClientResult<ToolResult> {
        let mut client = self.mcp_client.lock().await;
        client.call_tool(tool_name, arguments).await
    }
}
```

### 4. RAG Integration

**Knowledge Base Management:**
- Document indexing and retrieval
- Semantic search capabilities
- Context window management
- Source attribution

**RAG Query Processing:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RagResponse {
    pub content: String,
    pub sources: Vec<String>,
    pub confidence: f64,
}

pub struct RagSystem {
    vector_store: VectorStore,
    retriever: SemanticRetriever,
}

impl RagSystem {
    pub async fn process_query(&self, query: &str) -> Result<RagResponse, Box<dyn std::error::Error>> {
        // Retrieve relevant documents
        let documents = self.retriever.search(query, 5).await?;
        
        // Extract relevant context
        let context = self.extract_context(&documents, query).await?;
        
        // Generate response with citations
        Ok(RagResponse {
            content: context.text,
            sources: context.sources,
            confidence: context.relevance_score,
        })
    }
}
```

## 🔄 Integration Flow Details

### 1. Tool Discovery Process

```
Startup Sequence:
┌─────────────────────────────────────────────────┐
│ 1. Load MCP server configurations               │
│    • Server addresses and transport types       │
│    • Authentication credentials                 │
│    • Connection preferences                     │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 2. Establish Server Connections                 │
│    • Initialize transport layers                │
│    • Send protocol initialization               │
│    • Verify server compatibility                │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 3. Discover Capabilities                        │
│    • Request tools/list from each server        │
│    • Request resources/list                     │
│    • Request prompts/list                       │
│    • Parse and validate schemas                 │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 4. Build Capability Index                       │
│    • Create tool name lookup                    │
│    • Build category-based indexing              │
│    • Generate intent-to-tool mappings           │
│    • Cache for performance                      │
└─────────────────────────────────────────────────┘
```

### 2. Query Processing Pipeline

```
Query Processing Steps:
┌─────────────────────────────────────────────────┐
│ 1. Query Analysis                               │
│    • Parse natural language input               │
│    • Extract key entities and intent            │
│    • Identify required parameters               │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 2. Route Decision                               │
│    • Check tool availability                    │
│    • Evaluate RAG relevance                     │
│    • Apply routing rules                        │
│    • Select processing path                     │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 3. MCP Tool Execution (if routed)               │
│    • Validate tool parameters                   │
│    • Execute tool call                          │
│    • Process and format results                 │
│    • Handle errors and fallbacks                │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 4. Context Integration                          │
│    • Combine tool results with query context    │
│    • Add relevant RAG content if hybrid         │
│    • Format for LLM consumption                 │
│    • Include metadata and sources               │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ 5. LLM Processing                               │
│    • Send enriched context to Ollama            │
│    • Include tool results as additional context │
│    • Generate natural language response         │
│    • Apply response formatting                  │
└─────────────────────────────────────────────────┘
```

### 3. Tool Result Integration with LLM

```
Result Integration Process:
┌─────────────────────────────────────────────────┐
│ Tool Execution Results                          │
│ {                                               │
│   "content": [                                 │
│     {"text": "Your BMI is 22.9 (Normal range)"}│
│   ],                                           │
│   "isError": false,                            │
│   "source": "health-calculator-server"         │
│ }                                              │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ Context Enrichment                              │
│ {                                               │
│   "query": "What's my BMI if I weigh 70kg?",   │
│   "tool_results": [                            │
│     {                                          │
│       "tool": "calculate-bmi",                 │
│       "result": "BMI: 22.9 (Normal)",          │
│       "confidence": 0.95                       │
│     }                                          │
│   ],                                           │
│   "additional_context": "BMI ranges: ...",     │
│   "user_context": "..."                        │
│ }                                              │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ LLM Prompt Construction                         │
│                                                 │
│ System: You are a helpful AI assistant with    │
│ access to calculation tools. Use the provided  │
│ tool results to give accurate answers.         │
│                                                 │
│ User: What's my BMI if I weigh 70kg?           │
│                                                 │
│ Tool Results:                                  │
│ - calculate-bmi: BMI 22.9 (Normal range)       │
│                                                 │
│ Additional Context:                            │
│ BMI under 18.5 = Underweight                   │
│ BMI 18.5-24.9 = Normal                         │
│ BMI 25-29.9 = Overweight                       │
└─────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────┐
│ Ollama Response Generation                      │
│                                                 │
│ Based on your weight of 70kg and assuming a    │
│ standard height, your BMI is 22.9, which falls │
│ within the normal healthy range (18.5-24.9).   │
│ This indicates a healthy weight for your body  │
│ type. [Source: health-calculator-server]       │
└─────────────────────────────────────────────────┘
```

## 📊 Configuration Examples

### MCP Server Configuration

```yaml
# mcp_servers.yaml
servers:
  - name: "health-tools"
    transport: "stdio"
    command: "/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-server-rust/target/release/mcp-server-rust"
    args: []
    env:
      RUST_LOG: "info"
    
  - name: "file-system"
    transport: "stdio"
    command: "/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-server-rust/target/release/mcp-server-rust"
    args: ["--config", "filesystem"]
    
  - name: "web-search"
    transport: "http"
    url: "http://localhost:8080/mcp"
    headers:
      Authorization: "Bearer ${API_KEY}"
```

### Router Configuration

```yaml
# routing_config.yaml
routing:
  default_route: "llm_direct"
  confidence_thresholds:
    mcp_tool: 0.8
    rag: 0.7
    hybrid: 0.6
  
  tool_categories:
    calculator: ["calculate-bmi", "unit-converter"]
    file_operations: ["read-file", "write-file", "list-directory"]
    web: ["search-web", "fetch-url"]
  
  intent_mappings:
    "calculate bmi": "calculate-bmi"
    "what is my weight": "calculate-bmi"
    "convert units": "unit-converter"
```

### Connection Management

```rust
// connection_config.rs
use std::time::Duration;

pub struct McpConfig {
    pub connection: ConnectionConfig,
    pub discovery: DiscoveryConfig,
    pub health: HealthConfig,
}

pub struct ConnectionConfig {
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub pool_size: usize,
}

pub struct DiscoveryConfig {
    pub auto_refresh: bool,
    pub refresh_interval: Duration,
    pub cache_ttl: Duration,
}

pub const MCP_CONFIG: McpConfig = McpConfig {
    connection: ConnectionConfig {
        timeout: Duration::from_secs(30),
        max_retries: 3,
        retry_delay: Duration::from_secs(1),
        pool_size: 5,
    },
    discovery: DiscoveryConfig {
        auto_refresh: true,
        refresh_interval: Duration::from_secs(300),
        cache_ttl: Duration::from_secs(3600),
    },
    health: HealthConfig {
        check_interval: Duration::from_secs(60),
        failure_threshold: 3,
    },
};
```

## 🛡️ Error Handling & Fallbacks

### Error Handling Strategy

```rust
use mcp_client_rust::{ClientError, ClientResult};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IntegrationError {
    #[error("MCP client error: {0}")]
    McpError(#[from] ClientError),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("RAG system error: {0}")]
    RagError(String),
    #[error("All strategies failed after {attempts} attempts")]
    AllStrategiesFailed { attempts: usize },
}

pub struct FallbackExecutor {
    strategies: Vec<ExecutionStrategy>,
    metrics: ExecutionMetrics,
}

impl FallbackExecutor {
    pub async fn execute_with_fallback(
        &self, 
        query: &str,
        context: &QueryContext
    ) -> Result<FinalResponse, IntegrationError> {
        let mut attempts = 0;
        
        for strategy in &self.strategies {
            match self.execute_strategy(strategy, query, context).await {
                Ok(response) => {
                    self.metrics.record_success(strategy, attempts);
                    return Ok(response);
                }
                Err(e) => {
                    attempts += 1;
                    self.metrics.record_failure(strategy, &e);
                    log::warn!("Strategy {:?} failed: {}", strategy, e);
                    continue;
                }
            }
        }
        
        Err(IntegrationError::AllStrategiesFailed { attempts })
    }
}
```

### Common Error Scenarios

1. **Tool Not Found**: Route to RAG or direct LLM
2. **Tool Execution Error**: Try alternative tools or fallback
3. **Server Unavailable**: Use cached capabilities or RAG
4. **Network Timeout**: Retry with exponential backoff
5. **Invalid Parameters**: Request clarification from user

## 📈 Performance Optimization

### Caching Strategy

```rust
use moka::sync::Cache;
use std::time::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server: String,
    pub last_updated: std::time::SystemTime,
}

pub struct CapabilityCache {
    tool_cache: Cache<String, Vec<CachedTool>>,
    result_cache: Cache<String, ToolResult>,
    routing_cache: Cache<String, RouteDecision>,
}

impl CapabilityCache {
    pub fn new() -> Self {
        Self {
            tool_cache: Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(3600))
                .build(),
            result_cache: Cache::builder()
                .max_capacity(5000)
                .time_to_live(Duration::from_secs(300))
                .build(),
            routing_cache: Cache::builder()
                .max_capacity(10000)
                .time_to_live(Duration::from_secs(600))
                .build(),
        }
    }
    
    pub async fn get_cached_tools(&self, server_name: &str) -> Option<Vec<CachedTool>> {
        self.tool_cache.get(server_name)
    }
    
    pub async fn cache_tools(&self, server_name: String, tools: Vec<CachedTool>) {
        self.tool_cache.insert(server_name, tools);
    }
}
```

### Connection Pooling

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct ConnectionPool {
    semaphore: Arc<Semaphore>,
    connections: Vec<MCPClient>,
    config: ConnectionConfig,
}

impl ConnectionPool {
    pub fn new(max_connections: usize, config: ConnectionConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_connections)),
            connections: Vec::new(),
            config,
        }
    }
    
    pub async fn get_connection(&self) -> Result<ConnectionGuard, Box<dyn std::error::Error>> {
        let _permit = self.semaphore.acquire().await?;
        
        // Either get existing connection or create new one
        let connection = if let Some(conn) = self.connections.pop() {
            conn
        } else {
            self.create_new_connection().await?
        };
        
        Ok(ConnectionGuard {
            connection,
            pool: self,
            _permit,
        })
    }
    
    async fn create_new_connection(&self) -> Result<MCPClient, Box<dyn std::error::Error>> {
        // Implementation using mcp-client-rust
        let transport = StdioTransport::new(&self.config.server_path, &[])?;
        let client_info = ClientInfo {
            name: "AICLI-Pool".to_string(),
            version: "1.0.0".to_string(),
        };
        
        let mut client = MCPClient::new(Arc::new(transport), client_info);
        client.initialize().await?;
        
        Ok(client)
    }
}
```

## 🔍 Monitoring & Debugging

### Logging Configuration

```rust
use tracing::{info, warn, error, debug};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

// Configure detailed logging
pub fn setup_logging() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(true)
        .with_target(true)
        .with_level(true)
        .with_file(true)
        .with_line_number(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

// Structured logging for MCP operations
#[derive(Debug)]
pub struct McpOperation {
    pub operation: &'static str,
    pub server: String,
    pub tool_name: Option<String>,
    pub duration: std::time::Duration,
    pub success: bool,
}

impl McpOperation {
    pub fn log(&self) {
        if self.success {
            info!(
                operation = self.operation,
                server = self.server,
                tool_name = self.tool_name.as_deref().unwrap_or("none"),
                duration_ms = self.duration.as_millis(),
                "MCP operation completed successfully"
            );
        } else {
            warn!(
                operation = self.operation,
                server = self.server,
                tool_name = self.tool_name.as_deref().unwrap_or("none"),
                duration_ms = self.duration.as_millis(),
                "MCP operation failed"
            );
        }
    }
}
```

### Health Monitoring

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerHealth {
    pub status: HealthStatus,
    pub response_time: Duration,
    pub last_check: Instant,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

pub struct HealthMonitor {
    metrics: HashMap<String, ServerHealth>,
    check_interval: Duration,
}

impl HealthMonitor {
    pub fn new(check_interval: Duration) -> Self {
        Self {
            metrics: HashMap::new(),
            check_interval,
        }
    }
    
    pub async fn check_health(&mut self, client: &MCPClient) -> HashMap<String, ServerHealth> {
        let mut health_status = HashMap::new();
        
        // Check primary server
        match self.ping_server(client).await {
            Ok(response_time) => {
                health_status.insert("primary".to_string(), ServerHealth {
                    status: HealthStatus::Healthy,
                    response_time,
                    last_check: Instant::now(),
                    error: None,
                });
            }
            Err(e) => {
                health_status.insert("primary".to_string(), ServerHealth {
                    status: HealthStatus::Unhealthy,
                    response_time: Duration::from_millis(0),
                    last_check: Instant::now(),
                    error: Some(e.to_string()),
                });
            }
        }
        
        self.metrics = health_status.clone();
        health_status
    }
    
    async fn ping_server(&self, client: &MCPClient) -> Result<Duration, ClientError> {
        let start = Instant::now();
        // Simple ping operation
        let _ = client.list_tools().await?;
        Ok(start.elapsed())
    }
}
```

## 🚀 Deployment Considerations

### Production Setup

1. **Server Management**:
   - Use process managers (systemd, supervisor)
   - Implement health checks
   - Set up monitoring and alerting

2. **Security**:
   - Secure transport connections
   - Implement authentication
   - Validate all inputs
   - Rate limiting

3. **Scalability**:
   - Connection pooling
   - Load balancing
   - Caching strategies
   - Asynchronous processing

### Testing Strategy

```rust
// integration_tests.rs
use mcp_client_rust::{MCPClient, ClientInfo, ClientResult};
use tokio;

#[cfg(test)]
mod mcp_integration_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_discovery() -> Result<(), Box<dyn std::error::Error>> {
        // Test that all configured tools are discoverable
        let mut client = create_test_client().await?;
        let tools = client.list_tools().await?;
        
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "calculate-bmi"));
        assert!(tools.iter().any(|t| t.name == "greet"));
        assert!(tools.iter().any(|t| t.name == "fetch-weather"));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_tool_execution() -> Result<(), Box<dyn std::error::Error>> {
        // Test tool execution with valid parameters
        let mut client = create_test_client().await?;
        let result = client.call_tool(
            "calculate-bmi",
            json!({
                "weightKg": 70,
                "heightM": 1.75
            })
        ).await?;
        
        assert_eq!(result.is_error, Some(false));
        let content_text = result.content.first().unwrap();
        assert!(matches!(content_text, ToolResultContent::Text { text } if text.contains("22.9")));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_routing_decisions() -> Result<(), Box<dyn std::error::Error>> {
        // Test query routing logic
        let mut client = create_test_client().await?;
        let router = QueryRouter::new(client);
        
        let (route, tools) = router.route_query("Calculate my BMI").await?;
        assert!(matches!(route, Route::McpTool));
        assert!(tools.iter().any(|t| t.name == "calculate-bmi"));
        
        let (route, _) = router.route_query("Tell me about Rust programming").await?;
        assert!(matches!(route, Route::Rag));
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
        // Test graceful error handling
        let mut client = create_test_client().await?;
        
        // Test invalid tool name
        let result = client.call_tool("non-existent-tool", json!({})).await;
        assert!(result.is_err());
        
        // Test invalid parameters
        let result = client.call_tool(
            "calculate-bmi",
            json!({
                "weightKg": "invalid", // Should be number
                "heightM": 1.75
            })
        ).await;
        assert!(result.is_err());
        
        Ok(())
    }
    
    async fn create_test_client() -> ClientResult<MCPClient> {
        let transport = StdioTransport::new(
            "/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-server-rust/target/release/mcp-server-rust",
            &[]
        )?;
        
        let client_info = ClientInfo {
            name: "TestClient".to_string(),
            version: "1.0.0".to_string(),
        };
        
        let mut client = MCPClient::new(Arc::new(transport), client_info);
        client.initialize().await?;
        Ok(client)
    }
}
```

## 📚 Additional Resources

### Documentation Links
- [MCP Specification](https://modelcontextprotocol.io)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Ollama Documentation](https://github.com/ollama/ollama)
- [mcp-client-rust Documentation](/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-client-rust/README.md)
- [mcp-server-rust Documentation](/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-server-rust/README.md)

### Example Implementations
- Tool schema examples from mcp-server-rust
- Configuration templates for aicli integration
- Error handling patterns using mcp-client-rust
- Performance benchmarks with your setup

### Key File Paths
- **MCP Client**: `/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-client-rust`
- **MCP Server**: `/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-server-rust`
- **AICLI Project**: `/Users/sudhirkumar/Desktop/sudhir/gitsudhir/aicli`
- **Server Executable**: `/Users/sudhirkumar/Desktop/sudhir/gitsudhir/mcp-server-rust/target/release/mcp-server-rust`

### Integration Architecture
```
aicli/
├── src/
│   ├── mcp_integration.rs      # Main MCP integration logic
│   ├── query_router.rs         # Query routing and decision engine
│   ├── tool_manager.rs         # Tool discovery and management
│   └── fallback_handler.rs     # Error handling and fallbacks
├── config/
│   ├── mcp_servers.yaml        # MCP server configurations
│   └── routing_config.yaml     # Routing rules and thresholds
└── tests/
    └── integration_tests.rs    # Integration test suite
```

---

*Last Updated: February 13, 2026*
*Version: 1.0*