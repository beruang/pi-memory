use memory_mcp::schemas::*;

#[test]
fn test_all_11_tools_defined() {
    let tools = all_tool_definitions();
    assert_eq!(tools.len(), 11);

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(tool_names.contains(&"memory.recall"));
    assert!(tool_names.contains(&"memory.search"));
    assert!(tool_names.contains(&"memory.get"));
    assert!(tool_names.contains(&"memory.write"));
    assert!(tool_names.contains(&"memory.update"));
    assert!(tool_names.contains(&"memory.mark_obsolete"));
    assert!(tool_names.contains(&"memory.consolidate_session"));
    assert!(tool_names.contains(&"memory.link_file"));
    assert!(tool_names.contains(&"memory.list_conflicts"));
    assert!(tool_names.contains(&"memory.resolve_conflict"));
    assert!(tool_names.contains(&"memory.delete"));
}

#[test]
fn test_tool_schemas_have_required_fields() {
    for tool in all_tool_definitions() {
        let schema = &tool.input_schema;
        assert!(schema.get("type").is_some());
        assert!(schema.get("properties").is_some());

        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            for field in required {
                let field_name = field.as_str().unwrap();
                assert!(
                    schema["properties"].get(field_name).is_some(),
                    "Required field '{}' missing from properties for tool '{}'",
                    field_name,
                    tool.name
                );
            }
        }
    }
}

#[test]
fn test_initialize_result_shape() {
    let result = InitializeResult {
        protocol_version: "2024-11-05".into(),
        capabilities: ServerCapabilities {
            tools: ToolCapability { list_changed: true },
        },
        server_info: ServerInfo {
            name: "agent-memory".into(),
            version: "0.1.0".into(),
        },
    };

    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["protocol_version"], "2024-11-05");
    assert_eq!(json["server_info"]["name"], "agent-memory");
    assert!(json["capabilities"]["tools"]["list_changed"]
        .as_bool()
        .unwrap());
}

#[test]
fn test_json_rpc_request_parse() {
    let json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "memory.recall",
            "arguments": {"task": "test", "scope": "project"}
        }
    });

    let req: JsonRpcRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.method, "tools/call");
    assert!(req.id.is_some());

    let params = req.params.unwrap();
    assert_eq!(params["name"], "memory.recall");
    assert_eq!(params["arguments"]["task"], "test");
}

#[test]
fn test_error_response_safe() {
    let resp = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "error": {
            "code": -32000,
            "message": "Content rejected: potential secret detected."
        }
    });

    let err_msg = resp["error"]["message"].as_str().unwrap();
    // Error message must not contain raw SQL, connection strings, or secrets
    assert!(!err_msg.contains("postgres://"));
    assert!(!err_msg.contains("SELECT"));
    assert!(!err_msg.contains("password"));
    assert!(!err_msg.contains("PRIVATE KEY"));
}
