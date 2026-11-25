//! The OpenCode reader: one stdout line in, the harness events it means and the lines to write OpenCode next out. Holds the session, whether a prompt runs, the requests waiting for an answer, OpenCode's asks, the session's settings and what the session's updates left open.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::asks::{self, Asks, Parsed};
use super::commands::{Command, Pick, Settings};
use super::updates::Updates;
use super::{HARNESS, requests};
use crate::Error;
use crate::harness::calls::OpenCalls;
use crate::harness::relaunch;
use crate::harness::{DialogAnswer, HarnessEvent, Image};

/// A message of the user's, as a prompt or a steer carries it.
struct Message {
    text: String,
    images: Vec<Image>,
}

type Read = relaunch::Read<()>;

/// ACP's error code for a request that needs a sign-in.
const AUTH_REQUIRED: i64 = -32000;

/// A request the adapter wrote, waiting for its answer.
enum Pending {
    Initialize,
    Session,
    Prompt,
    /// A config pick, with the thinking level to pick once the model's answer lists it.
    Config(Option<String>),
    Fork,
    /// The session that reads each model's thinking levels, which OpenCode reports for a session's model alone.
    ProbeSession,
    /// The probe session switched to this model.
    Probe(String),
    /// A steer, with its message to send again as a prompt when the turn ended first.
    Steer(Message),
}

/// Reads one OpenCode process's stdout, line by line.
pub(super) struct Reader {
    cwd: PathBuf,
    next_id: u64,
    pending: HashMap<u64, Pending>,
    session: Option<String>,
    /// Why OpenCode opened no session, so a send says it.
    session_error: Option<String>,
    /// A `session/prompt` waits for its answer.
    running: bool,
    /// The steers the turn's end beat while the prompt's answer was still on its way, oldest first: the first goes out as the next prompt, the rest steer it.
    next: Vec<Message>,
    /// The user stopped the run and sent nothing since, so a steer the turn's end beat goes nowhere.
    stopped: bool,
    updates: Updates,
    settings: Settings,
    /// The probe session and the models it has yet to pick, the next last.
    probe: Option<(String, Vec<String>)>,
    calls: OpenCalls,
    asks: Asks,
}

impl Reader {
    pub(super) fn new(cwd: &Path) -> Self {
        Reader {
            cwd: cwd.to_path_buf(),
            next_id: 0,
            pending: HashMap::new(),
            session: None,
            session_error: None,
            running: false,
            next: vec![],
            stopped: false,
            updates: Updates::new(cwd),
            settings: Settings::default(),
            probe: None,
            calls: OpenCalls::default(),
            asks: Asks::default(),
        }
    }

    fn request(&mut self, pending: Pending) -> u64 {
        self.next_id += 1;
        self.pending.insert(self.next_id, pending);
        self.next_id
    }

    // ACP has no record of a turn's start, and OpenCode echoes no live message, so both go out with the prompt.
    fn prompt(&mut self, session: &str, message: Message) -> Read {
        self.running = true;
        let id = self.request(Pending::Prompt);
        let line = requests::prompt(id, session, &message.text, &message.images);
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
                    None => {
                        let id = self.request(Pending::Session);
                        lines(requests::session_new(id, &self.cwd.to_string_lossy()))
                    }
                }
            }
            Pending::Session => match (error, result["sessionId"].as_str()) {
                (None, Some(session)) => {
                    self.session = Some(session.into());
                    let mut events = vec![HarnessEvent::SessionInfo { id: session.into() }];
                    events.extend(self.settings.read(&result["configOptions"]));
                    events.push(self.settings.commands_event());
                    // The chat takes a message once a history ends, an empty one too.
                    events.extend([HarnessEvent::HistoryStart, HarnessEvent::HistoryEnd]);
                    let id = self.request(Pending::ProbeSession);
                    let cwd = self.cwd.to_string_lossy().into_owned();
                    Read {
                        events,
                        lines: vec![requests::session_new(id, &cwd)],
                        relaunch: None,
                    }
                }
                (error, _) => {
                    self.no_session(error.unwrap_or_else(|| "OpenCode opened no session".into()))
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
                    self.session = Some(session.into());
                    let mut events = vec![HarnessEvent::SessionInfo { id: session.into() }];
                    events.extend(self.settings.read(&result["configOptions"]));
                    events_only(events)
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

    // The chat waits for a history before it takes a message, so a failed start ends an empty one.
    fn no_session(&mut self, error: String) -> Read {
        self.session_error = Some(error.clone());
        events_only(vec![
            HarnessEvent::Notice { text: error },
            HarnessEvent::HistoryEnd,
        ])
    }

    fn prompt_answered(&mut self, record: &Value) -> Read {
        self.running = false;
        // `calls` ends the calls the turn left open.
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

    // OpenCode answers picks sent at once with whichever model the session holds then, so they go one at a time.
    /// The probe session's next model pick; once none is left, the probe session's delete.
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
        // Nobody waits for the delete's answer.
        self.next_id += 1;
        lines(requests::session_delete(self.next_id, &probe))
    }

    /// The line that sets a config option on the session; none before a session opens.
    fn set(&mut self, (config, value): Pick, level: Option<String>) -> Option<Value> {
        let session = self.session.clone()?;
        let id = self.request(Pending::Config(level));
        Some(requests::set_config_option(id, &session, &config, &value))
    }

    fn steer(&mut self, session: &str, message: Message) -> Read {
        self.next_id += 1;
        let line = requests::steer(self.next_id, session, &message.text, &message.images);
        self.pending.insert(self.next_id, Pending::Steer(message));
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
    type Open = ();

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
                // A session the pane left may still speak.
                if self.session.is_none() || params["sessionId"].as_str() != self.session.as_deref()
                {
                    return Read::default();
                }
                events_only(self.updates.update(&params["update"], &mut self.settings))
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

    /// A session command leaves the images for the next message.
    fn takes_images(&self, text: &str) -> bool {
        self.settings.command(text).is_none()
    }

    /// A new prompt, or a steer of the running one. A session command opens its picker or acts on the session.
    fn send(
        &mut self,
        text: &str,
        images: &[Image],
        _held_elsewhere: &[String],
    ) -> Result<Read, Error> {
        let command = self.settings.command(text);
        // A new process needs no session of the old one's.
        if let Some(Command::New) = command {
            return Ok(Read {
                relaunch: Some(()),
                ..Read::default()
            });
        }
        let Some(session) = self.session.clone() else {
            return Err(Error::Message(self.session_error.clone().unwrap_or_else(
                || "OpenCode is still starting. Send the message again in a moment".into(),
            )));
        };
        match command {
            Some(Command::Menu(event)) => return Ok(events_only(vec![event])),
            Some(Command::Clone | Command::Compact) if self.running => {
                return Err(Error::Message(
                    "Wait for the agent to finish, then change the session".into(),
                ));
            }
            Some(Command::Clone) => {
                let id = self.request(Pending::Fork);
                let cwd = self.cwd.to_string_lossy().into_owned();
                return Ok(lines(requests::session_fork(id, &session, &cwd)));
            }
            // The compaction shows itself, as under the other harnesses, so the command stays out of the transcript.
            Some(Command::Compact) => {
                let mut read = self.prompt(
                    &session,
                    Message {
                        text: text.into(),
                        images: vec![],
                    },
                );
                read.events
                    .retain(|event| !matches!(event, HarnessEvent::UserMessageChunk { .. }));
                return Ok(read);
            }
            Some(Command::New) | None => {}
        }
        self.stopped = false;
        let message = Message {
            text: text.into(),
            images: images.to_vec(),
        };
        Ok(if self.running {
            self.steer(&session, message)
        } else {
            self.prompt(&session, message)
        })
    }

    fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Result<Read, Error> {
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

    // OpenCode lists a model's levels only once the model is the session's, so the level waits for the model's answer.
    fn pick_model(&mut self, id: &str, level: Option<&str>) -> Read {
        let line = self
            .settings
            .model_pick(id)
            .and_then(|pick| self.set(pick, level.map(String::from)));
        Read {
            lines: line.into_iter().collect(),
            ..Read::default()
        }
    }

    // ACP has the client answer each ask still open with `cancelled` after the cancel.
    fn interrupt(&mut self) -> Read {
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
        let mut events = self.asks.withdraw(|_| true);
        events.extend(self.calls.stop());
        events
    }
}
