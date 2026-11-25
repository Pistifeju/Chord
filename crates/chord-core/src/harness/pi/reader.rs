//! The pi reader: one stdout line in, the harness events it means and the commands to send pi next out. Holds what the shown session and the open tool calls need across lines.

use std::hash::{DefaultHasher, Hash, Hasher};

use serde::Deserialize;
use serde_json::{Value, json};

use super::commands::session_requests;
use super::records::{events_from, message_update, newest_first, subagent_nodes};
use super::{PICKED_LEVEL, Picker, RELOAD_PROMPT, SESSION_PROMPT, SUBAGENTS_PREFIX};
use crate::harness::calls::OpenCalls;
use crate::harness::{DialogAsk, HarnessEvent, capitalized};

#[derive(Deserialize)]
struct Kind {
    #[serde(rename = "type")]
    kind: String,
    // A reply chunk needs only this field, so its record is not parsed a second time.
    #[serde(rename = "assistantMessageEvent")]
    update: Option<Update>,
}

// Each update carries the whole reply so far as `partial`; serde skips it unbuilt.
#[derive(Deserialize)]
struct Update {
    #[serde(rename = "type")]
    kind: Option<String>,
    delta: Option<String>,
}

const READ_KINDS: [&str; 11] = [
    "extension_ui_request",
    "compaction_start",
    "compaction_end",
    "extension_error",
    "agent_start",
    "agent_settled",
    "message_end",
    "tool_execution_start",
    "tool_execution_end",
    "queue_update",
    "response",
];

/// What one stdout record leads to: events for the chat and commands to send pi next.
#[derive(Default)]
pub(super) struct Read {
    pub(super) events: Vec<HarnessEvent>,
    pub(super) requests: Vec<Value>,
}

/// Reads one pi process's stdout, line by line.
#[derive(Default)]
pub(super) struct Reader {
    shown: ShownSession,
    calls: OpenCalls,
    dialog_open: bool,
}

impl Reader {
    /// A run that settles closes the tool calls pi left open. pi never says a dialog closed, so the first sign of the agent going on closes it.
    pub(super) fn feed(&mut self, line: &[u8]) -> Read {
        let Read { events, requests } = read_line(line, &mut self.shown);
        let mut out = Vec::new();
        for event in self.calls.track(events) {
            match event {
                HarnessEvent::Dialog { .. } => self.dialog_open = true,
                HarnessEvent::MessageChunk { .. }
                | HarnessEvent::ThoughtStart
                | HarnessEvent::ThoughtChunk { .. }
                | HarnessEvent::ToolStart { .. }
                | HarnessEvent::ToolEnd { .. }
                | HarnessEvent::State { running: true }
                    if self.dialog_open =>
                {
                    self.dialog_open = false;
                    out.push(HarnessEvent::DialogClosed);
                }
                _ => {}
            }
            out.push(event);
        }
        Read {
            events: out,
            requests,
        }
    }

    /// Stdout closed: the calls still open end as stopped.
    pub(super) fn end(&mut self) -> Vec<HarnessEvent> {
        self.calls.stop()
    }
}

/// The session the chat shows: its file, a hash of its messages, its model and thinking level, the widget that carries the subagent lineage and whether an automatic compaction runs.
#[derive(Default)]
struct ShownSession {
    file: Option<String>,
    messages: Option<u64>,
    /// The current model as `provider/id` and the current thinking level, so a picker marks them.
    model: Option<String>,
    thinking: Option<String>,
    /// The widget key that last carried a subagent snapshot, so clearing it empties the lineage.
    subagents_widget: Option<String>,
    /// An automatic compaction is running; the subagent extension clears its widget for it and redraws after.
    compacting_automatically: bool,
    /// The tokens the session's messages fill by Chord's count; pi reports none right after a compaction.
    estimate: Option<u32>,
}

impl ShownSession {
    fn track_compaction(&mut self, record: &Value) {
        match (record["type"].as_str(), record["reason"].as_str()) {
            (Some("compaction_start"), Some(reason)) => {
                self.compacting_automatically = reason != "manual"
            }
            (Some("compaction_end"), _) => self.compacting_automatically = false,
            _ => {}
        }
    }

    // A widget line with the snapshot prefix sets the lineage; the same widget cleared, or sent without one, empties it, except while an automatic compaction runs.
    fn subagents(&mut self, record: &Value) -> Option<HarnessEvent> {
        if record["type"] != "extension_ui_request" || record["method"] != "setWidget" {
            return None;
        }
        let key = record["widgetKey"].as_str()?;
        let snapshot = record["widgetLines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .find_map(|line| line.strip_prefix(SUBAGENTS_PREFIX));
        match snapshot {
            Some(snapshot) => {
                self.subagents_widget = Some(key.into());
                Some(HarnessEvent::Subagents {
                    nodes: subagent_nodes(snapshot),
                })
            }
            // The clear is only the extension hiding its widget while the context compacts; it redraws the true list after.
            None if self.compacting_automatically => None,
            None if self.subagents_widget.as_deref() == Some(key) => {
                Some(HarnessEvent::Subagents { nodes: vec![] })
            }
            None => None,
        }
    }

    // A cancelled or failed `/resume` or `/tree` still reads the session again; the same file and messages leave the transcript and status alone.
    fn repeats(&mut self, record: &Value) -> bool {
        if record["type"] != "response" || record["success"] != true {
            return false;
        }
        match record["command"].as_str() {
            Some("get_state") => {
                let data = &record["data"];
                self.file = data["sessionFile"].as_str().map(String::from);
                false
            }
            Some("get_messages") => {
                let mut hasher = DefaultHasher::new();
                (&self.file, record["data"]["messages"].to_string()).hash(&mut hasher);
                let messages = Some(hasher.finish());
                std::mem::replace(&mut self.messages, messages) == messages
            }
            _ => false,
        }
    }

    fn remember_estimate(&mut self, record: &Value) {
        if record["type"] == "response"
            && record["success"] == true
            && record["command"] == "get_messages"
        {
            self.estimate = estimated_tokens(&record["data"]["messages"]);
        }
    }

    // The state names the model and level a picker marks.
    fn remember_choice(&mut self, record: &Value) {
        if record["type"] != "response"
            || record["success"] != true
            || record["command"] != "get_state"
        {
            return;
        }
        let data = &record["data"];
        self.model = match (
            data["model"]["provider"].as_str(),
            data["model"]["id"].as_str(),
        ) {
            (Some(provider), Some(id)) => Some(format!("{provider}/{id}")),
            _ => None,
        };
        self.thinking = data["thinkingLevel"].as_str().map(capitalized);
    }

    // A picker's list is built from a response that does not know the current choice.
    fn with_current(&self, mut event: HarnessEvent) -> HarnessEvent {
        if let HarnessEvent::Context {
            tokens: tokens @ None,
            window: Some(_),
            estimated,
        } = &mut event
        {
            *tokens = self.estimate;
            *estimated = tokens.is_some();
        }
        if let HarnessEvent::Models { models, .. } = &mut event {
            newest_first(models, self.model.as_deref(), |model| &model.id);
        }
        if let HarnessEvent::Dialog {
            id,
            ask: DialogAsk::Menu {
                options, current, ..
            },
            ..
        } = &mut event
        {
            match Picker::from_id(id) {
                Some(Picker::Model) => {
                    newest_first(options, self.model.as_deref(), String::as_str);
                    *current = self.model.clone();
                }
                Some(Picker::Thinking) => *current = self.thinking.clone(),
                None => {}
            }
        }
        event
    }
}

// pi's own estimate: a token per four characters of text, tool arguments and summaries, 1200 per image. It leaves out the system prompt and the tool definitions.
fn estimated_tokens(messages: &Value) -> Option<u32> {
    fn chars(value: &Value) -> usize {
        match value {
            Value::String(text) => text.chars().count(),
            Value::Array(blocks) => blocks.iter().map(chars).sum(),
            Value::Object(block) if block.get("type").and_then(Value::as_str) == Some("image") => {
                4800
            }
            Value::Object(block) => {
                [
                    "text", "thinking", "summary", "content", "command", "output",
                ]
                .iter()
                .filter_map(|key| block.get(*key))
                .map(chars)
                .sum::<usize>()
                    + block
                        .get("arguments")
                        .map_or(0, |args| args.to_string().len())
            }
            _ => 0,
        }
    }
    u32::try_from(chars(messages) / 4).ok()
}

// Tool output and turn summaries are large and unused, so only the records `events_from` reads get a full parse.
fn read_line(line: &[u8], shown: &mut ShownSession) -> Read {
    match serde_json::from_slice::<Kind>(line) {
        Ok(Kind { kind, update }) if kind == "message_update" => Read {
            events: update.map_or_else(Vec::new, |update| {
                message_update(update.kind.as_deref(), update.delta.as_deref())
            }),
            requests: vec![],
        },
        Ok(Kind { kind, .. }) if READ_KINDS.contains(&kind.as_str()) => {
            serde_json::from_slice::<Value>(line)
                .ok()
                .inspect(|record| shown.remember_choice(record))
                .inspect(|record| shown.remember_estimate(record))
                .inspect(|record| shown.track_compaction(record))
                .filter(|record| !shown.repeats(record))
                .map(|record| Read {
                    events: shown
                        .subagents(&record)
                        .into_iter()
                        .chain(events_from(&record))
                        .map(|event| shown.with_current(event))
                        .collect(),
                    requests: requests_after(&record),
                })
                .unwrap_or_default()
        }
        _ => Read::default(),
    }
}

// pi reports a session change only as the response to what caused it.
fn requests_after(record: &Value) -> Vec<Value> {
    let changed = record["success"] == true && record["data"]["cancelled"] != true;
    match (record["type"].as_str(), record["command"].as_str()) {
        // A compaction leaves only the summary and the newest messages in the session.
        (Some("response"), Some("new_session" | "fork" | "clone" | "compact")) if changed => {
            session_requests().into()
        }
        // A reload can add or drop commands, so the slash menu is read again too.
        (Some("response"), Some("prompt"))
            if changed && (record["id"] == SESSION_PROMPT || record["id"] == RELOAD_PROMPT) =>
        {
            let mut requests = session_requests().to_vec();
            requests.push(json!({ "type": "get_commands" }));
            requests
        }
        // A new model can change the thinking level and the context window too.
        (Some("response"), Some("set_model" | "set_thinking_level")) if changed => {
            let level = record["id"]
                .as_str()
                .and_then(|id| id.strip_prefix(PICKED_LEVEL))
                .map(|level| json!({ "type": "set_thinking_level", "level": level }));
            level
                .into_iter()
                .chain([
                    json!({ "type": "get_state" }),
                    json!({ "type": "get_session_stats" }),
                ])
                .collect()
        }
        // The settled run is what grew the context.
        (Some("agent_settled"), _) => vec![json!({ "type": "get_session_stats" })],
        _ => vec![],
    }
}
