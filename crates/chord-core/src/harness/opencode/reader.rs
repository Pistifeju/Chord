use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::asks::{self, Asks, Parsed};
use super::commands::{Command, Pick, Settings};
use super::updates::Updates;
use super::{HARNESS, requests};
use crate::Error;
use crate::harness::calls::OpenCalls;
use crate::harness::relaunch;
use crate::harness::resume::Session;
use crate::harness::session_dialogs::{Answered, Picked, SessionDialogs};
use crate::harness::subagents::Subagents;
use crate::harness::{DialogAnswer, HarnessEvent, Image, SubagentState, text, timestamp};

struct Message {
    text: String,
    images: Vec<Image>,
}

type Read = relaunch::Read<Option<String>>;

const QUIET: Duration = Duration::from_secs(20);
const QUIET_CALL: Duration = Duration::from_secs(120);

const AUTH_REQUIRED: i64 = -32000;

enum Pending {
    Initialize,
    Session,
    Load {
        session: String,
        previous: Option<String>,
    },
    List {
        held: Vec<String>,
    },
    Prompt,
    Config(Option<String>),
    Fork,
    ProbeSession,
    Probe(String),
    Steer(Message),
}

pub(super) struct Reader {
    cwd: PathBuf,
    mcp: Value,
    instructions: Option<String>,
    instruct: bool,
    saved: Option<String>,
    next_id: u64,
    pending: HashMap<u64, Pending>,
    session: Option<String>,
    session_error: Option<String>,
    running: bool,
    next: Vec<Message>,
    stopped: bool,
    own_turn: bool,
    updates: Updates,
    settings: Settings,
    wanted: Option<(String, Option<String>)>,
    probe: Option<(String, Vec<String>)>,
    probed: bool,
    calls: OpenCalls,
    asks: Asks,
    dialogs: SessionDialogs,
    subagents: Subagents,
}

impl Reader {
    pub(super) fn new(
        cwd: &Path,
        saved: Option<String>,
        server: Option<(String, Vec<String>)>,
        instructions: Option<String>,
    ) -> Self {
        Reader {
            cwd: cwd.to_path_buf(),
            mcp: requests::mcp_servers(server),
            instructions,
            instruct: false,
            saved,
            next_id: 0,
            pending: HashMap::new(),
            session: None,
            session_error: None,
            running: false,
            next: vec![],
            stopped: false,
            own_turn: false,
            updates: Updates::new(cwd),
            settings: Settings::default(),
            wanted: None,
            probe: None,
            probed: false,
            calls: OpenCalls::default(),
            asks: Asks::default(),
            dialogs: SessionDialogs::default(),
            subagents: Subagents::default(),
        }
    }

    fn cwd(&self) -> String {
        self.cwd.to_string_lossy().into_owned()
    }

    fn new_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn request(&mut self, pending: Pending) -> u64 {
        let id = self.new_id();
        self.pending.insert(id, pending);
        id
    }

    fn loading(&self) -> bool {
        self.pending
            .values()
            .any(|pending| matches!(pending, Pending::Load { .. }))
    }

    fn open_session(&self) -> Result<String, Error> {
        if self.loading() {
            return Err(Error::Message(
                "OpenCode is still loading the session. Send the message again in a moment".into(),
            ));
        }
        self.session.clone().ok_or_else(|| {
            Error::Message(self.session_error.clone().unwrap_or_else(|| {
                "OpenCode is still starting. Send the message again in a moment".into()
            }))
        })
    }

    fn prompt(&mut self, session: &str, message: Message) -> Read {
        self.running = true;
        self.own_turn = false;
        let id = self.request(Pending::Prompt);
        let first = std::mem::take(&mut self.instruct);
        let instructions = self.instructions.as_deref().filter(|_| first);
        let line = requests::prompt(id, session, &message.text, &message.images, instructions);
        Read {
            events: vec![
                HarnessEvent::UserMessageChunk {
                    text: message.text,
                    images: message.images,
                    at: None,
                },
                HarnessEvent::State { running: true },
            ],
            lines: vec![line],
            relaunch: None,
        }
    }

    fn response(&mut self, pending: Pending, record: &Value) -> Read {
        let result = &record["result"];
        let error = record["error"]["message"].as_str().map(String::from);
        match pending {
            Pending::Initialize => {
                let version = &result["protocolVersion"];
                let refused = error.or_else(|| {
                    (*version != 1).then(|| {
                        format!(
                            "OpenCode speaks ACP version {version} and Chord speaks version 1. Upgrade it with: {}",
                            HARNESS.upgrade
                        )
                    })
                });
                match refused {
                    Some(error) => self.no_session(error),
                    None => match self.saved.take() {
                        Some(saved) => self.load(saved, None),
                        None => self.session_new(),
                    },
                }
            }
            Pending::Session => match (error, result["sessionId"].as_str()) {
                (None, Some(session)) => {
                    self.instruct = true;
                    let mut read = self.opened(session, result);
                    read.events
                        .extend([HarnessEvent::HistoryStart, HarnessEvent::HistoryEnd]);
                    read
                }
                (error, _) => {
                    self.no_session(error.unwrap_or_else(|| "OpenCode opened no session".into()))
                }
            },
            Pending::Load { session, previous } => match error {
                Some(error) => match previous {
                    Some(previous) => {
                        let mut read = self.load(previous, None);
                        read.events.insert(0, HarnessEvent::Notice { text: error });
                        read
                    }
                    None => {
                        self.session = None;
                        self.session_new()
                    }
                },
                None => {
                    let mut read = self.opened(&session, result);
                    read.events.push(HarnessEvent::HistoryEnd);
                    if let Some(previous) = previous.filter(|previous| *previous != session) {
                        let id = self.new_id();
                        read.lines.push(requests::session_close(id, &previous));
                    }
                    read
                }
            },
            Pending::List { held } => match error {
                Some(error) => events_only(vec![HarnessEvent::Notice { text: error }]),
                None => {
                    let sessions = self.sessions(&result["sessions"], &held);
                    events_only(vec![self.dialogs.offer_sessions(sessions)])
                }
            },
            Pending::Prompt => self.prompt_answered(record),
            Pending::Config(level) => {
                if let Some(error) = error {
                    return events_only(vec![HarnessEvent::Notice { text: error }]);
                }
                let mut read = events_only(self.settings.read(&result["configOptions"]));
                let effort = level.and_then(|level| self.settings.effort_pick(&level));
                read.lines
                    .extend(effort.and_then(|pick| self.set(pick, None)));
                read
            }
            Pending::ProbeSession => match result["sessionId"].as_str() {
                Some(probe) => {
                    self.probe = Some((probe.into(), self.settings.model_ids()));
                    self.probe_next()
                }
                None => Read::default(),
            },
            Pending::Probe(model) => {
                if error.is_none() {
                    self.settings.learn(&model, &result["configOptions"]);
                }
                let mut read = self.probe_next();
                if self.probe.is_none() {
                    read.events.push(self.settings.models());
                }
                read
            }
            Pending::Fork => match (error, result["sessionId"].as_str()) {
                (None, Some(session)) => {
                    self.instruct = false;
                    let cleared = self.subagents.clear();
                    let mut read = self.opened(session, result);
                    read.events.splice(0..0, cleared);
                    read
                }
                (error, _) => events_only(vec![HarnessEvent::Notice {
                    text: error.unwrap_or_else(|| "OpenCode copied no session".into()),
                }]),
            },
            Pending::Steer(message) => match (error, result["outcome"].as_str()) {
                (Some(error), _) => events_only(vec![HarnessEvent::Notice { text: error }]),
                (None, Some("injected")) => events_only(vec![HarnessEvent::UserMessageChunk {
                    text: message.text,
                    images: message.images,
                    at: None,
                }]),
                (None, Some("promptRequired")) if self.stopped => Read::default(),
                (None, Some("promptRequired")) if self.running => {
                    self.next.push(message);
                    Read::default()
                }
                (None, Some("promptRequired")) => match self.session.clone() {
                    Some(session) => self.prompt(&session, message),
                    None => Read::default(),
                },
                _ => Read::default(),
            },
        }
    }

    fn session_new(&mut self) -> Read {
        let id = self.request(Pending::Session);
        lines(requests::session_new(id, &self.cwd(), &self.mcp))
    }

    fn load(&mut self, session: String, previous: Option<String>) -> Read {
        self.own_turn = false;
        self.instruct = false;
        let id = self.request(Pending::Load {
            session: session.clone(),
            previous,
        });
        let line = requests::session_load(id, &session, &self.cwd(), &self.mcp);
        self.session = Some(session);
        let mut events = vec![HarnessEvent::HistoryStart];
        events.extend(self.subagents.clear());
        Read {
            events,
            lines: vec![line],
            relaunch: None,
        }
    }

    fn opened(&mut self, session: &str, result: &Value) -> Read {
        self.session = Some(session.into());
        self.updates.settle();
        let mut events: Vec<HarnessEvent> = self.updates.close().into_iter().collect();
        events.extend(self.calls.stop());
        events.push(HarnessEvent::SessionInfo { id: session.into() });
        events.extend(self.settings.read(&result["configOptions"]));
        events.push(self.settings.commands_event());
        let mut read = events_only(events);
        if let Some((id, level)) = self.wanted.take() {
            read.lines.extend(self.pick_line(&id, level));
        }
        if !self.probed {
            self.probed = true;
            let id = self.request(Pending::ProbeSession);
            read.lines.push(requests::session_new(
                id,
                &self.cwd(),
                &Value::Array(vec![]),
            ));
        }
        read
    }

    fn sessions(&self, list: &Value, held: &[String]) -> Vec<Session> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        list.as_array()
            .into_iter()
            .flatten()
            .filter_map(|session| {
                let id = session["sessionId"].as_str()?;
                let free =
                    Some(id) != self.session.as_deref() && !held.iter().any(|other| other == id);
                let updated =
                    timestamp(&session["updatedAt"]).map_or(now, |ms| (ms / 1000.0) as u64);
                free.then(|| Session {
                    id: id.into(),
                    label: session["title"].as_str().unwrap_or(id).into(),
                    age: now.saturating_sub(updated),
                })
            })
            .collect()
    }

    fn child(&mut self, params: &Value) -> Vec<HarnessEvent> {
        if self.session.is_none() || params["rootSessionId"].as_str() != self.session.as_deref() {
            return vec![];
        }
        let child = text(&params["childSessionId"]);
        let title = text(&params["title"]);
        let event = match (params["type"].as_str(), params["status"].as_str()) {
            (Some("status"), Some("created" | "running")) => Some(
                self.subagents.start(
                    &child,
                    params["parentSessionId"].as_str(),
                    Some(&title)
                        .filter(|title| !title.is_empty())
                        .map(String::as_str),
                ),
            ),
            (Some("status"), Some("completed")) => self.subagents.end(&child, SubagentState::Done),
            (Some("status"), Some("failed" | "interrupted")) => {
                self.subagents.end(&child, SubagentState::Error)
            }
            (Some("update"), _) => {
                let update = &params["update"];
                let running = update["sessionUpdate"] == "tool_call_update"
                    && update["status"] == "in_progress";
                let tool = text(&update["title"]);
                let prefix = format!("{title}: ");
                let tool = tool.strip_prefix(&prefix).unwrap_or(&tool);
                running.then(|| self.subagents.tool(&child, tool)).flatten()
            }
            _ => None,
        };
        event.into_iter().collect()
    }

    fn no_session(&mut self, error: String) -> Read {
        self.session_error = Some(error.clone());
        events_only(vec![
            HarnessEvent::Notice { text: error },
            HarnessEvent::HistoryEnd,
        ])
    }

    fn prompt_answered(&mut self, record: &Value) -> Read {
        self.running = false;
        self.updates.settle();
        let mut events = self.asks.withdraw(|_| true);
        events.extend(self.updates.close());
        let reason = record["result"]["stopReason"].as_str();
        let error = &record["error"];
        let failure = if error.is_object() {
            let message = error["message"]
                .as_str()
                .unwrap_or("OpenCode refused the prompt");
            Some(if error["code"].as_i64() == Some(AUTH_REQUIRED) {
                format!("{message}. Sign in with: opencode auth login")
            } else {
                message.into()
            })
        } else {
            match reason {
                Some("end_turn" | "cancelled") => None,
                Some("max_tokens") => Some("The reply reached the model's output limit".into()),
                Some("max_turn_requests") => {
                    Some("The turn reached OpenCode's limit of model requests".into())
                }
                Some("refusal") => Some("The model refused to answer".into()),
                other => Some(format!(
                    "OpenCode ended the turn: {}",
                    other.unwrap_or("no reason given")
                )),
            }
        };
        if let Some(text) = failure {
            events.retain(|event| !matches!(event, HarnessEvent::MessageEnd { .. }));
            events.push(HarnessEvent::MessageEnd {
                failed: true,
                at: None,
            });
            events.push(HarnessEvent::Notice { text });
        }
        events.push(HarnessEvent::State { running: false });
        if reason == Some("cancelled") {
            events.push(HarnessEvent::Aborted);
        }
        let mut read = events_only(events);
        let mut held = std::mem::take(&mut self.next).into_iter();
        if let (Some(first), Some(session)) = (held.next(), self.session.clone()) {
            for next in [self.prompt(&session, first)]
                .into_iter()
                .chain(held.map(|message| self.steer(&session, message)))
            {
                read.events.extend(next.events);
                read.lines.extend(next.lines);
            }
        }
        read
    }

    fn probe_next(&mut self) -> Read {
        let Some((probe, models)) = self.probe.as_mut() else {
            return Read::default();
        };
        let probe = probe.clone();
        while let Some(model) = models.pop() {
            if let Some((config, value)) = self.settings.model_pick(&model) {
                let id = self.request(Pending::Probe(model));
                return lines(requests::set_config_option(id, &probe, &config, &value));
            }
        }
        self.probe = None;
        let id = self.new_id();
        lines(requests::session_delete(id, &probe))
    }

    fn pick_line(&mut self, id: &str, level: Option<String>) -> Option<Value> {
        let pick = self.settings.model_pick(id)?;
        self.set(pick, level)
    }

    fn set(&mut self, (config, value): Pick, level: Option<String>) -> Option<Value> {
        let session = self.session.clone()?;
        let id = self.request(Pending::Config(level));
        Some(requests::set_config_option(id, &session, &config, &value))
    }

    fn steer(&mut self, session: &str, message: Message) -> Read {
        let id = self.new_id();
        let line = requests::steer(id, session, &message.text, &message.images);
        self.pending.insert(id, Pending::Steer(message));
        lines(line)
    }
}

fn lines(line: Value) -> Read {
    Read {
        lines: vec![line],
        ..Read::default()
    }
}

fn events_only(events: Vec<HarnessEvent>) -> Read {
    Read {
        events,
        ..Read::default()
    }
}

impl relaunch::Reader for Reader {
    type Open = Option<String>;

    fn initialize(&mut self) -> Value {
        requests::initialize(self.request(Pending::Initialize))
    }

    fn feed(&mut self, line: &[u8]) -> Read {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            return Read::default();
        };
        let mut read = match (&record["id"], record["method"].as_str()) {
            (Value::Null, Some("session/update")) => {
                let params = &record["params"];
                if self.session.is_none() || params["sessionId"].as_str() != self.session.as_deref()
                {
                    return Read::default();
                }
                let update = &params["update"];
                let mut events = vec![];
                let loading = self.loading();
                let opens = matches!(
                    update["sessionUpdate"].as_str(),
                    Some("agent_message_chunk" | "agent_thought_chunk" | "tool_call")
                );
                if opens && !self.running && !self.own_turn && !loading {
                    self.own_turn = true;
                    events.push(HarnessEvent::State { running: true });
                }
                events.extend(self.updates.update(update, &mut self.settings));
                events_only(events)
            }
            (Value::Null, Some("opencode/session/child_update")) => {
                events_only(self.child(&record["params"]))
            }
            (Value::Null, Some(_)) => Read::default(),
            (id, Some(method)) => match asks::parse(id, method, &record["params"], &self.cwd) {
                Parsed::Ask(ask) => events_only(self.asks.push(ask)),
                Parsed::Refused { line, notice } => Read {
                    events: vec![HarnessEvent::Notice { text: notice }],
                    lines: vec![line],
                    relaunch: None,
                },
            },
            (id, None) => match id.as_u64().and_then(|id| self.pending.remove(&id)) {
                Some(pending) => self.response(pending, &record),
                None => Read::default(),
            },
        };
        read.events = self.calls.track(read.events);
        read
    }

    fn takes_images(&self, text: &str) -> bool {
        self.settings.command(text).is_none()
    }

    fn send(
        &mut self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[String],
    ) -> Result<Read, Error> {
        let busy = self.running || self.own_turn;
        match self.settings.command(text) {
            Some(Command::New) => Ok(Read {
                relaunch: Some(None),
                ..Read::default()
            }),
            Some(Command::Reload) => Ok(Read {
                relaunch: Some(self.session.clone().or_else(|| self.saved.clone())),
                ..Read::default()
            }),
            Some(Command::Menu(event)) => {
                self.open_session()?;
                Ok(events_only(vec![event]))
            }
            Some(Command::Resume | Command::Clone | Command::Compact) if busy => Err(
                Error::Message("Wait for the agent to finish, then change the session".into()),
            ),
            Some(Command::Resume) => {
                self.open_session()?;
                let held = held_elsewhere.to_vec();
                let id = self.request(Pending::List { held });
                Ok(lines(requests::session_list(id, &self.cwd())))
            }
            Some(Command::Clone) => {
                let session = self.open_session()?;
                let id = self.request(Pending::Fork);
                Ok(lines(requests::session_fork(
                    id,
                    &session,
                    &self.cwd(),
                    &self.mcp,
                )))
            }
            Some(Command::Compact) => {
                let session = self.open_session()?;
                let message = Message {
                    text: text.into(),
                    images: vec![],
                };
                let mut read = self.prompt(&session, message);
                read.events
                    .retain(|event| !matches!(event, HarnessEvent::UserMessageChunk { .. }));
                Ok(read)
            }
            None => {
                let session = self.open_session()?;
                self.stopped = false;
                let message = Message {
                    text: text.into(),
                    images: images.to_vec(),
                };
                Ok(if busy {
                    self.steer(&session, message)
                } else {
                    self.prompt(&session, message)
                })
            }
        }
    }

    fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Result<Read, Error> {
        if let Some(Answered { events, picked }) = self.dialogs.answer(id, answer) {
            let mut read = events_only(events);
            if let Picked::Session(Some(session)) = picked {
                let previous = self.session.clone();
                let loaded = self.load(session, previous);
                read.events.extend(loaded.events);
                read.lines.extend(loaded.lines);
            }
            return Ok(read);
        }
        if let Some(pick) = self.settings.pick(id, answer) {
            return Ok(Read {
                events: vec![HarnessEvent::DialogClosed],
                lines: pick
                    .and_then(|pick| self.set(pick, None))
                    .into_iter()
                    .collect(),
                relaunch: None,
            });
        }
        let answered = self.asks.answer(id, answer);
        Ok(Read {
            events: answered.events,
            lines: answered
                .settled
                .map(|(reply, settled)| asks::answer(reply, settled))
                .into_iter()
                .collect(),
            relaunch: None,
        })
    }

    fn pick_model(&mut self, id: &str, level: Option<&str>) -> Read {
        let level = level.map(String::from);
        let line = self.pick_line(id, level.clone());
        if line.is_none() && self.session.is_none() {
            self.wanted = Some((id.into(), level));
        }
        Read {
            lines: line.into_iter().collect(),
            ..Read::default()
        }
    }

    fn interrupt(&mut self) -> Read {
        if self.own_turn && !self.running {
            return events_only(self.quiet());
        }
        let Some(session) = self.session.as_deref().filter(|_| self.running) else {
            return Read::default();
        };
        self.stopped = true;
        self.next.clear();
        let mut lines = vec![requests::cancel(session)];
        lines.extend(self.asks.replies().map(asks::stopped));
        Read {
            events: self.asks.withdraw(|_| true),
            lines,
            relaunch: None,
        }
    }

    fn end(&mut self) -> Vec<HarnessEvent> {
        self.updates.settle();
        self.own_turn = false;
        let mut events = self.asks.withdraw(|_| true);
        events.extend(self.calls.stop());
        events.extend(self.subagents.clear());
        events
    }

    fn quiet_after(&self) -> Option<Duration> {
        let asking = self.asks.replies().next().is_some();
        (self.own_turn && !asking).then(|| {
            if self.updates.calls_open() {
                QUIET_CALL
            } else {
                QUIET
            }
        })
    }

    fn quiet(&mut self) -> Vec<HarnessEvent> {
        if !self.own_turn {
            return vec![];
        }
        self.own_turn = false;
        self.updates.settle();
        let mut events: Vec<HarnessEvent> = self.updates.close().into_iter().collect();
        events.push(HarnessEvent::State { running: false });
        self.calls.track(events)
    }
}
