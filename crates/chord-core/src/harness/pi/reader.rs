//! The pi reader: one stdout line in, the harness events it means and the commands to send pi next out. Holds what the shown session and the open tool calls need across lines.

use std::hash::{DefaultHasher, Hash, Hasher};

use serde::Deserialize;
use serde_json::{Value, json};

use super::commands::session_requests;
use super::records::{OpenCalls, capitalized, events_from, message_update, subagent_nodes};
use super::{Picker, SESSION_PROMPT, SUBAGENTS_PREFIX};
use crate::harness::{DialogAsk, HarnessEvent};

#[derive(Deserialize)]
struct Kind {
    #[serde(rename = "type")]
    kind: String,
    // A reply chunk needs only this field, so its record is not parsed a second time.
    #[serde(rename = "assistantMessageEvent")]
    update: Option<Value>,
}

const READ_KINDS: [&str; 12] = [
    "extension_ui_request",
    "compaction_start",
    "compaction_end",
    "extension_error",
    "agent_start",
    "agent_settled",
    "message_update",
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
}

impl Reader {
    /// A run that settles closes the tool calls pi left open.
    pub(super) fn feed(&mut self, line: &[u8]) -> Read {
        let Read { events, requests } = read_line(line, &mut self.shown);
        Read {
            events: self.calls.track(events),
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
        if let HarnessEvent::Dialog {
            id,
            ask: DialogAsk::Menu { current, .. },
            ..
        } = &mut event
        {
            match Picker::from_id(id) {
                Some(Picker::Model) => *current = self.model.clone(),
                Some(Picker::Thinking) => *current = self.thinking.clone(),
                None => {}
            }
        }
        event
    }
}

// Tool output and turn summaries are large and unused, so only the records `events_from` reads get a full parse.
fn read_line(line: &[u8], shown: &mut ShownSession) -> Read {
    match serde_json::from_slice::<Kind>(line) {
        Ok(Kind { kind, update }) if kind == "message_update" => Read {
            events: message_update(update.as_ref().unwrap_or(&Value::Null)),
            requests: vec![],
        },
        Ok(Kind { kind, .. }) if READ_KINDS.contains(&kind.as_str()) => {
            serde_json::from_slice::<Value>(line)
                .ok()
                .inspect(|record| shown.remember_choice(record))
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
        (Some("response"), Some("new_session" | "fork" | "clone")) if changed => {
            session_requests().into()
        }
        // A reload can add or drop commands, so the slash menu is read again too.
        (Some("response"), Some("prompt")) if changed && record["id"] == SESSION_PROMPT => {
            let mut requests = session_requests().to_vec();
            requests.push(json!({ "type": "get_commands" }));
            requests
        }
        // A new model can change the thinking level and the context window too.
        (Some("response"), Some("set_model" | "set_thinking_level")) if changed => {
            vec![
                json!({ "type": "get_state" }),
                json!({ "type": "get_session_stats" }),
            ]
        }
        // The settled run is what grew the context.
        (Some("agent_settled"), _) => vec![json!({ "type": "get_session_stats" })],
        _ => vec![],
    }
}
