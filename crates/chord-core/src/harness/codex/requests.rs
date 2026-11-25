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

pub(super) fn rate_limits(id: u64) -> Value {
    json!({ "id": id, "method": "account/rateLimits/read", "params": { "excludeResetCreditDetails": true } })
}

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

pub(super) fn thread_revert(id: u64, thread: &str, turn: &str) -> Value {
    json!({ "id": id, "method": "thread/revert", "params": { "threadId": thread, "beforeTurnId": turn } })
}

pub(super) fn thread_turns(id: u64, thread: &str) -> Value {
    json!({ "id": id, "method": "thread/turns/list", "params": { "threadId": thread, "sortDirection": "desc", "limit": TURNS_LISTED } })
}

pub(super) fn thread_compact(id: u64, thread: &str) -> Value {
    json!({ "id": id, "method": "thread/compact/start", "params": { "threadId": thread } })
}

const TURNS_LISTED: u32 = 200;

pub(super) fn thread_list(id: u64, cwd: &str) -> Value {
    json!({ "id": id, "method": "thread/list", "params": { "cwd": cwd, "sortKey": "updated_at", "limit": 50 } })
}

pub(super) fn skills_list(id: u64, cwd: &str) -> Value {
    json!({ "id": id, "method": "skills/list", "params": { "cwds": [cwd] } })
}

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

pub(super) fn input(text: &str, images: &[Image]) -> Vec<Value> {
    let text =
        (!text.is_empty()).then(|| json!({ "type": "text", "text": text, "text_elements": [] }));
    text.into_iter()
        .chain(images.iter().map(|image| {
            json!({ "type": "image", "url": format!("data:{};base64,{}", image.mime_type, image.data) })
        }))
        .collect()
}
