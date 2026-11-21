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

/// The account's rate limit windows, without the reset credits' details.
pub(super) fn rate_limits(id: u64) -> Value {
    json!({ "id": id, "method": "account/rateLimits/read", "params": { "excludeResetCreditDetails": true } })
}

/// The params every thread the pane opens carries: the worktree, and the guidelines of Chord's tools when the agent has them. Codex hides MCP tools and their server's instructions from the model, so the guidelines go in as developer instructions.
pub(super) fn opening(cwd: &str, instructions: Option<String>) -> Map<String, Value> {
    let mut params = Map::new();
    params.insert("cwd".into(), json!(cwd));
    if let Some(instructions) = instructions {
        params.insert("developerInstructions".into(), json!(instructions));
    }
    params
}

pub(super) fn thread_start(id: u64, opening: &Map<String, Value>) -> Value {
    json!({ "id": id, "method": "thread/start", "params": opening })
}

pub(super) fn thread_resume(id: u64, thread: &str, opening: &Map<String, Value>) -> Value {
    let mut params = opening.clone();
    params.insert("threadId".into(), json!(thread));
    json!({ "id": id, "method": "thread/resume", "params": params })
}

/// A new thread with the turns of `thread` through `last_turn`, or with all of them without one.
pub(super) fn thread_fork(
    id: u64,
    thread: &str,
    last_turn: Option<&str>,
    opening: &Map<String, Value>,
) -> Value {
    let mut params = opening.clone();
    params.insert("threadId".into(), json!(thread));
    params.insert("lastTurnId".into(), json!(last_turn));
    json!({ "id": id, "method": "thread/fork", "params": params })
}

/// Drops `turn` and every later turn from the thread.
pub(super) fn thread_revert(id: u64, thread: &str, turn: &str) -> Value {
    json!({ "id": id, "method": "thread/revert", "params": { "threadId": thread, "beforeTurnId": turn } })
}

/// The thread's newest turns, newest first, each with its user message.
pub(super) fn thread_turns(id: u64, thread: &str) -> Value {
    json!({ "id": id, "method": "thread/turns/list", "params": { "threadId": thread, "sortDirection": "desc", "limit": TURNS_LISTED } })
}

pub(super) fn thread_compact(id: u64, thread: &str) -> Value {
    json!({ "id": id, "method": "thread/compact/start", "params": { "threadId": thread } })
}

// shortcut: fork and rewind offer the newest 200 turns alone; page with the cursor once a session outgrows them.
const TURNS_LISTED: u32 = 200;

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
