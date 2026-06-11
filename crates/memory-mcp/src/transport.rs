use serde_json::Value;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

use super::schemas::*;
use super::server::MemoryMcpServer;

pub async fn run_stdio_server(server: MemoryMcpServer) -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    info!("MCP server started on stdio");

    while let Some(line) = lines.next_line().await? {
        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse request: {}", e);
                continue;
            }
        };

        let response = match request.method.as_str() {
            "initialize" => handle_initialize(&request),
            "tools/list" => handle_tools_list(&request),
            "tools/call" => handle_tools_call(&request, &server).await,
            _ => make_error_response(
                request.id,
                -32601,
                &format!("Method not found: {}", request.method),
                None,
            ),
        };

        let response_json = serde_json::to_string(&response)?;
        let mut out = io::stdout();
        out.write_all(response_json.as_bytes()).await?;
        out.write_all(b"\n").await?;
        out.flush().await?;
    }

    Ok(())
}

fn handle_initialize(req: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: req.id.clone(),
        result: Some(serde_json::to_value(InitializeResult {
            protocol_version: "2024-11-05".into(),
            capabilities: ServerCapabilities {
                tools: ToolCapability { list_changed: true },
            },
            server_info: ServerInfo {
                name: "agent-memory".into(),
                version: "0.1.0".into(),
            },
        }).unwrap()),
        error: None,
    }
}

fn handle_tools_list(req: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = all_tool_definitions();
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: req.id.clone(),
        result: Some(serde_json::json!({ "tools": tools })),
        error: None,
    }
}

async fn handle_tools_call(req: &JsonRpcRequest, server: &MemoryMcpServer) -> JsonRpcResponse {
    let params = match &req.params {
        Some(Value::Object(p)) => p.clone(),
        _ => {
            return make_error_response(req.id.clone(), -32602, "Invalid params", None);
        }
    };

    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => {
            return make_error_response(req.id.clone(), -32602, "Missing tool name", None);
        }
    };

    let arguments = params.get("arguments").cloned().unwrap_or(Value::Object(serde_json::Map::new()));

    match server.handle_tool_call(&tool_name, arguments).await {
        Ok(result) => {
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: req.id.clone(),
                result: Some(serde_json::json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string(&result).unwrap_or_default(),
                    }]
                })),
                error: None,
            }
        }
        Err(e) => {
            let message = match &e {
                memory_core::MemoryError::SecretContentRejected => "Content rejected: potential secret detected.".into(),
                memory_core::MemoryError::ObservationNotFound(id) => format!("Observation not found: {}", id),
                memory_core::MemoryError::InvalidScope => "Invalid scope.".into(),
                memory_core::MemoryError::MissingEvidence => "Evidence is required for durable memory.".into(),
                _ => format!("Internal error: {}", e),
            };
            make_error_response(req.id.clone(), -32000, &message, None)
        }
    }
}

fn make_error_response(id: Option<Value>, code: i32, message: &str, data: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_response() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: "initialize".into(),
            params: None,
        };
        let resp = handle_initialize(&req);
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["protocol_version"], "2024-11-05");
    }

    #[test]
    fn test_tools_list_has_all_tools() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: "tools/list".into(),
            params: None,
        };
        let resp = handle_tools_list(&req);
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 11);
    }

    #[test]
    fn test_error_response_for_unknown_method() {
        let resp = make_error_response(Some(Value::Number(1.into())), -32601, "Method not found", None);
        assert!(resp.result.is_none());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }
}
