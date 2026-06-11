use serde::{Deserialize, Serialize};
use serde_json::Value;

// --- MCP JSON-RPC types ---

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// --- MCP Initialize ---

#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: ToolCapability,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCapability {
    pub list_changed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

// --- Tool Definitions ---

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

pub fn all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "memory.recall".into(),
            description: "Task-aware memory recall using hybrid search with token budget.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "task": {"type": "string", "description": "Task description for context-aware recall"},
                    "scope": {"type": "string", "enum": ["session", "project", "user", "organization"]},
                    "project_id": {"type": "string", "format": "uuid"},
                    "files": {"type": "array", "items": {"type": "string"}},
                    "token_budget": {"type": "integer", "default": 1200}
                },
                "required": ["task", "scope"]
            }),
        },
        ToolDefinition {
            name: "memory.search".into(),
            description: "Hybrid memory search combining vector, keyword, and structured filters.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string"},
                    "scope": {"type": "string", "enum": ["session", "project", "user", "organization"]},
                    "project_id": {"type": "string", "format": "uuid"},
                    "kinds": {"type": "array", "items": {"type": "string"}},
                    "files": {"type": "array", "items": {"type": "string"}},
                    "limit": {"type": "integer", "default": 10}
                },
                "required": ["query", "scope"]
            }),
        },
        ToolDefinition {
            name: "memory.get".into(),
            description: "Fetch a full memory observation with provenance and evidence.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "format": "uuid"}
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "memory.write".into(),
            description: "Write a source-backed observation to durable memory.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "scope": {"type": "string"},
                    "project_id": {"type": "string", "format": "uuid"},
                    "session_id": {"type": "string"},
                    "kind": {"type": "string"},
                    "summary": {"type": "string"},
                    "confidence": {"type": "string", "enum": ["low", "medium", "high"]},
                    "sensitivity": {"type": "string", "enum": ["public", "internal", "private", "secret"]},
                    "evidence": {"type": "array", "items": {"type": "object"}},
                    "entities": {"type": "array", "items": {"type": "string"}},
                    "files": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["scope", "session_id", "kind", "summary"]
            }),
        },
        ToolDefinition {
            name: "memory.update".into(),
            description: "Update an existing memory observation.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "format": "uuid"},
                    "summary": {"type": "string"},
                    "confidence": {"type": "string", "enum": ["low", "medium", "high"]},
                    "status": {"type": "string", "enum": ["active", "unconfirmed", "superseded", "obsolete", "conflicted", "deleted"]}
                },
                "required": ["id"]
            }),
        },
        ToolDefinition {
            name: "memory.mark_obsolete".into(),
            description: "Mark a memory as obsolete with a reason.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "format": "uuid"},
                    "reason": {"type": "string"}
                },
                "required": ["id", "reason"]
            }),
        },
        ToolDefinition {
            name: "memory.consolidate_session".into(),
            description: "Convert session events into durable observations.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "project_id": {"type": "string", "format": "uuid"}
                },
                "required": ["session_id"]
            }),
        },
        ToolDefinition {
            name: "memory.link_file".into(),
            description: "Link an observation to a file path.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "observation_id": {"type": "string", "format": "uuid"},
                    "file_path": {"type": "string"}
                },
                "required": ["observation_id", "file_path"]
            }),
        },
        ToolDefinition {
            name: "memory.list_conflicts".into(),
            description: "List unresolved memory conflicts for a project.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {"type": "string", "format": "uuid"}
                },
                "required": ["project_id"]
            }),
        },
        ToolDefinition {
            name: "memory.resolve_conflict".into(),
            description: "Resolve a memory conflict with a resolution strategy.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "conflict_id": {"type": "string", "format": "uuid"},
                    "resolution": {"type": "string", "enum": ["left_wins", "right_wins", "merge"]},
                    "reason": {"type": "string"}
                },
                "required": ["conflict_id", "resolution"]
            }),
        },
        ToolDefinition {
            name: "memory.delete".into(),
            description: "Soft-delete a memory with a reason.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "format": "uuid"},
                    "reason": {"type": "string"}
                },
                "required": ["id", "reason"]
            }),
        },
    ]
}
