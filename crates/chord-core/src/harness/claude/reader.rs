//! The Claude Code reader: one stdout line in, the harness events it means and the lines to write Claude Code next out. Holds the session, whether a turn runs, the control requests waiting for an answer, the messages waiting for their echo and the open tool calls.

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

/// What one record leads to: events for the chat, lines to write Claude Code, and what a new process opens in place of this one.
pub(super) type Read = relaunch::Read<Open>;

/// What a Claude Code process opens.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Open {
    /// The session to resume; none for a new one.
    pub(super) session: Option<String>,
    /// The resumed session's turns go into a new session, which Claude Code names at the first message.
    pub(super) fork: bool,
    /// The last reply the resumed session keeps; the later turns leave it.
    pub(super) at: Option<String>,
    /// The user's picks the process starts with, in place of the config's.
    pub(super) picks: Picks,
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
    Usage,
}

/// Claude Code sends `rate_limit_event` as often as each reply; usage is read again at most this often.
const USAGE_EVERY: Duration = Duration::from_secs(30);

/// The task types that are agents; a long shell command or a monitor is a task too.
const AGENT_TASKS: [&str; 4] = [
    "local_agent",
    "remote_agent",
    "in_process_teammate",
    "local_workflow",
];

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
    /// What the process opened, until its first message: Claude Code writes a fork's session and a rewind's branch only with it.
    opened: Open,
    dialogs: SessionDialogs,
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
    subagents: Subagents,
    /// When a `rate_limit_event` last read the usage.
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
            thought_started: false,
            tools: HashMap::new(),
            held: vec![],
            calls: OpenCalls::default(),
            asks: Asks::default(),
            subagents: Subagents::default(),
            usage_read: None,
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
        let command = session_command(text);
        if let Some(command @ ("new" | "reload")) = command {
            // A reload keeps the session and the user's picks; a new session takes the config's.
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
                // Claude Code runs it from the message text.
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

    /// What the process stands on, with the user's picks: what it opened until its first message, then its session.
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

    /// The model and effort the agent picker chose; Claude Code reads the settings again once they took hold.
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

    /// The user's answer to an open ask, and the line that answers Claude Code.
    pub(super) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Read {
        if let Some(session_dialogs::Answered { events, picked }) = self.dialogs.answer(id, answer)
        {
            let relaunch = match picked {
                Picked::Session(session) => session.map(|session| Open {
                    session: Some(session),
                    ..Open::default()
                }),
                // Before the first reply nothing stays, so a new session opens.
                Picked::Message(picked) => picked.map(|(back, message)| {
                    let standing = self.standing();
                    match message.before {
                        // A rewind of a fork that has no message yet stays a fork, so the old session keeps its turns.
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

    /// Stdout closed: the calls still open end as stopped, the open ask closes and the subagents go with the process.
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
        // A subagent speaks on the same stdout.
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
            // The event holds one window's state, so the usage is read whole.
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
        Read {
            events,
            lines: vec![
                self.request(Pending::Context, "get_context_usage", json!({})),
                self.request(Pending::Usage, "get_usage", json!({})),
            ],
            relaunch: None,
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
        // Usage is a hint: a login without it shows none, and says nothing.
        if let Some(text) = error.clone().filter(|_| !matches!(pending, Pending::Usage)) {
            read.events.push(HarnessEvent::Notice { text });
        }
        match pending {
            // The chat waits for a history, even an empty one, before it takes a message.
            Pending::Initialize => {
                // Claude Code replays nothing on a resume, so the turns come from its file.
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
                    // Without `thinking_display` a thought streams and saves empty.
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
                // Before a new session's first message Claude Code counts its own prompt, tools and skills.
                let empty = self.session.is_none() && self.opened.session.is_none();
                read.events.push(HarnessEvent::Context {
                    tokens: if empty {
                        Some(0)
                    } else {
                        count(&answer["totalTokens"])
                    },
                    window: self.window,
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

// The five-hour and weekly windows of a subscription; an API key has none.
fn usage_windows(usage: &Value) -> Vec<UsageWindow> {
    if usage["rate_limits_available"] != true {
        return vec![];
    }
    [("5h", "five_hour"), ("week", "seven_day")]
        .into_iter()
        .filter_map(|(name, key)| {
            let window = &usage["rate_limits"][key];
            // Claude Code writes the reset with an offset, which is always UTC.
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

    fn interrupt(&mut self) -> Option<Value> {
        Reader::interrupt(self)
    }

    fn end(&mut self) -> Vec<HarnessEvent> {
        Reader::end(self)
    }
}
