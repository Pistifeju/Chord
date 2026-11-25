//! The JSON-RPC lines the adapter writes to OpenCode: the handshake, the session and its copy, the prompt, the steer, the config picks, the cancel, and the refusal of a request Chord does not answer.

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

/// A new session in the worktree `cwd`, an absolute path.
pub(super) fn session_new(id: u64, cwd: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/new", "params": { "cwd": cwd, "mcpServers": [] } })
}

pub(super) fn session_delete(id: u64, session: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/delete", "params": { "sessionId": session } })
}

/// A copy of `session` as a new session.
pub(super) fn session_fork(id: u64, session: &str, cwd: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/fork", "params": { "sessionId": session, "cwd": cwd, "mcpServers": [] } })
}

/// Sets the config option `config` of `session` to `value`.
pub(super) fn set_config_option(id: u64, session: &str, config: &str, value: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/set_config_option", "params": { "sessionId": session, "configId": config, "value": value } })
}

pub(super) fn prompt(id: u64, session: &str, text: &str, images: &[Image]) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "session/prompt", "params": { "sessionId": session, "prompt": blocks(text, images) } })
}

/// A message OpenCode adds to the running turn.
pub(super) fn steer(id: u64, session: &str, text: &str, images: &[Image]) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": "_session/steering", "params": { "sessionId": session, "prompt": blocks(text, images) } })
}

pub(super) fn cancel(session: &str) -> Value {
    json!({ "jsonrpc": "2.0", "method": "session/cancel", "params": { "sessionId": session } })
}

/// The answer to a request of OpenCode's that Chord does not take.
pub(super) fn refusal(id: &Value, method: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("Chord does not handle {method}") } })
}

/// The user's text, then each image.
fn blocks(text: &str, images: &[Image]) -> Vec<Value> {
    let text = (!text.is_empty()).then(|| json!({ "type": "text", "text": text }));
    text.into_iter()
        .chain(images.iter().map(
            |image| json!({ "type": "image", "mimeType": image.mime_type, "data": image.data }),
        ))
        .collect()
}
