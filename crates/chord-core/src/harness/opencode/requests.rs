use serde_json::{Value, json};

use crate::harness::Image;

pub(super) fn initialize(id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": 1,
            "clientInfo": { "name": "chord", "version": env!("CARGO_PKG_VERSION") },
            "clientCapabilities": {
                "elicitation": { "form": {} },
                "session": { "compaction": {} },
                "_meta": { "opencode/child-session-updates": true },
            },
        },
    })
}

pub(super) fn mcp_servers(server: Option<(String, Vec<String>)>) -> Value {
    let servers = server.map(
        |(command, args)| json!({ "name": "chord", "command": command, "args": args, "env": [] }),
    );
    json!(servers.into_iter().collect::<Vec<_>>())
}

pub(super) fn session_new(id: u64, cwd: &str, mcp: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/new", "params": { "cwd": cwd, "mcpServers": mcp } })
}

pub(super) fn session_load(id: u64, session: &str, cwd: &str, mcp: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/load", "params": { "sessionId": session, "cwd": cwd, "mcpServers": mcp } })
}

pub(super) fn session_list(id: u64, cwd: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/list", "params": { "cwd": cwd } })
}

pub(super) fn session_close(id: u64, session: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/close", "params": { "sessionId": session } })
}

pub(super) fn session_delete(id: u64, session: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/delete", "params": { "sessionId": session } })
}

pub(super) fn session_fork(id: u64, session: &str, cwd: &str, mcp: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/fork", "params": { "sessionId": session, "cwd": cwd, "mcpServers": mcp } })
}

pub(super) fn set_config_option(id: u64, session: &str, config: &str, value: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/set_config_option", "params": { "sessionId": session, "configId": config, "value": value } })
}

pub(super) fn prompt(
    id: u64,
    session: &str,
    text: &str,
    images: &[Image],
    instructions: Option<&str>,
) -> Value {
    let mut prompt = blocks(text, images);
    if let Some(instructions) = instructions {
        prompt.push(json!({ "type": "text", "text": instructions, "annotations": { "audience": ["assistant"] } }));
    }
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/prompt", "params": { "sessionId": session, "prompt": prompt } })
}

pub(super) fn steer(id: u64, session: &str, text: &str, images: &[Image]) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "_session/steering", "params": { "sessionId": session, "prompt": blocks(text, images) } })
}

pub(super) fn cancel(session: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session } })
}

pub(super) fn refusal(id: &Value, method: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("Chord does not handle {method}") } })
}

fn blocks(text: &str, images: &[Image]) -> Vec<Value> {
    let text = (!text.is_empty()).then(|| json!({ "type": "text", "text": text }));
    text.into_iter()
        .chain(images.iter().map(
            |image| json!({ "type": "image", "mimeType": image.mime_type, "data": image.data }),
        ))
        .collect()
}
