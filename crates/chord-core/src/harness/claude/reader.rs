use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::asks::{self, Asks, Parsed};
use super::blocks::{self, Streamed};
use super::commands::{Change, Picks, Settings, commands, session_command};
use super::{NAME, requests, transcript};
use crate::Error;
use crate::harness::calls::OpenCalls;
use crate::harness::earlier::Back;
use crate::harness::relaunch;
use crate::harness::session_dialogs::{self, Picked, SessionDialogs};
use crate::harness::subagents::Subagents;
use crate::harness::timestamp;
use crate::harness::{DialogAnswer, HarnessEvent, Image, SubagentState, UsageWindow, count, text};

pub(super) type Read = relaunch::Read<Open>;

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Open {
    pub(super) session: Option<String>,
    pub(super) fork: bool,
    pub(super) at: Option<String>,
    pub(super) picks: Picks,
}

enum Pending {
    Initialize,
    Thinking,
    Settings,
    Context,
    Interrupt,
    Changed { model: Option<String> },
    Mode,
    Usage,
}

const USAGE_EVERY: Duration = Duration::from_secs(30);

const AGENT_TASKS: [&str; 4] = [
    "local_agent",
    "remote_agent",
    "in_process_teammate",
    "local_workflow",
];

struct Sent {
    uuid: String,
    text: String,
    steer: bool,
}

pub(super) struct Reader {
    cwd: PathBuf,
    folder: Option<PathBuf>,
    opened: Open,
    dialogs: SessionDialogs,
    session: Option<String>,
    running: bool,
    requests: u64,
    pending: HashMap<String, Pending>,
    messages: u64,
    sent: Vec<Sent>,
    settings: Settings,
    window: Option<u32>,
    compacted: bool,
    text_streamed: bool,
    thought_started: bool,
    tools: HashMap<String, String>,
    held: Vec<HarnessEvent>,
    calls: OpenCalls,
    asks: Asks,
    subagents: Subagents,
    usage_read: Option<Instant>,
}

impl Reader {
    pub(super) fn new(cwd: &Path, folder: Option<PathBuf>, opened: Open) -> Self {
        let settings = Settings::with_picks(opened.picks.clone());
        Reader {
            cwd: cwd.to_path_buf(),
            folder,
            opened,
            dialogs: SessionDialogs::default(),
            session: None,
            running: false,
            requests: 0,
            pending: HashMap::new(),
            messages: 0,
            sent: vec![],
            settings,
            window: None,
            text_streamed: false,
            compacted: false,
            thought_started: false,
            tools: HashMap::new(),
            held: vec![],
            calls: OpenCalls::default(),
            asks: Asks::default(),
            subagents: Subagents::default(),
            usage_read: None,
        }
    }

    pub(super) fn initialize(&mut self) -> Value {
        self.request(Pending::Initialize, "initialize", json!({}))
    }

    pub(super) fn send(
        &mut self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[String],
    ) -> Result<Read, Error> {
        let command = session_command(text);
        if let Some(command @ ("new" | "reload")) = command {
            let open = match command {
                "reload" => self.standing(),
                _ => Open::default(),
            };
            return Ok(Read {
                relaunch: Some(open),
                ..Read::default()
            });
        }
        if let Some(command @ ("resume" | "fork" | "rewind" | "clone" | "compact")) = command {
            if self.running {
                return Err(Error::Message(
                    "Wait for the agent to finish, then change the session".into(),
                ));
            }
            let event = match command {
                "resume" => {
                    let sessions = self.folder.as_deref().map_or_else(Vec::new, |folder| {
                        transcript::sessions(folder, self.session.as_deref(), held_elsewhere)
                    });
                    self.dialogs.offer_sessions(sessions)
                }
                "clone" => {
                    let standing = self.standing();
                    if standing.session.is_none() {
                        return Err(Error::Message("The session has no message to copy".into()));
                    }
                    return Ok(Read {
                        relaunch: Some(Open {
                            fork: true,
                            ..standing
                        }),
                        ..Read::default()
                    });
                }
                "compact" => return Ok(self.message(text, images)),
                _ => {
                    let back = if command == "fork" {
                        Back::Fork
                    } else {
                        Back::Rewind
                    };
                    let standing = self.standing();
                    let file = self.folder.as_deref().zip(standing.session.as_deref());
                    let file = file.and_then(|(folder, id)| {
                        std::fs::read_to_string(transcript::file(folder, id)).ok()
                    });
                    let prompts =
                        transcript::prompts(&file.unwrap_or_default(), standing.at.as_deref());
                    self.dialogs.offer_messages(back, prompts)
                }
            };
            return Ok(events(vec![event]));
        }
        if let Some(event) = images
            .is_empty()
            .then(|| self.settings.command(text))
            .flatten()
        {
            return Ok(events(vec![event]));
        }
        Ok(self.message(text, images))
    }

    fn standing(&self) -> Open {
        let picks = self.settings.picks().clone();
        match &self.opened.session {
            Some(_) => Open {
                picks,
                ..self.opened.clone()
            },
            None => Open {
                session: self.session.clone(),
                picks,
                ..Open::default()
            },
        }
    }

    fn message(&mut self, text: &str, images: &[Image]) -> Read {
        self.messages += 1;
        let uuid = requests::uuid(self.messages);
        let line = requests::user(&uuid, text, images);
        self.sent.push(Sent {
            uuid,
            text: text.into(),
            steer: self.running,
        });
        Read {
            events: self.running.then(|| self.queue()).into_iter().collect(),
            lines: vec![line],
            relaunch: None,
        }
    }

    pub(super) fn pick_model(&mut self, model: &str, effort: Option<&str>) -> Read {
        self.settings.chose(model, effort);
        let mut lines = vec![self.request(
            Pending::Changed {
                model: Some(model.into()),
            },
            "set_model",
            json!({ "model": model }),
        )];
        if let Some(level) = effort {
            lines.push(self.request(
                Pending::Changed { model: None },
                "apply_flag_settings",
                json!({ "settings": { "effortLevel": level } }),
            ));
        }
        Read {
            lines,
            ..Read::default()
        }
    }

    pub(super) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Read {
        if let Some(session_dialogs::Answered { events, picked }) = self.dialogs.answer(id, answer)
        {
            let relaunch = match picked {
                Picked::Session(session) => session.map(|session| Open {
                    session: Some(session),
                    ..Open::default()
                }),
                Picked::Message(picked) => picked.map(|(back, message)| {
                    let standing = self.standing();
                    match message.before {
                        Some(at) => Open {
                            fork: back == Back::Fork || standing.fork,
                            at: Some(at),
                            ..standing
                        },
                        None => Open {
                            picks: self.settings.picks().clone(),
                            ..Open::default()
                        },
                    }
                }),
            };
            return Read {
                events,
                lines: vec![],
                relaunch,
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
                relaunch: None,
            };
        }
        let (events, line) = asks::answer(&mut self.asks, id, answer);
        Read {
            events,
            lines: line.into_iter().collect(),
            relaunch: None,
        }
    }

    pub(super) fn interrupt(&mut self) -> Option<Value> {
        self.running.then(|| {
            self.request(
                Pending::Interrupt,
                "interrupt",
                json!({ "cancel_queued": true }),
            )
        })
    }

    pub(super) fn end(&mut self) -> Vec<HarnessEvent> {
        let mut events = self.asks.withdraw(|_| true);
        events.extend(self.calls.stop());
        events.extend(self.subagents.clear());
        events
    }

    pub(super) fn feed(&mut self, line: &[u8]) -> Read {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            return Read::default();
        };
        let subagent = !record["parent_tool_use_id"].is_null();
        let mut read = match record["type"].as_str() {
            Some("control_request") => match asks::parse(&record, &self.cwd) {
                Parsed::Ask(ask) => events(self.asks.push(ask)),
                Parsed::Refused { line, notice } => Read {
                    events: vec![HarnessEvent::Notice { text: notice }],
                    lines: vec![line],
                    relaunch: None,
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
            Some("rate_limit_event") => {
                let due = self.usage_read.is_none_or(|at| at.elapsed() >= USAGE_EVERY);
                if due {
                    self.usage_read = Some(Instant::now());
                }
                Read {
                    lines: due
                        .then(|| self.request(Pending::Usage, "get_usage", json!({})))
                        .into_iter()
                        .collect(),
                    ..Read::default()
                }
            }
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
                self.opened = Open::default();
                if !id.is_empty() && self.session.as_deref() != Some(id) {
                    self.session = Some(id.into());
                    events.push(HarnessEvent::SessionInfo { id: id.into() });
                }
                events
            }
            Some("status") => {
                let mut events = vec![];
                if record["status"] == "compacting" {
                    self.running = true;
                    events.push(HarnessEvent::State { running: true });
                    events.push(HarnessEvent::Compacting { active: true });
                }
                if let Some(mode) = record["permissionMode"].as_str() {
                    events.push(self.settings.mode_changed(mode.into()));
                }
                events
            }
            Some("commands_changed") => vec![commands(&record["commands"])],
            Some("task_started") => {
                let agent = AGENT_TASKS.contains(&record["task_type"].as_str().unwrap_or_default());
                let (Some(id), true) = (record["task_id"].as_str(), agent) else {
                    return vec![];
                };
                let name = ["subagent_type", "workflow_name", "description"]
                    .into_iter()
                    .find_map(|key| record[key].as_str().filter(|name| !name.is_empty()));
                vec![
                    self.subagents
                        .start(id, record["parent_task_id"].as_str(), name),
                ]
            }
            Some("task_progress") => record["task_id"]
                .as_str()
                .zip(record["last_tool_name"].as_str())
                .and_then(|(id, tool)| self.subagents.tool(id, tool))
                .into_iter()
                .collect(),
            Some("task_notification" | "task_updated") => {
                let status = record["status"]
                    .as_str()
                    .or(record["patch"]["status"].as_str());
                let state = match status {
                    Some("completed") => SubagentState::Done,
                    Some("failed" | "killed" | "stopped") => SubagentState::Error,
                    _ => return vec![],
                };
                let id = record["task_id"].as_str().unwrap_or_default();
                self.subagents.end(id, state).into_iter().collect()
            }
            Some("compact_boundary") => {
                self.compacted = true;
                vec![
                    HarnessEvent::Compacting { active: false },
                    HarnessEvent::Compaction {
                        summary: String::new(),
                        tokens_before: count(&record["compact_metadata"]["pre_tokens"]),
                    },
                    HarnessEvent::Context {
                        tokens: count(&record["compact_metadata"]["post_tokens"]),
                        window: self.window,
                        estimated: false,
                    },
                ]
            }
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

    fn assistant(&mut self, record: &Value) -> Vec<HarnessEvent> {
        self.compacted = false;
        let streamed = Streamed {
            text: self.text_streamed,
            thought: self.thought_started,
        };
        let events = blocks::assistant(record, streamed, &mut self.tools, &self.cwd);
        self.text_streamed = false;
        self.thought_started = false;
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
            let uuid = record["uuid"].as_str().unwrap_or_default();
            let Some(at) = self.sent.iter().position(|sent| sent.uuid == uuid) else {
                return vec![];
            };
            let sent = self.sent.remove(at);
            let mut events = vec![HarnessEvent::UserMessageChunk {
                text: blocks::user_text(content),
                images: blocks::images(content),
                at: timestamp(&record["timestamp"]),
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
        let context = (!self.compacted)
            .then(|| self.request(Pending::Context, "get_context_usage", json!({})));
        Read {
            events,
            lines: context
                .into_iter()
                .chain([self.request(Pending::Usage, "get_usage", json!({}))])
                .collect(),
            relaunch: None,
        }
    }

    fn response(&mut self, response: &Value) -> Read {
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
        if let Some(text) = error.clone().filter(|_| !matches!(pending, Pending::Usage)) {
            read.events.push(HarnessEvent::Notice { text });
        }
        match pending {
            Pending::Initialize => {
                read.events.push(HarnessEvent::HistoryStart);
                let opened = self.opened.clone();
                if let Some(id) = opened.session {
                    if !opened.fork {
                        read.events
                            .push(HarnessEvent::SessionInfo { id: id.clone() });
                        self.session = Some(id.clone());
                    }
                    let file = self
                        .folder
                        .as_deref()
                        .map(|folder| transcript::file(folder, &id));
                    let file = file.and_then(|file| std::fs::read_to_string(file).ok());
                    read.events.extend(transcript::history(
                        &file.unwrap_or_default(),
                        opened.at.as_deref(),
                        &self.cwd,
                    ));
                }
                read.events.push(HarnessEvent::HistoryEnd);
                if error.is_none() {
                    read.events.extend(self.settings.initialized(answer));
                    read.lines = vec![
                        self.request(
                            Pending::Thinking,
                            "set_max_thinking_tokens",
                            json!({ "thinking_display": "summarized" }),
                        ),
                        self.request(Pending::Settings, "get_settings", json!({})),
                        self.request(Pending::Context, "get_context_usage", json!({})),
                        self.request(Pending::Usage, "get_usage", json!({})),
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
                let empty = self.session.is_none() && self.opened.session.is_none();
                read.events.push(HarnessEvent::Context {
                    tokens: if empty {
                        Some(0)
                    } else {
                        count(&answer["totalTokens"])
                    },
                    window: self.window,
                    estimated: false,
                });
            }
            Pending::Usage if error.is_none() => read.events.push(HarnessEvent::Usage {
                harness: NAME,
                windows: usage_windows(answer),
            }),
            Pending::Interrupt if error.is_none() && self.sent.iter().any(|sent| sent.steer) => {
                self.sent.retain(|sent| !sent.steer);
                read.events.push(self.queue());
            }
            _ => {}
        }
        read
    }
}

fn usage_windows(usage: &Value) -> Vec<UsageWindow> {
    if usage["rate_limits_available"] != true {
        return vec![];
    }
    [("5h", "five_hour"), ("week", "seven_day")]
        .into_iter()
        .filter_map(|(name, key)| {
            let window = &usage["rate_limits"][key];
            let resets_at = window["resets_at"]
                .as_str()
                .map(|at| Value::from(at.replace("+00:00", "Z")));
            Some(UsageWindow::used(
                name,
                window["utilization"].as_f64()?,
                resets_at.as_ref().and_then(timestamp),
            ))
        })
        .collect()
}

fn events(events: Vec<HarnessEvent>) -> Read {
    Read {
        events,
        lines: vec![],
        relaunch: None,
    }
}

impl relaunch::Reader for Reader {
    type Open = Open;

    fn initialize(&mut self) -> Value {
        Reader::initialize(self)
    }

    fn feed(&mut self, line: &[u8]) -> Read {
        Reader::feed(self, line)
    }

    fn takes_images(&self, text: &str) -> bool {
        session_command(text).is_none()
    }

    fn send(
        &mut self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[String],
    ) -> Result<Read, Error> {
        Reader::send(self, text, images, held_elsewhere)
    }

    fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Result<Read, Error> {
        Ok(Reader::answer(self, id, answer))
    }

    fn pick_model(&mut self, id: &str, level: Option<&str>) -> Read {
        Reader::pick_model(self, id, level)
    }

    fn interrupt(&mut self) -> Read {
        Read {
            lines: Reader::interrupt(self).into_iter().collect(),
            ..Read::default()
        }
    }

    fn end(&mut self) -> Vec<HarnessEvent> {
        Reader::end(self)
    }
}
