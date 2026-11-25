//! What a `session/update` means as harness events: the open reply, thought or replayed user message, the tool calls, the context, the commands and the compaction. Live updates and a loaded session's replay share it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::commands::Settings;
use crate::harness::shell::read_only_title;
use crate::harness::{HarnessEvent, Image, LineChanges, ToolOutcome, count, image, relative, text};

/// The message whose chunks stream now; a chunk of the same message joins it.
enum Open {
    Reply(String),
    Thought(String),
    User {
        id: String,
        text: Vec<String>,
        images: Vec<Image>,
    },
}

/// A tool call OpenCode announced, with the latest of its fields.
struct Call {
    name: String,
    kind: String,
    title: String,
    input: Value,
    locations: Value,
    /// Its `ToolStart` went out.
    started: bool,
}

/// Reads one session's updates.
pub(super) struct Updates {
    cwd: PathBuf,
    open: Option<Open>,
    calls: HashMap<String, Call>,
    /// The model's context window, from the last `usage_update`.
    window: Option<u32>,
    /// The summary chunks of each running compaction.
    summaries: HashMap<String, String>,
}

impl Updates {
    pub(super) fn new(cwd: &Path) -> Self {
        Updates {
            cwd: cwd.to_path_buf(),
            open: None,
            calls: HashMap::new(),
            window: None,
            summaries: HashMap::new(),
        }
    }

    /// Ends the message that streams: a reply's end, or a replayed user message whole. A thought ends without an event.
    pub(super) fn close(&mut self) -> Option<HarnessEvent> {
        match self.open.take()? {
            Open::Reply(_) => Some(HarnessEvent::MessageEnd {
                failed: false,
                at: None,
            }),
            Open::Thought(_) => None,
            Open::User { text, images, .. } => Some(HarnessEvent::UserMessageChunk {
                text: text.join("\n"),
                images,
                at: None,
            }),
        }
    }

    /// The turn ended: its open calls end with it, and a late update of one gives nothing.
    pub(super) fn settle(&mut self) {
        self.calls.clear();
    }

    pub(super) fn update(&mut self, update: &Value, settings: &mut Settings) -> Vec<HarnessEvent> {
        let kind = update["sessionUpdate"].as_str().unwrap_or_default();
        let id = text(&update["messageId"]);
        let content = &update["content"];
        let chunk = chunk_text(content);
        match (kind, &mut self.open) {
            ("agent_message_chunk", Some(Open::Reply(open))) if *open == id => {
                return chunk
                    .map(|text| HarnessEvent::MessageChunk { text })
                    .into_iter()
                    .collect();
            }
            ("agent_thought_chunk", Some(Open::Thought(open))) if *open == id => {
                return chunk
                    .map(|text| HarnessEvent::ThoughtChunk { text })
                    .into_iter()
                    .collect();
            }
            (
                "user_message_chunk",
                Some(Open::User {
                    id: open,
                    text,
                    images,
                }),
            ) if *open == id => {
                user_block(content, text, images);
                return vec![];
            }
            _ => {}
        }
        let mut events: Vec<HarnessEvent> = self.close().into_iter().collect();
        match kind {
            "agent_message_chunk" => {
                self.open = Some(Open::Reply(id));
                events.extend(chunk.map(|text| HarnessEvent::MessageChunk { text }));
            }
            "agent_thought_chunk" => {
                self.open = Some(Open::Thought(id));
                events.push(HarnessEvent::ThoughtStart);
                events.extend(chunk.map(|text| HarnessEvent::ThoughtChunk { text }));
            }
            "user_message_chunk" => {
                let (mut text, mut images) = (vec![], vec![]);
                user_block(content, &mut text, &mut images);
                self.open = Some(Open::User { id, text, images });
            }
            "tool_call" | "tool_call_update" => events.extend(self.call(update)),
            "usage_update" => {
                self.window = count(&update["size"]);
                events.push(HarnessEvent::Context {
                    tokens: count(&update["used"]),
                    window: self.window,
                    estimated: false,
                });
            }
            "config_option_update" => events.extend(settings.read(&update["configOptions"])),
            "available_commands_update" => {
                events.push(settings.read_commands(&update["availableCommands"]))
            }
            "compaction_update" => events.extend(self.compaction(update)),
            "compaction_summary_chunk" => {
                let held = self
                    .summaries
                    .entry(text(&update["compactionId"]))
                    .or_default();
                held.push_str(&chunk.unwrap_or_default());
            }
            _ => {}
        }
        events
    }

    // A call starts at its first `in_progress`, which carries its input, or at its end when none came.
    fn call(&mut self, update: &Value) -> Vec<HarnessEvent> {
        let id = text(&update["toolCallId"]);
        if update["sessionUpdate"] == "tool_call" {
            self.calls.insert(
                id.clone(),
                Call {
                    name: String::new(),
                    kind: String::new(),
                    title: String::new(),
                    input: Value::Null,
                    locations: Value::Null,
                    started: false,
                },
            );
        }
        let Some(call) = self.calls.get_mut(&id) else {
            return vec![];
        };
        // An update carries what changed.
        for (field, key) in [
            (&mut call.name, "name"),
            (&mut call.kind, "kind"),
            (&mut call.title, "title"),
        ] {
            if let Some(value) = update[key].as_str() {
                *field = value.into();
            }
        }
        if update["rawInput"]
            .as_object()
            .is_some_and(|input| !input.is_empty())
        {
            call.input = update["rawInput"].clone();
        }
        if update["locations"]
            .as_array()
            .is_some_and(|paths| !paths.is_empty())
        {
            call.locations = update["locations"].clone();
        }
        let status = update["status"].as_str();
        let ends = matches!(status, Some("completed" | "failed"));
        let mut events = vec![];
        if !call.started && (ends || status == Some("in_progress")) {
            call.started = true;
            events.push(start(&id, call, &self.cwd));
        }
        if ends {
            self.calls.remove(&id);
            events.push(end(id, update));
        }
        events
    }

    fn compaction(&mut self, update: &Value) -> Vec<HarnessEvent> {
        let status = update["status"].as_str();
        if status == Some("in_progress") {
            return vec![HarnessEvent::Compacting { active: true }];
        }
        let held = self.summaries.remove(&text(&update["compactionId"]));
        match status {
            Some("completed") => {
                let held = held.unwrap_or_default();
                let summary = blocks_text(&update["summary"]);
                let summary = if summary.is_empty() { held } else { summary };
                // OpenCode reports no usage until the next reply; the context then holds the summary, counted as pi does: a token per four characters.
                let tokens = (summary.chars().count() / 4) as u32;
                vec![
                    HarnessEvent::Compacting { active: false },
                    HarnessEvent::Compaction {
                        summary,
                        tokens_before: None,
                    },
                    HarnessEvent::Context {
                        tokens: Some(tokens),
                        window: self.window,
                        estimated: true,
                    },
                ]
            }
            Some(status @ ("failed" | "cancelled")) => {
                let error = &update["error"];
                let notice = (status == "failed").then(|| HarnessEvent::Notice {
                    text: error
                        .as_str()
                        .or(error["message"].as_str())
                        .unwrap_or("The compaction failed")
                        .into(),
                });
                [HarnessEvent::Compacting { active: false }]
                    .into_iter()
                    .chain(notice)
                    .collect()
            }
            _ => vec![],
        }
    }
}

fn start(id: &str, call: &Call, cwd: &Path) -> HarnessEvent {
    let path = || {
        call.locations[0]["path"]
            .as_str()
            .map(|path| relative(path, cwd))
            .unwrap_or_default()
    };
    let (title, detail, changes) = match (call.name.as_str(), call.kind.as_str()) {
        ("shell", _) => {
            let command = text(&call.input["command"]);
            (
                read_only_title(&command).unwrap_or("Run").into(),
                command,
                None,
            )
        }
        ("read", _) => ("Read".into(), path(), None),
        ("edit", _) => ("Edit".into(), path(), None),
        ("write", _) => {
            let added = text(&call.input["content"]).lines().count() as u32;
            let changes = LineChanges { added, removed: 0 };
            ("Write".into(), path(), Some(changes))
        }
        (_, "search") => {
            let pattern = call.input["pattern"].as_str().unwrap_or(&call.title);
            ("Search".into(), pattern.into(), None)
        }
        (name, _) => {
            let detail = if call.title == name { "" } else { &call.title };
            (name.into(), detail.into(), None)
        }
    };
    HarnessEvent::ToolStart {
        id: id.into(),
        title,
        detail,
        command: call.name == "shell",
        changes,
    }
}

fn end(id: String, update: &Value) -> HarnessEvent {
    let metadata = &update["rawOutput"]["metadata"];
    let outcome = match update["status"].as_str() {
        Some("completed") => ToolOutcome::Done,
        _ if metadata["interrupted"] == true => ToolOutcome::Stopped,
        _ => ToolOutcome::Failed,
    };
    let changes = metadata["files"].as_array().map(|files| {
        let sum = |key: &str| files.iter().filter_map(|file| count(&file[key])).sum();
        LineChanges {
            added: sum("additions"),
            removed: sum("deletions"),
        }
    });
    let images = update["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| image(&block["content"]))
        .collect();
    HarnessEvent::ToolEnd {
        id,
        outcome,
        images,
        ms: None,
        changes,
    }
}

fn chunk_text(block: &Value) -> Option<String> {
    (block["type"] == "text").then(|| text(&block["text"]))
}

fn blocks_text(blocks: &Value) -> String {
    blocks
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(chunk_text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn user_block(block: &Value, text: &mut Vec<String>, images: &mut Vec<Image>) {
    text.extend(chunk_text(block));
    images.extend(image(block));
}
