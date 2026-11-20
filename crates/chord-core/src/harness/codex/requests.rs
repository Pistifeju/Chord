//! The JSON-RPC lines the adapter writes to Codex: the handshake, the thread, and the turn.

use serde_json::{Map, Value, json};

use crate::harness::Image;

pub(super) fn initialize(id: u64) -> Value {
    json!({
        "id": id,
        "method": "initialize",
        "params": {
            "clientInfo": { "name": "chord", "title": "Chord", "version": env!("CARGO_PKG_VERSION") },
            "capabilities": null,
        },
    })
}

pub(super) fn initialized() -> Value {
    json!({ "method": "initialized" })
}

pub(super) fn model_list(id: u64) -> Value {
    json!({ "id": id, "method": "model/list", "params": {} })
}

pub(super) fn thread_start(id: u64, cwd: &str) -> Value {
    json!({ "id": id, "method": "thread/start", "params": { "cwd": cwd } })
}

pub(super) fn thread_resume(id: u64, thread: &str, cwd: &str) -> Value {
    json!({ "id": id, "method": "thread/resume", "params": { "threadId": thread, "cwd": cwd } })
}

/// The worktree's threads, the last updated first.
pub(super) fn thread_list(id: u64, cwd: &str) -> Value {
    json!({ "id": id, "method": "thread/list", "params": { "cwd": cwd, "sortKey": "updated_at", "limit": 50 } })
}

pub(super) fn skills_list(id: u64, cwd: &str) -> Value {
    json!({ "id": id, "method": "skills/list", "params": { "cwds": [cwd] } })
}

/// A turn with the user's picks: `model`, `effort`, `approvalPolicy`, `sandboxPolicy`.
pub(super) fn turn_start(
    id: u64,
    thread: &str,
    input: &[Value],
    picks: &Map<String, Value>,
) -> Value {
    let mut params = picks.clone();
    params.insert("threadId".into(), json!(thread));
    params.insert("input".into(), json!(input));
    json!({ "id": id, "method": "turn/start", "params": params })
}

pub(super) fn turn_steer(id: u64, thread: &str, turn: &str, input: &[Value]) -> Value {
    json!({
        "id": id,
        "method": "turn/steer",
        "params": { "threadId": thread, "input": input, "expectedTurnId": turn },
    })
}

pub(super) fn turn_interrupt(id: u64, thread: &str, turn: &str) -> Value {
    json!({ "id": id, "method": "turn/interrupt", "params": { "threadId": thread, "turnId": turn } })
}

/// The user's text, then each image as a data URL.
pub(super) fn input(text: &str, images: &[Image]) -> Vec<Value> {
    let text =
        (!text.is_empty()).then(|| json!({ "type": "text", "text": text, "text_elements": [] }));
    text.into_iter()
        .chain(images.iter().map(|image| {
            json!({ "type": "image", "url": format!("data:{};base64,{}", image.mime_type, image.data) })
        }))
        .collect()
}
