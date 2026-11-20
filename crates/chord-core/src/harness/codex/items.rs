//! What a Codex thread item means as harness events, at its start and at its end. Live items and a resumed thread's turns share it.

use std::path::Path;

use serde_json::Value;

use crate::harness::shell::{read_only_title, shell_script};
use crate::harness::{HarnessEvent, Image, LineChanges, ToolOutcome, relative, text};
use crate::images;

/// The events of an item that started; `at` is when, in epoch milliseconds.
pub(super) fn started(item: &Value, at: Option<f64>, cwd: &Path) -> Vec<HarnessEvent> {
    let id = text(&item["id"]);
    let tool = |title: &str, detail: String, changes| {
        vec![HarnessEvent::ToolStart {
            id: id.clone(),
            title: title.into(),
            detail,
            changes,
        }]
    };
    match item["type"].as_str() {
        Some("userMessage") => vec![HarnessEvent::UserMessageChunk {
            text: user_text(item),
            images: user_images(item),
            at,
        }],
        Some("commandExecution") => {
            let command = text(&item["command"]);
            let script = shell_script(&command).unwrap_or(command);
            tool(read_only_title(&script).unwrap_or("Run"), script, None)
        }
        Some("fileChange") => {
            let changes = item["changes"].as_array().map(Vec::as_slice).unwrap_or(&[]);
            let all = |kind: &str| {
                !changes.is_empty() && changes.iter().all(|change| change["kind"]["type"] == kind)
            };
            let title = if all("add") {
                "Write"
            } else if all("delete") {
                "Delete"
            } else {
                "Edit"
            };
            let paths = changes
                .iter()
                .map(|change| relative(&text(&change["path"]), cwd))
                .collect::<Vec<_>>()
                .join(", ");
            tool(title, paths, Some(line_changes(changes)))
        }
        Some("mcpToolCall") => {
            let arguments = match &item["arguments"] {
                Value::Null => String::new(),
                Value::Object(map) if map.is_empty() => String::new(),
                arguments => format!(" {arguments}"),
            };
            tool(
                &text(&item["tool"]),
                format!("{}{arguments}", text(&item["server"])),
                None,
            )
        }
        Some("webSearch") => tool("Search", text(&item["query"]), None),
        Some("imageView") => tool("Read", relative(&text(&item["path"]), cwd), None),
        _ => vec![],
    }
}

/// The events of an item that completed; `streamed` when its text already came as deltas.
pub(super) fn completed(item: &Value, at: Option<f64>, streamed: bool) -> Vec<HarnessEvent> {
    let end = |images: Vec<Image>| {
        vec![HarnessEvent::ToolEnd {
            id: text(&item["id"]),
            outcome: match item["status"].as_str() {
                Some("failed" | "declined") => ToolOutcome::Failed,
                _ => ToolOutcome::Done,
            },
            images,
            ms: item["durationMs"]
                .as_u64()
                .and_then(|ms| u32::try_from(ms).ok()),
            changes: None,
        }]
    };
    match item["type"].as_str() {
        Some("agentMessage") => {
            let text = text(&item["text"]);
            let chunk =
                (!streamed && !text.is_empty()).then_some(HarnessEvent::MessageChunk { text });
            chunk
                .into_iter()
                .chain([HarnessEvent::MessageEnd { failed: false, at }])
                .collect()
        }
        Some("commandExecution" | "fileChange" | "webSearch") => end(vec![]),
        Some("mcpToolCall") => end(content_images(&item["result"]["content"])),
        Some("imageView") => end(file_image(Path::new(&text(&item["path"])))
            .into_iter()
            .collect()),
        Some("contextCompaction") => vec![HarnessEvent::Compaction {
            summary: String::new(),
            tokens_before: None,
        }],
        _ => vec![],
    }
}

/// A saved turn's items as the events a live turn sends; each item starts and completes at once.
pub(super) fn turn_history(turn: &Value, cwd: &Path) -> Vec<HarnessEvent> {
    turn["items"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|item| {
            let mut events = started(item, None, cwd);
            events.extend(completed(item, None, false));
            events
        })
        .collect()
}

/// The text of a user message's text inputs.
pub(super) fn user_text(item: &Value) -> String {
    item["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|input| input["type"] == "text")
        .filter_map(|input| input["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn user_images(item: &Value) -> Vec<Image> {
    item["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|input| input["type"] == "image")
        .filter_map(|input| {
            let (mime_type, data) = input["url"]
                .as_str()?
                .strip_prefix("data:")?
                .split_once(";base64,")?;
            Some(Image {
                data: data.into(),
                mime_type: mime_type.into(),
            })
        })
        .collect()
}

fn content_images(content: &Value) -> Vec<Image> {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "image")
        .filter_map(|block| {
            Some(Image {
                data: block["data"].as_str()?.into(),
                mime_type: block["mimeType"].as_str()?.into(),
            })
        })
        .collect()
}

fn file_image(path: &Path) -> Option<Image> {
    Some(Image {
        mime_type: images::mime_type(path)?.into(),
        data: images::read_base64(path).ok()?,
    })
}

// An update's diff marks its lines with + and -; an added file's diff is its content, a deleted one's what it held.
fn line_changes(changes: &[Value]) -> LineChanges {
    let mut counted = LineChanges {
        added: 0,
        removed: 0,
    };
    for change in changes {
        let diff = change["diff"].as_str().unwrap_or_default();
        let lines = |keep: fn(&str) -> bool| diff.lines().filter(|line| keep(line)).count() as u32;
        match change["kind"]["type"].as_str() {
            Some("add") => counted.added += lines(|_| true),
            Some("delete") => counted.removed += lines(|_| true),
            _ => {
                counted.added += lines(|line| line.starts_with('+') && !line.starts_with("+++"));
                counted.removed += lines(|line| line.starts_with('-') && !line.starts_with("---"));
            }
        }
    }
    counted
}
