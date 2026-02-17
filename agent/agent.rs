use serde::Deserialize;
use serde_json::{Value, json};

use crate::build_prompt::{Message, format_context_from_hits};
use crate::config::Config;
use crate::embed_query::embed_query;
use crate::generate::{generate_answer, generate_json};
use crate::mcp::{McpCapabilities, McpClient};
use crate::retrieve_chunks::retrieve_top;

#[derive(Clone, Debug)]
pub struct AgentState {
    pub conversation: Vec<Message>,
    pub current_step: usize,
    pub max_steps: usize,
    pub context_log: Vec<String>,
}

impl AgentState {
    pub fn new(max_steps: usize) -> Self {
        Self {
            conversation: Vec::new(),
            current_step: 0,
            max_steps,
            context_log: Vec::new(),
        }
    }

    pub fn append_user(&mut self, text: String) {
        self.conversation.push(Message {
            role: "user".to_string(),
            content: text,
        });
    }

    pub fn append_system(&mut self, text: String) {
        self.conversation.push(Message {
            role: "system".to_string(),
            content: text,
        });
    }

    pub fn append_context(&mut self, text: String) {
        self.context_log.push(text.clone());
        self.conversation.push(Message {
            role: "system".to_string(),
            content: text,
        });
    }

    pub fn append_tool(&mut self, text: String) {
        self.context_log.push(text.clone());
        self.conversation.push(Message {
            role: "tool".to_string(),
            content: text,
        });
    }

    pub fn context_text(&self) -> String {
        if self.context_log.is_empty() {
            "(no context found)".to_string()
        } else {
            self.context_log.join("\n\n")
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    Retrieve { query: String },
    ToolCall { name: String, args: Value },
    PromptCall { name: String, args: Value },
    ResourceRead { uri: String },
    FinalAnswer(String),
}

#[derive(Deserialize)]
struct DecisionEnvelope {
    action: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Value,
    #[serde(default)]
    answer: Option<String>,
    #[serde(default)]
    uri: Option<String>,
}

pub fn answer_query_hybrid(cfg: &Config, question: &str) -> Result<(String, String), String> {
    let mcp = McpClient::from_config(cfg);
    let mcp_enabled = mcp.is_enabled();
    let caps = mcp.discover_capabilities();
    let mut state = AgentState::new(cfg.agent_max_steps.max(1));
    state.append_system(build_hybrid_system_prompt(cfg, &caps, mcp_enabled));
    if is_rag_only_query(question) {
        state.append_system(
            "User requested RAG-only mode for this query. Do not use MCP tool/prompt/resource actions. Use retrieve and final only."
                .to_string(),
        );
    }
    state.append_user(question.to_string());
    let answer = run_agent(&mut state, cfg, &mcp)?;
    Ok((state.context_text(), answer))
}

pub fn run_agent(state: &mut AgentState, cfg: &Config, mcp: &McpClient) -> Result<String, String> {
    while state.current_step < state.max_steps {
        let raw = generate_json(cfg, &state.conversation)?;
        let decision = match parse_decision(&raw) {
            Ok(d) => d,
            Err(err) => {
                state.append_system(format!(
                    "Invalid controller JSON output: {}. Return valid JSON with one action and required fields.",
                    err
                ));
                state.current_step += 1;
                continue;
            }
        };

        match decision {
            Decision::Retrieve { query } => match run_retrieve(cfg, &query) {
                Ok(ctx) => state.append_context(format!("RAG retrieve for query: {}\n{}", query, ctx)),
                Err(err) => state.append_tool(format!("RAG retrieve error: {}", err)),
            },
            Decision::ToolCall { name, args } => {
                if is_rag_only_state(state) {
                    let fallback_query = latest_user_query(state).unwrap_or_else(|| name.clone());
                    match run_retrieve(cfg, &fallback_query) {
                        Ok(ctx) => state.append_context(format!(
                            "RAG retrieve fallback (RAG-only mode) for query: {}\n{}",
                            fallback_query, ctx
                        )),
                        Err(err) => state.append_tool(format!(
                            "RAG retrieve fallback error (RAG-only mode): {}",
                            err
                        )),
                    }
                    state.current_step += 1;
                    continue;
                }
                if !mcp.is_enabled() {
                    state.append_system(
                        "MCP is unavailable in this session. Choose only: retrieve or final."
                            .to_string(),
                    );
                    state.current_step += 1;
                    continue;
                }
                let normalized_args = normalize_tool_args(&name, args, state);
                let result = mcp
                    .call_tool(&name, normalized_args)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|e| format!("Tool call failed for {}: {}", name, e));
                state.append_tool(format!("Tool result [{}]: {}", name, result));
            }
            Decision::PromptCall { name, args } => {
                if is_rag_only_state(state) {
                    let fallback_query = latest_user_query(state).unwrap_or_else(|| name.clone());
                    match run_retrieve(cfg, &fallback_query) {
                        Ok(ctx) => state.append_context(format!(
                            "RAG retrieve fallback (RAG-only mode) for query: {}\n{}",
                            fallback_query, ctx
                        )),
                        Err(err) => state.append_tool(format!(
                            "RAG retrieve fallback error (RAG-only mode): {}",
                            err
                        )),
                    }
                    state.current_step += 1;
                    continue;
                }
                if !mcp.is_enabled() {
                    state.append_system(
                        "MCP is unavailable in this session. Choose only: retrieve or final."
                            .to_string(),
                    );
                    state.current_step += 1;
                    continue;
                }
                let result = mcp
                    .get_prompt(&name, args)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|e| format!("Prompt fetch failed for {}: {}", name, e));
                state.append_tool(format!("Prompt result [{}]: {}", name, result));
            }
            Decision::ResourceRead { uri } => {
                if is_rag_only_state(state) {
                    let fallback_query = latest_user_query(state).unwrap_or_else(|| uri.clone());
                    match run_retrieve(cfg, &fallback_query) {
                        Ok(ctx) => state.append_context(format!(
                            "RAG retrieve fallback (RAG-only mode) for query: {}\n{}",
                            fallback_query, ctx
                        )),
                        Err(err) => state.append_tool(format!(
                            "RAG retrieve fallback error (RAG-only mode): {}",
                            err
                        )),
                    }
                    state.current_step += 1;
                    continue;
                }
                if !mcp.is_enabled() {
                    let fallback_query = latest_user_query(state).unwrap_or_else(|| uri.clone());
                    match run_retrieve(cfg, &fallback_query) {
                        Ok(ctx) => state.append_context(format!(
                            "RAG retrieve fallback (MCP disabled) for query: {}\n{}",
                            fallback_query, ctx
                        )),
                        Err(err) => state.append_tool(format!(
                            "RAG retrieve fallback error (MCP disabled): {}",
                            err
                        )),
                    }
                    state.current_step += 1;
                    continue;
                }
                match mcp.read_resource(&uri) {
                    Ok(value) => {
                        state.append_tool(format!("Resource result [{}]: {}", uri, value));
                    }
                    Err(err) => {
                        state.append_tool(format!("Resource read failed for {}: {}", uri, err));
                        let fallback_query = latest_user_query(state).unwrap_or_else(|| uri.clone());
                        match run_retrieve(cfg, &fallback_query) {
                            Ok(ctx) => state.append_context(format!(
                                "RAG retrieve fallback (resource read failed) for query: {}\n{}",
                                fallback_query, ctx
                            )),
                            Err(retrieve_err) => state.append_tool(format!(
                                "RAG retrieve fallback error (resource read failed): {}",
                                retrieve_err
                            )),
                        }
                    }
                }
            }
            Decision::FinalAnswer(answer) => return Ok(answer),
        }

        state.current_step += 1;
    }

    force_final_answer(state, cfg).or_else(|fallback_err| {
        Err(format!(
            "Max steps exceeded (limit: {}) before final answer; fallback generation failed: {}",
            state.max_steps, fallback_err
        ))
    })
}

fn run_retrieve(cfg: &Config, query: &str) -> Result<String, String> {
    let query_vec = embed_query(cfg, query)?;
    let hits = retrieve_top(cfg, &query_vec)?;
    Ok(format_context_from_hits(&hits))
}

fn build_hybrid_system_prompt(cfg: &Config, caps: &McpCapabilities, mcp_enabled: bool) -> String {
    let mut prompt = format!(
        "{}\n\nAvailable Tools:\n{}\n\nAvailable Prompts:\n{}\n\nAvailable Resources:\n{}",
        cfg.hybrid_system_prompt,
        list_or_none(&caps.tools),
        list_or_none(&caps.prompts),
        list_or_none(&caps.resources),
    );

    if !caps.diagnostics.is_empty() {
        prompt.push_str("\n\nMCP Diagnostics:\n");
        prompt.push_str(&caps.diagnostics.join("\n"));
    }

    if !mcp_enabled {
        prompt.push_str(
            "\n\nMCP is currently unavailable. Do not choose tool/prompt/resource. Use retrieve and final only.",
        );
    } else if caps.tools.is_empty() && caps.prompts.is_empty() && caps.resources.is_empty() {
        prompt.push_str(
            "\n\nMCP is configured but capability discovery returned no tools/prompts/resources. Prefer retrieve/final unless user explicitly asks for MCP, and inspect MCP diagnostics.",
        );
    }
    prompt
}

fn list_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "- (none)".to_string()
    } else {
        items
            .iter()
            .map(|it| format!("- {}", it))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub fn parse_decision(raw: &str) -> Result<Decision, String> {
    let data = parse_json_object(raw)?;
    let env: DecisionEnvelope = serde_json::from_value(data).map_err(|e| e.to_string())?;
    let action = env.action.trim().to_lowercase();

    match action.as_str() {
        "retrieve" => {
            let query = env
                .arguments
                .get("query")
                .and_then(|q| q.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "retrieve action requires arguments.query".to_string())?;
            Ok(Decision::Retrieve { query })
        }
        "tool" => {
            let name = env
                .name
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| "tool action requires name".to_string())?;
            let args = if env.arguments.is_null() {
                json!({})
            } else {
                env.arguments
            };
            Ok(Decision::ToolCall { name, args })
        }
        "prompt" => {
            let name = env
                .name
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| "prompt action requires name".to_string())?;
            let args = if env.arguments.is_null() {
                json!({})
            } else {
                env.arguments
            };
            Ok(Decision::PromptCall { name, args })
        }
        "resource" => {
            let uri = env
                .uri
                .or_else(|| {
                    env.arguments
                        .get("uri")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                })
                .or(env.name)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| "resource action requires uri".to_string())?;
            Ok(Decision::ResourceRead { uri })
        }
        "final" => {
            let answer = env
                .answer
                .filter(|s| !s.trim().is_empty())
                .or_else(|| {
                    env.arguments
                        .as_str()
                        .map(|s| s.to_string())
                        .filter(|s| !s.trim().is_empty())
                })
                .or_else(|| {
                    env.arguments
                        .get("answer")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .filter(|s| !s.trim().is_empty())
                })
                .or_else(|| {
                    env.arguments
                        .get("final")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .filter(|s| !s.trim().is_empty())
                })
                .or_else(|| {
                    env.arguments
                        .get("text")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .filter(|s| !s.trim().is_empty())
                })
                .or_else(|| {
                    env.arguments
                        .get("response")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .filter(|s| !s.trim().is_empty())
                })
                .or_else(|| env.name.filter(|s| !s.trim().is_empty()))
                .ok_or_else(|| "final action requires answer".to_string())?;
            Ok(Decision::FinalAnswer(answer))
        }
        other => Err(format!("unknown action: {}", other)),
    }
}

fn parse_json_object(raw: &str) -> Result<Value, String> {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        return Ok(v);
    }
    let start = raw
        .find('{')
        .ok_or_else(|| "No JSON object found in model output".to_string())?;
    let end = raw
        .rfind('}')
        .ok_or_else(|| "No JSON object found in model output".to_string())?;
    let slice = &raw[start..=end];
    serde_json::from_str::<Value>(slice)
        .map_err(|e| format!("Failed to parse JSON decision: {}", e))
}

fn latest_user_query(state: &AgentState) -> Option<String> {
    state
        .conversation
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
}

fn normalize_tool_args(name: &str, args: Value, state: &AgentState) -> Value {
    if !name.eq_ignore_ascii_case("fetch-weather") {
        return args;
    }

    if let Some(city) = extract_city_from_args(&args).or_else(|| {
        latest_user_query(state).and_then(|q| infer_city_from_text(&q))
    }) {
        return json!({ "city": city });
    }

    args
}

fn extract_city_from_args(args: &Value) -> Option<String> {
    if let Some(s) = args.as_str() {
        let city = s.trim();
        if !city.is_empty() {
            return Some(city.to_string());
        }
    }

    let obj = args.as_object()?;
    let direct_keys = ["city", "location", "place", "town", "query"];
    for key in direct_keys {
        if let Some(v) = obj.get(key).and_then(|v| v.as_str()) {
            let city = v.trim();
            if !city.is_empty() {
                return Some(city.to_string());
            }
        }
    }

    for (k, v) in obj {
        if k.to_ascii_lowercase().contains("city") {
            if let Some(s) = v.as_str() {
                let city = s.trim();
                if !city.is_empty() {
                    return Some(city.to_string());
                }
            }
        }
    }

    None
}

fn infer_city_from_text(text: &str) -> Option<String> {
    let patterns = ["city=", "city:", " in ", " for "];
    for pat in patterns {
        if let Some(raw) = take_after_case_insensitive(text, pat) {
            let city = clean_city_candidate(&raw);
            if !city.is_empty() {
                return Some(city);
            }
        }
    }
    None
}

fn take_after_case_insensitive(text: &str, needle: &str) -> Option<String> {
    let hay = text.to_ascii_lowercase();
    let n = needle.to_ascii_lowercase();
    let idx = hay.find(&n)?;
    let start = idx + needle.len();
    if start > text.len() {
        return None;
    }
    Some(text[start..].to_string())
}

fn clean_city_candidate(raw: &str) -> String {
    let mut out = raw.trim().trim_matches('"').trim_matches('\'').to_string();
    for delim in [',', ';', '\n'] {
        if let Some(i) = out.find(delim) {
            out.truncate(i);
        }
    }
    out.trim().trim_matches('"').trim_matches('\'').to_string()
}

fn is_rag_only_query(question: &str) -> bool {
    let q = question.to_ascii_lowercase();
    q.contains("use rag")
        || q.contains("rag only")
        || q.contains("only rag")
        || q.contains("do not use mcp")
        || q.contains("don't use mcp")
        || q.contains("without mcp")
}

fn is_rag_only_state(state: &AgentState) -> bool {
    latest_user_query(state)
        .map(|q| is_rag_only_query(&q))
        .unwrap_or(false)
}

fn force_final_answer(state: &AgentState, cfg: &Config) -> Result<String, String> {
    let question = latest_user_query(state).unwrap_or_default();
    let context = state.context_text();
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: cfg.system_prompt.clone(),
        },
        Message {
            role: "user".to_string(),
            content: format!(
                "Use the context below to answer the question.\n\nContext:\n{}\n\nQuestion: {}\n\nReturn only a direct final answer in plain text. Do not return JSON.",
                context, question
            ),
        },
    ];
    let answer = generate_answer(cfg, &messages)?;
    if answer.trim().is_empty() {
        return Err("Model returned an empty fallback final answer".to_string());
    }
    Ok(answer)
}
