//! The Claude Code reader: one stdout line in, the harness events it means and the lines to write Claude Code next out. Holds the session, whether a turn runs, the control requests waiting for an answer, the messages waiting for their echo and the open tool calls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::asks::{self, Asks, Parsed};
use super::blocks::{self, Streamed};
use super::commands::{Change, Settings, commands};
use super::{requests, transcript};
use crate::Error;
use crate::harness::calls::OpenCalls;
use crate::harness::resume::{self, Resume};
use crate::harness::{DialogAnswer, HarnessEvent, Image, count, text};

/// What one record leads to: events for the chat and lines to write Claude Code.
#[derive(Default)]
pub(super) struct Read {
    pub(super) events: Vec<HarnessEvent>,
    pub(super) lines: Vec<Value>,
    /// The session the user picked to resume, which a new process opens.
    pub(super) resume: Option<String>,
}

/// A control request the adapter wrote, waiting for its answer.
enum Pending {
    Initialize,
    Thinking,
    Settings,
    Context,
    Interrupt,
    /// A model or effort change, after which `get_settings` reads what took hold; `model` is the picked entry's value.
    Changed {
        model: Option<String>,
    },
    Mode,
}

/// A message the adapter wrote, until Claude Code echoes it.
struct Sent {
    uuid: String,
    text: String,
    /// Sent while a turn ran, so it waits in the queue.
    steer: bool,
}

/// Reads one Claude Code process's stdout, line by line.
pub(super) struct Reader {
    cwd: PathBuf,
    /// The worktree's folder of session files.
    folder: Option<PathBuf>,
    /// The session `--resume` opened, until its history goes out.
    resumed: Option<String>,
    resume: Resume,
    session: Option<String>,
    running: bool,
    requests: u64,
    pending: HashMap<String, Pending>,
    messages: u64,
    sent: Vec<Sent>,
    settings: Settings,
    /// The model's context window, kept for the meter after a compaction.
    window: Option<u32>,
    /// The content block that streams now had text deltas, so its `assistant` record does not repeat them.
    text_streamed: bool,
    thought_started: bool,
    /// The open tool calls' tool names by id.
    tools: HashMap<String, String>,
    /// A local command's output, held until the command's echo.
    held: Vec<HarnessEvent>,
    calls: OpenCalls,
    asks: Asks,
}

impl Reader {
    pub(super) fn new(cwd: &Path, folder: Option<PathBuf>, resumed: Option<String>) -> Self {
        Reader {
            cwd: cwd.to_path_buf(),
            folder,
            resumed,
            resume: Resume::default(),
            session: None,
            running: false,
            requests: 0,
            pending: HashMap::new(),
            messages: 0,
            sent: vec![],
            settings: Settings::default(),
            window: None,
            text_streamed: false,
            thought_started: false,
            tools: HashMap::new(),
            held: vec![],
            calls: OpenCalls::default(),
            asks: Asks::default(),
        }
    }

    /// The first line Claude Code hears.
    pub(super) fn initialize(&mut self) -> Value {
        self.request(Pending::Initialize, "initialize", json!({}))
    }

    /// The user's message; mid-turn it waits in the queue until Claude Code takes it in.
    pub(super) fn send(
        &mut self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[String],
    ) -> Result<Read, Error> {
        if images.is_empty() && text.trim() == "/resume" {
            if self.running {
                return Err(Error::Message(
                    "Wait for the agent to finish, then change the session".into(),
                ));
            }
            let sessions = self.folder.as_deref().map_or_else(Vec::new, |folder| {
                transcript::sessions(folder, self.session.as_deref(), held_elsewhere)
            });
            return Ok(events(vec![self.resume.offer(sessions)]));
        }
        if let Some(event) = images
            .is_empty()
            .then(|| self.settings.command(text))
            .flatten()
        {
            return Ok(events(vec![event]));
        }
        self.messages += 1;
        let uuid = requests::uuid(self.messages);
        let line = requests::user(&uuid, text, images);
        self.sent.push(Sent {
            uuid,
            text: text.into(),
            steer: self.running,
        });
        Ok(Read {
            events: self.running.then(|| self.queue()).into_iter().collect(),
            lines: vec![line],
            resume: None,
        })
    }

    /// The user's answer to an open ask, and the line that answers Claude Code.
    pub(super) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Read {
        if id == resume::DIALOG {
            return Read {
                events: vec![HarnessEvent::DialogClosed],
                lines: vec![],
                resume: self.resume.pick(answer),
            };
        }
        if Settings::owns(id) {
            let line = self.settings.pick(id, answer).map(|change| match change {
                Change::Model(model) => self.request(
                    Pending::Changed {
                        model: Some(model.clone()),
                    },
                    "set_model",
                    json!({ "model": model }),
                ),
                Change::Effort(level) => self.request(
                    Pending::Changed { model: None },
                    "apply_flag_settings",
                    json!({ "settings": { "effortLevel": level } }),
                ),
                Change::Mode(mode) => self.request(
                    Pending::Mode,
                    "set_permission_mode",
                    json!({ "mode": mode }),
                ),
            });
            return Read {
                events: vec![HarnessEvent::DialogClosed],
                lines: line.into_iter().collect(),
                resume: None,
            };
        }
        let (events, line) = asks::answer(&mut self.asks, id, answer);
        Read {
            events,
            lines: line.into_iter().collect(),
            resume: None,
        }
    }

    /// Stops the running turn and drops the queued messages; none when no turn runs.
    pub(super) fn interrupt(&mut self) -> Option<Value> {
        self.running.then(|| {
            self.request(
                Pending::Interrupt,
                "interrupt",
                json!({ "cancel_queued": true }),
            )
        })
    }

    /// Stdout closed: the calls still open end as stopped.
    pub(super) fn end(&mut self) -> Vec<HarnessEvent> {
        self.calls.stop()
    }

    pub(super) fn feed(&mut self, line: &[u8]) -> Read {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            return Read::default();
        };
        // A subagent speaks on the same stdout.
        let subagent = !record["parent_tool_use_id"].is_null();
        let mut read = match record["type"].as_str() {
            Some("control_request") => match asks::parse(&record, &self.cwd) {
                Parsed::Ask(ask) => events(self.asks.push(ask)),
                Parsed::Refused { line, notice } => Read {
                    events: vec![HarnessEvent::Notice { text: notice }],
                    lines: vec![line],
                    resume: None,
                },
            },
            Some("control_cancel_request") => {
                let id = record["request_id"].as_str().unwrap_or_default();
                events(self.asks.withdraw(|reply| reply.request == id))
            }
            Some("control_response") => self.response(&record["response"]),
            Some("system") => events(self.system(&record)),
            Some("stream_event") if !subagent => events(self.stream(&record["event"])),
            Some("assistant") if !subagent => events(self.assistant(&record)),
            Some("user") if !subagent => events(self.user(&record)),
            Some("result") => self.result(&record),
            _ => Read::default(),
        };
        read.events = self.calls.track(read.events);
        read
    }

    fn request(&mut self, pending: Pending, subtype: &str, fields: Value) -> Value {
        self.requests += 1;
        let id = format!("chord-{}", self.requests);
        let line = requests::control(&id, subtype, fields);
        self.pending.insert(id, pending);
        line
    }

    fn queue(&self) -> HarnessEvent {
        HarnessEvent::Queue {
            steering: self
                .sent
                .iter()
                .filter(|sent| sent.steer)
                .map(|sent| sent.text.clone())
                .collect(),
        }
    }

    fn system(&mut self, record: &Value) -> Vec<HarnessEvent> {
        match record["subtype"].as_str() {
            Some("init") => {
                self.running = true;
                let mut events = vec![HarnessEvent::State { running: true }];
                let id = record["session_id"].as_str().unwrap_or_default();
                if !id.is_empty() && self.session.as_deref() != Some(id) {
                    self.session = Some(id.into());
                    events.push(HarnessEvent::SessionInfo { id: id.into() });
                }
                events
            }
            Some("status") => {
                let mut events = vec![];
                // A compaction sends its `init` only after it is done.
                if record["status"] == "compacting" {
                    self.running = true;
                    events.push(HarnessEvent::State { running: true });
                }
                // One follows a pick, and one a change Claude Code makes itself, as after an approved plan.
                if let Some(mode) = record["permissionMode"].as_str() {
                    events.push(self.settings.mode_changed(mode.into()));
                }
                events
            }
            Some("commands_changed") => vec![commands(&record["commands"])],
            Some("compact_boundary") => vec![
                HarnessEvent::Compaction {
                    summary: String::new(),
                    tokens_before: count(&record["compact_metadata"]["pre_tokens"]),
                },
                HarnessEvent::Context {
                    tokens: None,
                    window: self.window,
                },
            ],
            _ => vec![],
        }
    }

    fn stream(&mut self, event: &Value) -> Vec<HarnessEvent> {
        let delta = &event["delta"];
        match (event["type"].as_str(), delta["type"].as_str()) {
            (Some("content_block_start"), _) => {
                self.text_streamed = false;
                self.thought_started = false;
                vec![]
            }
            (Some("content_block_delta"), Some("text_delta")) => {
                self.text_streamed = true;
                vec![HarnessEvent::MessageChunk {
                    text: text(&delta["text"]),
                }]
            }
            (Some("content_block_delta"), Some("thinking_delta")) => {
                let text = text(&delta["thinking"]);
                if text.is_empty() {
                    return vec![];
                }
                let start = (!self.thought_started).then_some(HarnessEvent::ThoughtStart);
                self.thought_started = true;
                start
                    .into_iter()
                    .chain([HarnessEvent::ThoughtChunk { text }])
                    .collect()
            }
            _ => vec![],
        }
    }

    // One record per content block, sent before the block's `content_block_stop`.
    fn assistant(&mut self, record: &Value) -> Vec<HarnessEvent> {
        let streamed = Streamed {
            text: self.text_streamed,
            thought: self.thought_started,
        };
        let events = blocks::assistant(record, streamed, &mut self.tools, &self.cwd);
        self.text_streamed = false;
        self.thought_started = false;
        // A local command's output comes before the command's echo, so it waits for it.
        if record
            .get("local_command_run")
            .is_some_and(|run| !run.is_null() && *run != false)
        {
            self.held.extend(events);
            return vec![];
        }
        events
    }

    fn user(&mut self, record: &Value) -> Vec<HarnessEvent> {
        let content = &record["message"]["content"];
        if record["isReplay"] == true {
            // Claude Code echoes its own lines too, and those have no message of ours.
            let uuid = record["uuid"].as_str().unwrap_or_default();
            let Some(at) = self.sent.iter().position(|sent| sent.uuid == uuid) else {
                return vec![];
            };
            let sent = self.sent.remove(at);
            let mut events = vec![HarnessEvent::UserMessageChunk {
                text: blocks::user_text(content),
                images: blocks::images(content),
                at: blocks::timestamp(&record["timestamp"]),
            }];
            if sent.steer {
                events.push(self.queue());
            }
            events.append(&mut self.held);
            return events;
        }
        blocks::tool_results(content, &record["tool_use_result"], &mut self.tools)
    }

    fn result(&mut self, record: &Value) -> Read {
        self.running = false;
        self.tools.clear();
        let mut events = self.asks.withdraw(|_| true);
        events.append(&mut self.held);
        let aborted = matches!(
            record["terminal_reason"].as_str(),
            Some("aborted_streaming" | "aborted_tools")
        );
        if !aborted && record["subtype"] != "success" {
            let error = record["errors"]
                .as_array()
                .and_then(|errors| errors.first());
            events.push(HarnessEvent::MessageEnd {
                failed: true,
                at: None,
            });
            events.push(HarnessEvent::Notice {
                text: error
                    .and_then(Value::as_str)
                    .unwrap_or("The turn failed")
                    .into(),
            });
        }
        events.push(HarnessEvent::State { running: false });
        if aborted {
            events.push(HarnessEvent::Aborted);
        }
        Read {
            events,
            lines: vec![self.request(Pending::Context, "get_context_usage", json!({}))],
            resume: None,
        }
    }

    fn response(&mut self, response: &Value) -> Read {
        // Claude Code echoes each answer the adapter writes; those wait for nothing.
        let Some(pending) = response["request_id"]
            .as_str()
            .and_then(|id| self.pending.remove(id))
        else {
            return Read::default();
        };
        let answer = &response["response"];
        let error = (response["subtype"] == "error").then(|| {
            response["error"]
                .as_str()
                .unwrap_or("Claude Code refused a request")
                .to_string()
        });
        let mut read = Read::default();
        if let Some(text) = error.clone() {
            read.events.push(HarnessEvent::Notice { text });
        }
        match pending {
            // The chat waits for a history, even an empty one, before it takes a message.
            Pending::Initialize => {
                // Claude Code replays nothing on a resume, so the turns come from its file.
                read.events.push(HarnessEvent::HistoryStart);
                if let Some(id) = self.resumed.take() {
                    read.events
                        .push(HarnessEvent::SessionInfo { id: id.clone() });
                    let file = self
                        .folder
                        .as_deref()
                        .map(|folder| transcript::file(folder, &id));
                    let file = file.and_then(|file| std::fs::read_to_string(file).ok());
                    read.events
                        .extend(transcript::history(&file.unwrap_or_default(), &self.cwd));
                    self.session = Some(id);
                }
                read.events.push(HarnessEvent::HistoryEnd);
                if error.is_none() {
                    read.events.extend(self.settings.initialized(answer));
                    // Without `thinking_display` a thought streams and saves empty.
                    read.lines = vec![
                        self.request(
                            Pending::Thinking,
                            "set_max_thinking_tokens",
                            json!({ "thinking_display": "summarized" }),
                        ),
                        self.request(Pending::Settings, "get_settings", json!({})),
                        self.request(Pending::Context, "get_context_usage", json!({})),
                    ];
                }
            }
            Pending::Settings if error.is_none() => {
                read.events
                    .extend(self.settings.applied(&answer["applied"]));
            }
            Pending::Changed { model } if error.is_none() => {
                if let Some(model) = model {
                    self.settings.model_set(model);
                }
                read.lines = vec![self.request(Pending::Settings, "get_settings", json!({}))];
            }
            Pending::Context if error.is_none() => {
                self.window = count(&answer["maxTokens"]);
                read.events.push(HarnessEvent::Context {
                    tokens: count(&answer["totalTokens"]),
                    window: self.window,
                });
            }
            Pending::Interrupt if error.is_none() && self.sent.iter().any(|sent| sent.steer) => {
                self.sent.retain(|sent| !sent.steer);
                read.events.push(self.queue());
            }
            _ => {}
        }
        read
    }
}

fn events(events: Vec<HarnessEvent>) -> Read {
    Read {
        events,
        lines: vec![],
        resume: None,
    }
}
