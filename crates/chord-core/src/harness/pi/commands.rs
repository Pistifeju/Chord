use serde_json::{Value, json};

use super::{EXTENSION_PREFIX, RELOAD_PROMPT, SESSION_PROMPT};
use crate::harness::{DialogAnswer, Image};

pub(super) fn prompt_command(text: &str, images: &[Image]) -> Value {
    let images: Vec<Value> = images
        .iter()
        .map(|image| json!({ "type": "image", "data": image.data, "mimeType": image.mime_type }))
        .collect();
    json!({ "type": "prompt", "message": text, "images": images, "streamingBehavior": "steer" })
}

pub(super) fn session_command(text: &str, held_elsewhere: &[String]) -> Option<Value> {
    let extension = |name: &str| json!({ "id": SESSION_PROMPT, "type": "prompt", "message": format!("/{EXTENSION_PREFIX}{name}") });
    let name = command_name(text);
    Some(match (name, text[name.len()..].trim()) {
        ("/export", "") => json!({ "type": "export_html" }),
        ("/export", path) => json!({ "type": "export_html", "outputPath": path }),
        ("/compact", "") => json!({ "type": "compact" }),
        ("/compact", focus) => json!({ "type": "compact", "customInstructions": focus }),
        ("/new", "") => json!({ "type": "new_session" }),
        ("/resume", "") => extension(&format!("resume {}", json!(held_elsewhere))),
        ("/model", "") => json!({ "type": "get_available_models" }),
        ("/thinking", "") => json!({ "type": "get_available_thinking_levels" }),
        ("/fork", "") => json!({ "type": "get_fork_messages" }),
        ("/clone", "") => json!({ "type": "clone" }),
        ("/tree", "") => extension("tree"),
        ("/reload", "") => {
            let mut reload = extension("reload");
            reload["id"] = RELOAD_PROMPT.into();
            reload
        }
        _ => return None,
    })
}

pub(super) fn command_name(text: &str) -> &str {
    text.split(char::is_whitespace).next().unwrap_or_default()
}

pub(super) fn session_requests() -> [Value; 3] {
    [
        json!({ "type": "get_state" }),
        json!({ "type": "get_messages" }),
        json!({ "type": "get_session_stats" }),
    ]
}

pub(super) fn fork_command(entries: &str, answer: &DialogAnswer) -> Option<Value> {
    let number: usize = picked_value(answer)?.split_once('.')?.0.parse().ok()?;
    let entry = entries.split(',').nth(number.checked_sub(1)?)?;
    Some(json!({ "type": "fork", "entryId": entry }))
}

pub(super) fn model_command(answer: &DialogAnswer) -> Option<Value> {
    let (provider, model) = picked_value(answer)?.split_once('/')?;
    Some(json!({ "type": "set_model", "provider": provider, "modelId": model }))
}

pub(super) fn thinking_command(answer: &DialogAnswer) -> Option<Value> {
    Some(json!({ "type": "set_thinking_level", "level": picked_value(answer)?.to_lowercase() }))
}

fn picked_value(answer: &DialogAnswer) -> Option<&str> {
    match answer {
        DialogAnswer::Value { value } => Some(value),
        _ => None,
    }
}

pub(super) fn dialog_response(id: &str, answer: &DialogAnswer) -> Value {
    let mut response = json!({ "type": "extension_ui_response", "id": id });
    match answer {
        DialogAnswer::Value { value } => response["value"] = json!(value),
        DialogAnswer::Values { values } => response["value"] = json!(values.join(", ")),
        DialogAnswer::Confirmed { confirmed } => response["confirmed"] = json!(confirmed),
        DialogAnswer::Cancelled => response["cancelled"] = json!(true),
    }
    response
}
