use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::harness::shell::read_only_title;
use crate::harness::{HarnessEvent, Image, LineChanges, ToolOutcome, relative, text, timestamp};

#[derive(Clone, Copy, Default)]
pub(super) struct Streamed {
    pub(super) text: bool,
    pub(super) thought: bool,
}

pub(super) fn assistant(
    record: &Value,
    streamed: Streamed,
    tools: &mut HashMap<String, String>,
    cwd: &Path,
) -> Vec<HarnessEvent> {
    let failed = !record["error"].is_null();
    let at = timestamp(&record["timestamp"]);
    let mut events = vec![];
    for block in record["message"]["content"]
        .as_array()
        .into_iter()
        .flatten()
    {
        match block["type"].as_str() {
            Some("text") => {
                let text = text(&block["text"]);
                if !streamed.text && !text.is_empty() {
                    events.push(HarnessEvent::MessageChunk { text });
                }
                events.push(HarnessEvent::MessageEnd { failed, at });
            }
            Some("thinking") => {
                let text = text(&block["thinking"]);
                if !streamed.thought && !text.is_empty() {
                    events.push(HarnessEvent::ThoughtStart);
                    events.push(HarnessEvent::ThoughtChunk { text });
                }
            }
            Some("tool_use") => {
                let started = tool_use(block, cwd);
                if let Some(HarnessEvent::ToolStart { id, .. }) = started.first() {
                    tools.insert(id.clone(), text(&block["name"]));
                }
                events.extend(started);
            }
            _ => {}
        }
    }
    events
}

pub(super) fn tool_results(
    content: &Value,
    result: &Value,
    tools: &mut HashMap<String, String>,
) -> Vec<HarnessEvent> {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "tool_result")
        .filter_map(|block| {
            let name = tools.remove(block["tool_use_id"].as_str()?)?;
            Some(tool_result(block, &name, result))
        })
        .collect()
}

fn tool_use(block: &Value, cwd: &Path) -> Vec<HarnessEvent> {
    let name = text(&block["name"]);
    let input = &block["input"];
    if name == "ExitPlanMode" {
        return vec![
            HarnessEvent::MessageChunk {
                text: text(&input["plan"]),
            },
            HarnessEvent::MessageEnd {
                failed: false,
                at: None,
            },
        ];
    }
    let path = |key: &str| relative(&text(&input[key]), cwd);
    let mut changes = None;
    let (title, detail) = match name.as_str() {
        "Bash" => {
            let command = text(&input["command"]);
            (
                read_only_title(&command).unwrap_or("Run").to_string(),
                command,
            )
        }
        "Read" => ("Read".into(), path("file_path")),
        "Write" => {
            changes = Some(LineChanges {
                added: text(&input["content"]).lines().count() as u32,
                removed: 0,
            });
            ("Write".into(), path("file_path"))
        }
        "Edit" => ("Edit".into(), path("file_path")),
        "NotebookEdit" => ("Edit".into(), path("notebook_path")),
        "WebSearch" => ("Search".into(), text(&input["query"])),
        "AskUserQuestion" => {
            let questions: Vec<String> = input["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|question| text(&question["question"]))
                .collect();
            ("Ask".into(), questions.join("\n"))
        }
        _ => match name
            .strip_prefix("mcp__")
            .and_then(|rest| rest.split_once("__"))
        {
            Some((server, tool)) => (tool.into(), format!("{server}{}", arguments(input, " "))),
            None => (name.clone(), arguments(input, "")),
        },
    };
    vec![HarnessEvent::ToolStart {
        id: text(&block["id"]),
        title,
        detail,
        command: name == "Bash",
        changes,
    }]
}

fn tool_result(block: &Value, name: &str, result: &Value) -> HarnessEvent {
    let changes = (name == "Edit").then(|| {
        let lines: Vec<&str> = result["structuredPatch"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|hunk| hunk["lines"].as_array().into_iter().flatten())
            .filter_map(Value::as_str)
            .collect();
        let count = |mark: char| lines.iter().filter(|line| line.starts_with(mark)).count() as u32;
        LineChanges {
            added: count('+'),
            removed: count('-'),
        }
    });
    HarnessEvent::ToolEnd {
        id: text(&block["tool_use_id"]),
        outcome: match block["is_error"] == true {
            true => ToolOutcome::Failed,
            false => ToolOutcome::Done,
        },
        images: images(&block["content"]),
        ms: None,
        changes,
    }
}

pub(super) fn user_text(content: &Value) -> String {
    let text = match content {
        Value::String(text) => text.clone(),
        blocks => blocks
            .as_array()
            .into_iter()
            .flatten()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    };
    let Some(name) = tagged(&text, "command-name") else {
        return text;
    };
    let name = format!("/{}", name.trim_start_matches('/'));
    match tagged(&text, "command-args").map(str::trim) {
        Some(args) if !args.is_empty() => format!("{name} {args}"),
        _ => name,
    }
}

pub(super) fn images(content: &Value) -> Vec<Image> {
    content
        .as_array()
        .into_iter()
        .flatten()
        .filter(|block| block["type"] == "image" && block["source"]["type"] == "base64")
        .filter_map(|block| {
            Some(Image {
                data: block["source"]["data"].as_str()?.into(),
                mime_type: block["source"]["media_type"].as_str()?.into(),
            })
        })
        .collect()
}

fn tagged<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let start = text.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = start + text[start..].find(&format!("</{tag}>"))?;
    Some(&text[start..end])
}

pub(super) fn arguments(input: &Value, lead: &str) -> String {
    match input {
        Value::Null => String::new(),
        Value::Object(map) if map.is_empty() => String::new(),
        input => format!("{lead}{input}"),
    }
}
