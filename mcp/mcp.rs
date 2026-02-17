use mcp_client_rust::client::MCPClient;
use mcp_client_rust::transport::{HttpSSETransport, StdioTransport, Transport};
use mcp_client_rust::types::{ClientInfo, ContentItem, MessageContent, ToolResultContent};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Builder;

use crate::config::Config;

#[derive(Clone, Debug)]
pub struct McpCapabilities {
    pub tools: Vec<String>,
    pub prompts: Vec<String>,
    pub resources: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct McpClient {
    transport: McpTransport,
}

#[derive(Clone, Debug)]
enum McpTransport {
    Http { endpoint: String },
    Stdio { command: String, args: Vec<String> },
    Disabled,
}

impl McpClient {
    pub fn from_config(cfg: &Config) -> Self {
        if !cfg.mcp_url.trim().is_empty() {
            return Self {
                transport: McpTransport::Http {
                    endpoint: cfg.mcp_url.clone(),
                },
            };
        }
        if !cfg.mcp_command.trim().is_empty() {
            return Self {
                transport: McpTransport::Stdio {
                    command: cfg.mcp_command.clone(),
                    args: cfg.mcp_args.clone(),
                },
            };
        }
        Self {
            transport: McpTransport::Disabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self.transport, McpTransport::Disabled)
    }

    pub fn list_tools(&self) -> Result<Vec<String>, String> {
        self.run_with_client(|rt, client| {
            let tools = rt
                .block_on(client.list_tools())
                .map_err(|e| format!("tools/list failed: {}", e))?;
            Ok(tools.into_iter().map(|t| t.name).collect())
        })
    }

    pub fn list_prompts(&self) -> Result<Vec<String>, String> {
        self.run_with_client(|rt, client| {
            let prompts = rt
                .block_on(client.list_prompts())
                .map_err(|e| format!("prompts/list failed: {}", e))?;
            Ok(prompts.into_iter().map(|p| p.name).collect())
        })
    }

    pub fn list_resources(&self) -> Result<Vec<String>, String> {
        self.run_with_client(|rt, client| {
            let (resources, templates) = rt
                .block_on(client.list_resources())
                .map_err(|e| format!("resources/list failed: {}", e))?;

            let mut items: Vec<String> = resources.into_iter().map(|r| r.uri).collect();
            items.extend(templates.into_iter().map(|t| t.uri_template));
            Ok(items)
        })
    }

    pub fn discover_capabilities(&self) -> McpCapabilities {
        let (tools, tool_diag) = match self.list_tools() {
            Ok(v) => (v, None),
            Err(e) => (Vec::new(), Some(format!("tools/list error: {}", e))),
        };
        let (prompts, prompt_diag) = match self.list_prompts() {
            Ok(v) => (v, None),
            Err(e) => (Vec::new(), Some(format!("prompts/list error: {}", e))),
        };
        let (resources, resource_diag) = match self.list_resources() {
            Ok(v) => (v, None),
            Err(e) => (Vec::new(), Some(format!("resources/list error: {}", e))),
        };

        let mut diagnostics = Vec::new();
        if let Some(d) = tool_diag {
            diagnostics.push(d);
        }
        if let Some(d) = prompt_diag {
            diagnostics.push(d);
        }
        if let Some(d) = resource_diag {
            diagnostics.push(d);
        }

        McpCapabilities {
            tools,
            prompts,
            resources,
            diagnostics,
        }
    }

    pub fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        if !self.is_enabled() {
            return Err("MCP is not configured. Set MCP_URL or MCP_COMMAND.".to_string());
        }

        self.run_with_client(move |rt, client| {
            let result = rt
                .block_on(client.call_tool(name, args))
                .map_err(|e| format!("tools/call failed for {}: {}", name, e))?;
            Ok(tool_result_to_value(result))
        })
    }

    pub fn get_prompt(&self, name: &str, args: Value) -> Result<Value, String> {
        if !self.is_enabled() {
            return Err("MCP is not configured. Set MCP_URL or MCP_COMMAND.".to_string());
        }

        let prompt_args = value_to_prompt_args(args);
        self.run_with_client(move |rt, client| {
            let result = rt
                .block_on(client.get_prompt(name, prompt_args))
                .map_err(|e| format!("prompts/get failed for {}: {}", name, e))?;
            Ok(prompt_result_to_value(result))
        })
    }

    pub fn read_resource(&self, uri: &str) -> Result<Value, String> {
        if !self.is_enabled() {
            return Err("MCP is not configured. Set MCP_URL or MCP_COMMAND.".to_string());
        }

        self.run_with_client(move |rt, client| {
            let result = rt
                .block_on(client.read_resource(uri))
                .map_err(|e| format!("resources/read failed for {}: {}", uri, e))?;
            Ok(resource_content_to_value(result))
        })
    }

    fn run_with_client<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&tokio::runtime::Runtime, &mut MCPClient) -> Result<T, String>,
    {
        let transport = self.build_transport()?;
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to build async runtime for MCP client: {}", e))?;

        let client_info = ClientInfo {
            name: "aicli".to_string(),
            version: "0.1.0".to_string(),
        };
        let mut client = MCPClient::new(transport, client_info);
        rt.block_on(client.initialize())
            .map_err(|e| format!("MCP initialize failed: {}", e))?;

        let out = f(&rt, &mut client);
        let _ = rt.block_on(client.close());
        out
    }

    fn build_transport(&self) -> Result<Arc<dyn Transport>, String> {
        match &self.transport {
            McpTransport::Http { endpoint } => {
                Ok(Arc::new(HttpSSETransport::new(endpoint)) as Arc<dyn Transport>)
            }
            McpTransport::Stdio { command, args } => {
                let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                let t = StdioTransport::new(command, &arg_refs)
                    .map_err(|e| format!("Failed to create MCP stdio transport '{}': {}", command, e))?;
                Ok(Arc::new(t) as Arc<dyn Transport>)
            }
            McpTransport::Disabled => Err("MCP transport is disabled".to_string()),
        }
    }
}

fn value_to_prompt_args(args: Value) -> Option<HashMap<String, String>> {
    let obj = args.as_object()?;
    let mut map = HashMap::new();
    for (k, v) in obj {
        let value = match v {
            Value::String(s) => s.clone(),
            _ => v.to_string(),
        };
        map.insert(k.clone(), value);
    }
    Some(map)
}

fn tool_result_to_value(result: mcp_client_rust::types::ToolResult) -> Value {
    let content = result
        .content
        .into_iter()
        .map(|item| match item {
            ToolResultContent::Text { text } => json!({ "type": "text", "text": text }),
            ToolResultContent::Blob { blob } => json!({ "type": "blob", "blob": blob }),
        })
        .collect::<Vec<_>>();

    json!({
        "content": content,
        "isError": result.is_error.unwrap_or(false)
    })
}

fn resource_content_to_value(result: mcp_client_rust::types::ResourceContent) -> Value {
    let contents = result
        .contents
        .into_iter()
        .map(|item| match item {
            ContentItem::Text { text } => json!({ "type": "text", "text": text }),
            ContentItem::Blob { blob } => json!({ "type": "blob", "blob": blob }),
        })
        .collect::<Vec<_>>();

    json!({ "contents": contents })
}

fn prompt_result_to_value(result: mcp_client_rust::types::PromptsResult) -> Value {
    let messages = result
        .messages
        .into_iter()
        .map(|msg| {
            let content = msg
                .content
                .into_iter()
                .map(|item| match item {
                    MessageContent::Text { text } => json!({ "type": "text", "text": text }),
                    MessageContent::Blob { blob } => json!({ "type": "blob", "blob": blob }),
                })
                .collect::<Vec<_>>();

            json!({
                "role": msg.role,
                "content": content,
            })
        })
        .collect::<Vec<_>>();

    json!({ "messages": messages })
}
