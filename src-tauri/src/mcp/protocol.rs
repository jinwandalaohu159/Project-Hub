//! JSON-RPC over MCP: protocol negotiation, method dispatch, batches, and the stdio transport.

use super::tools::{call_tool, tools};
use super::McpHost;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const SERVER_NAME: &str = "project-hub";
const SUPPORTED_PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
const INSTRUCTIONS: &str = "Project Hub keeps local versions of the user's projects. \
Use get_status or get_diff to see unsaved work, then save_version with a short title and an optional \
description to record it. Pass the project's folder path (your working directory works) or its name.";

pub(super) fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Handles one JSON-RPC message. Notifications (no id) produce no response.
pub(super) fn handle_message(host: &dyn McpHost, message: &Value) -> Option<Value> {
    let id = message.get("id").cloned();
    let method = message.get("method").and_then(Value::as_str).unwrap_or_default();
    let params = message.get("params").cloned().unwrap_or(Value::Null);

    let id = id.filter(|id| !id.is_null())?;

    let result = match method {
        "initialize" => {
            let requested = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let version = if SUPPORTED_PROTOCOLS.contains(&requested) {
                requested
            } else {
                SUPPORTED_PROTOCOLS[0]
            };
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            match call_tool(host, name, &args) {
                Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
                Err(error) if error.starts_with("UNKNOWN_TOOL:") => {
                    return Some(rpc_error(id, -32602, &format!("Unknown tool: {name}")));
                }
                Err(error) => json!({ "content": [{ "type": "text", "text": error }], "isError": true }),
            }
        }
        "resources/list" => json!({ "resources": [] }),
        "prompts/list" => json!({ "prompts": [] }),
        _ => return Some(rpc_error(id, -32601, &format!("Method not found: {method}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

pub(super) fn handle_payload(host: &dyn McpHost, payload: &Value) -> Option<Value> {
    match payload {
        Value::Array(batch) => {
            let responses: Vec<Value> = batch.iter().filter_map(|m| handle_message(host, m)).collect();
            (!responses.is_empty()).then_some(Value::Array(responses))
        }
        single => handle_message(host, single),
    }
}

pub fn run_stdio(host: &dyn McpHost) {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(payload) => handle_payload(host, &payload),
            Err(_) => Some(rpc_error(Value::Null, -32700, "Parse error")),
        };
        if let Some(response) = response {
            let _ = writeln!(stdout, "{}", response);
            let _ = stdout.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::test_support::host_with_repo;

    #[test]
    fn initialize_negotiates_protocol() {
        let (host, _dir) = host_with_repo();
        let response = handle_message(
            &host,
            &json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": { "protocolVersion": "2025-03-26" } }),
        )
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2025-03-26");
        assert!(handle_message(&host, &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
        let tools = handle_message(&host, &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 5);
    }
}
