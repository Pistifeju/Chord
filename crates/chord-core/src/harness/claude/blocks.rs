//! What Claude Code's content blocks mean as harness events: a user message's text and images, a tool call's start and end, and a record's time. Live records and a transcript's lines share it.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use crate::harness::shell::read_only_title;
use crate::harness::{HarnessEvent, Image, LineChanges, ToolOutcome, relative, text};

/// What of an `assistant` record's block already came as deltas.
#[derive(Clone, Copy, Default)]
pub(super) struct Streamed {
    pub(super) text: bool,
    pub(super) thought: bool,
}

/// The events of an `assistant` record's blocks: the text, the thought and the tool calls, whose names `tools` keeps by id until they end.
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

/// The ends of the tool calls a user record's `tool_result` blocks answer; `result` is the record's tool result. A call `tools` does not hold, as `ExitPlanMode`, has no card to end.
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

// A tool card, or for `ExitPlanMode` the plan as a reply.
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
        changes,
    }]
}

// The end of the tool call a `tool_result` block answers; `name` is the tool's.
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

/// A user message's text: a string, or its `text` blocks joined. A slash command's markup reads as the user typed it.
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

/// The base64 `image` blocks of a message or a tool's result.
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

/// A record's `timestamp`, ISO 8601 in UTC, as epoch milliseconds.
pub(super) fn timestamp(value: &Value) -> Option<f64> {
    let stamp = value.as_str()?.strip_suffix('Z')?;
    let (date, time) = stamp.split_once('T')?;
    let mut date = date.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (date.next()??, date.next()??, date.next()??);
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let mut clock = clock.split(':').map(|part| part.parse::<i64>().ok());
    let (hour, minute, second) = (clock.next()??, clock.next()??, clock.next()??);
    let millis = format!("{fraction:0<3}").get(..3)?.parse::<i64>().ok()?;
    // Days since 1970-01-01 in the proleptic Gregorian calendar, by Howard Hinnant's days_from_civil.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_cycle - 719_468;
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second;
    Some((seconds * 1_000 + millis) as f64)
}

fn tagged<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let start = text.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = start + text[start..].find(&format!("</{tag}>"))?;
    Some(&text[start..end])
}

/// The input as JSON after `lead`, nothing for an empty object.
pub(super) fn arguments(input: &Value, lead: &str) -> String {
    match input {
        Value::Null => String::new(),
        Value::Object(map) if map.is_empty() => String::new(),
        input => format!("{lead}{input}"),
    }
}
