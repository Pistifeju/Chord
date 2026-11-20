//! The Codex reader: one stdout line in, the harness events it means and the lines to write Codex next out. Holds the pane's thread, the running turn, the requests waiting for an answer, the items whose text streams and Codex's asks.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::asks::{self, Answered, Asks, Parsed};
use super::commands::{Command, Settings};
use super::{items, requests};
use crate::Error;
use crate::harness::calls::OpenCalls;
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent, Image};

const RESUME_DIALOG: &str = "codex-resume";

/// What one record leads to: events for the chat and lines to write Codex.
#[derive(Default)]
pub(super) struct Read {
    pub(super) events: Vec<HarnessEvent>,
    pub(super) lines: Vec<Value>,
}

/// A request the adapter wrote, waiting for its answer.
enum Pending {
    Initialize,
    Models,
    Skills,
    Thread,
    /// A thread to open; for the pane's saved thread, a refusal starts a new one.
    Resume {
        saved: bool,
    },
    /// The worktree's threads, for `/resume`, without those other panes hold.
    Threads {
        held: Vec<String>,
    },
    Turn,
    /// A steer, with its text and input to send again as a turn when the turn ended first.
    Steer {
        text: String,
        input: Vec<Value>,
    },
    Interrupt,
}

/// Reads one Codex process's stdout, line by line.
pub(super) struct Reader {
    cwd: PathBuf,
    /// The pane's saved thread, until the handshake asks Codex to open it.
    saved: Option<String>,
    /// The threads the open resume dialog lists, in its order.
    resumable: Vec<String>,
    thread: Option<String>,
    /// Why Codex started no thread, so a send says it.
    thread_error: Option<String>,
    turn: Option<String>,
    next_id: u64,
    pending: HashMap<u64, Pending>,
    settings: Settings,
    /// The context window of the thread's model, kept for the meter after a compaction.
    window: Option<u32>,
    /// Messages and thoughts whose text came as deltas, so their completion does not repeat it.
    streamed: HashSet<String>,
    /// The steers Codex has not taken in yet, oldest first.
    steers: Vec<String>,
    calls: OpenCalls,
    asks: Asks,
    /// The paths of each running file change by item id, for its approval.
    files: HashMap<String, String>,
}

impl Reader {
    pub(super) fn new(cwd: &Path, saved: Option<&str>) -> Self {
        Reader {
            cwd: cwd.to_path_buf(),
            saved: saved.map(String::from),
            resumable: vec![],
            thread: None,
            thread_error: None,
            turn: None,
            next_id: 0,
            pending: HashMap::new(),
            settings: Settings::default(),
            window: None,
            streamed: HashSet::new(),
            steers: vec![],
            calls: OpenCalls::default(),
            asks: Asks::default(),
            files: HashMap::new(),
        }
    }

    /// The first line Codex hears.
    pub(super) fn initialize(&mut self) -> Value {
        requests::initialize(self.request(Pending::Initialize))
    }

    /// The user's message: a new turn, or a steer of the running one. A session command opens its picker, and a skill's command sends the skill.
    pub(super) fn send(
        &mut self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[String],
    ) -> Result<Read, Error> {
        let Some(thread) = self.thread.clone() else {
            return Err(Error::Message(self.thread_error.clone().unwrap_or_else(
                || "Codex is still starting. Send the message again in a moment".into(),
            )));
        };
        let (text, input) = match self.settings.command(text) {
            Some(Command::Menu(event)) if images.is_empty() => {
                return Ok(Read {
                    events: vec![event],
                    lines: vec![],
                });
            }
            Some(Command::Resume) if images.is_empty() && self.turn.is_none() => {
                let cwd = self.cwd.to_string_lossy().into_owned();
                let held = held_elsewhere.to_vec();
                return Ok(Read {
                    events: vec![],
                    lines: vec![requests::thread_list(
                        self.request(Pending::Threads { held }),
                        &cwd,
                    )],
                });
            }
            Some(Command::Resume) if images.is_empty() => {
                return Err(Error::Message(
                    "Wait for the agent to finish, then change the session".into(),
                ));
            }
            Some(Command::Menu(_) | Command::Resume) => {
                return Err(Error::Message(
                    "Remove the images, then change the session".into(),
                ));
            }
            Some(Command::Skill { text, skill }) => {
                let mut input = requests::input(&text, images);
                input.push(skill);
                (text, input)
            }
            None => (text.to_string(), requests::input(text, images)),
        };
        let text = text.as_str();
        let Some(turn) = self.turn.clone() else {
            return Ok(Read {
                events: vec![],
                lines: vec![self.turn_start(&thread, &input)],
            });
        };
        let line = requests::turn_steer(
            self.request(Pending::Steer {
                text: text.into(),
                input: input.clone(),
            }),
            &thread,
            &turn,
            &input,
        );
        self.steers.push(text.into());
        Ok(Read {
            events: vec![self.queue()],
            lines: vec![line],
        })
    }

    /// The user's answer to an open ask: the line that answers Codex, or a new turn with the answers.
    pub(super) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Result<Read, Error> {
        if id == RESUME_DIALOG {
            let picked = match answer {
                DialogAnswer::Value { value } => value
                    .split_once('.')
                    .and_then(|(number, _)| number.parse::<usize>().ok())
                    .and_then(|number| self.resumable.get(number.checked_sub(1)?).cloned()),
                _ => None,
            };
            self.resumable.clear();
            let cwd = self.cwd.to_string_lossy().into_owned();
            let lines = picked.map(|thread| {
                let id = self.request(Pending::Resume { saved: false });
                requests::thread_resume(id, &thread, &cwd)
            });
            return Ok(Read {
                events: vec![HarnessEvent::DialogClosed],
                lines: lines.into_iter().collect(),
            });
        }
        if let Some(picked) = self.settings.pick(id, answer) {
            let mut events = vec![HarnessEvent::DialogClosed];
            events.extend(picked);
            return Ok(Read {
                events,
                lines: vec![],
            });
        }
        let Answered { events, line, text } = self.asks.answer(id, answer);
        let mut read = Read {
            events,
            lines: line.into_iter().collect(),
        };
        if let Some(text) = text {
            let sent = self.send(&text, &[], &[])?;
            read.events.extend(sent.events);
            read.lines.extend(sent.lines);
        }
        Ok(read)
    }

    /// Stops the running turn; none when no turn runs.
    pub(super) fn interrupt(&mut self) -> Option<Value> {
        let (thread, turn) = (self.thread.clone()?, self.turn.clone()?);
        Some(requests::turn_interrupt(
            self.request(Pending::Interrupt),
            &thread,
            &turn,
        ))
    }

    /// Stdout closed: the calls still open end as stopped.
    pub(super) fn end(&mut self) -> Vec<HarnessEvent> {
        self.calls.stop()
    }

    pub(super) fn feed(&mut self, line: &[u8]) -> Read {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            return Read::default();
        };
        let mut read = match (&record["id"], record["method"].as_str()) {
            (Value::Null, Some("skills/changed")) => Read {
                events: vec![],
                lines: vec![self.skills_list()],
            },
            (Value::Null, Some(method)) => Read {
                events: self.notification(method, &record["params"]),
                lines: vec![],
            },
            (id, Some(method)) => {
                let files = |item: &str| self.files.get(item).cloned();
                match asks::parse(id, method, &record["params"], files) {
                    Parsed::Ask(ask) => Read {
                        events: self.asks.push(ask),
                        lines: vec![],
                    },
                    Parsed::Refused { line, notice } => Read {
                        events: vec![HarnessEvent::Notice { text: notice }],
                        lines: vec![line],
                    },
                }
            }
            (id, None) => match id.as_u64().and_then(|id| self.pending.remove(&id)) {
                Some(pending) => self.response(pending, &record),
                None => Read::default(),
            },
        };
        read.events = self.calls.track(read.events);
        read
    }

    // The worktree's other threads, newest first, as a numbered select: each one's first line and age.
    fn resume_dialog(&mut self, result: &Value, held: &[String]) -> Read {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let threads: Vec<(String, String)> = result["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|thread| {
                let id = thread["id"].as_str()?;
                let preview = thread["preview"]
                    .as_str()?
                    .lines()
                    .find(|line| !line.trim().is_empty())?;
                let free =
                    Some(id) != self.thread.as_deref() && !held.iter().any(|other| other == id);
                let age = age(now.saturating_sub(thread["updatedAt"].as_u64().unwrap_or(now)));
                free.then(|| (id.into(), format!("{} · {age}", preview.trim())))
            })
            .collect();
        if threads.is_empty() {
            return Read {
                events: vec![HarnessEvent::Notice {
                    text: "This worktree has no other session to resume".into(),
                }],
                lines: vec![],
            };
        }
        let options = threads
            .iter()
            .enumerate()
            .map(|(at, (_, label))| format!("{}. {label}", at + 1))
            .collect();
        self.resumable = threads.into_iter().map(|(id, _)| id).collect();
        Read {
            events: vec![HarnessEvent::Dialog {
                id: RESUME_DIALOG.into(),
                title: "Resume Session".into(),
                ask: DialogAsk::Select {
                    options,
                    current: None,
                    detail: None,
                    descriptions: vec![],
                    multi: false,
                },
            }],
            lines: vec![],
        }
    }

    fn skills_list(&mut self) -> Value {
        let cwd = self.cwd.to_string_lossy().into_owned();
        requests::skills_list(self.request(Pending::Skills), &cwd)
    }

    fn turn_start(&mut self, thread: &str, input: &[Value]) -> Value {
        let id = self.request(Pending::Turn);
        requests::turn_start(id, thread, input, self.settings.picks())
    }

    fn thread_start(&mut self) -> Value {
        let cwd = self.cwd.to_string_lossy().into_owned();
        requests::thread_start(self.request(Pending::Thread), &cwd)
    }

    fn request(&mut self, pending: Pending) -> u64 {
        self.next_id += 1;
        self.pending.insert(self.next_id, pending);
        self.next_id
    }

    fn queue(&self) -> HarnessEvent {
        HarnessEvent::Queue {
            steering: self.steers.clone(),
        }
    }

    fn response(&mut self, pending: Pending, record: &Value) -> Read {
        let result = &record["result"];
        let error = record["error"]["message"].as_str().map(String::from);
        let notice = |text: String| Read {
            events: vec![HarnessEvent::Notice { text }],
            lines: vec![],
        };
        match pending {
            Pending::Initialize => match error {
                Some(error) => notice(error),
                None => Read {
                    events: vec![],
                    lines: vec![
                        requests::initialized(),
                        self.skills_list(),
                        requests::model_list(self.request(Pending::Models)),
                    ],
                },
            },
            // Without the list the model shows its id; the thread starts either way.
            Pending::Models => {
                self.settings.read_models(result);
                let line = match self.saved.take() {
                    Some(thread) => {
                        let cwd = self.cwd.to_string_lossy().into_owned();
                        requests::thread_resume(
                            self.request(Pending::Resume { saved: true }),
                            &thread,
                            &cwd,
                        )
                    }
                    None => self.thread_start(),
                };
                Read {
                    events: vec![],
                    lines: vec![line],
                }
            }
            // Without the list the menu holds the session commands alone.
            Pending::Skills => Read {
                events: vec![self.settings.read_skills(result)],
                lines: vec![],
            },
            // A thread that never ran a turn may have no file to open, so a refusal starts a new one.
            Pending::Resume { saved: true } if error.is_some() => Read {
                events: vec![],
                lines: vec![self.thread_start()],
            },
            // The pane stays on its thread when Codex cannot open the picked one.
            Pending::Resume { saved: false } if error.is_some() => {
                notice(error.unwrap_or_default())
            }
            Pending::Threads { held } => match error {
                Some(error) => notice(error),
                None => self.resume_dialog(result, &held),
            },
            Pending::Thread | Pending::Resume { .. } => {
                let mut events = vec![HarnessEvent::HistoryStart];
                match (error, result["thread"]["id"].as_str()) {
                    (None, Some(thread)) => {
                        self.thread = Some(thread.into());
                        events.push(HarnessEvent::SessionInfo { id: thread.into() });
                        events.extend(self.settings.opened(result));
                        let turns = result["thread"]["turns"].as_array();
                        events.extend(
                            turns
                                .into_iter()
                                .flatten()
                                .flat_map(|turn| items::turn_history(turn, &self.cwd)),
                        );
                    }
                    (error, _) => {
                        let error = error.unwrap_or_else(|| "Codex started no thread".into());
                        self.thread_error = Some(error.clone());
                        events.push(HarnessEvent::Notice { text: error });
                    }
                }
                // The chat waits for a history, even an empty one, before it takes a message.
                events.push(HarnessEvent::HistoryEnd);
                Read {
                    events,
                    lines: vec![],
                }
            }
            Pending::Turn => {
                if let Some(error) = error {
                    return notice(error);
                }
                if self.turn.is_none() {
                    self.turn = result["turn"]["id"].as_str().map(String::from);
                }
                Read::default()
            }
            Pending::Steer { text, input } => {
                let Some(error) = error else {
                    return Read::default();
                };
                if let Some(at) = self.steers.iter().position(|steer| *steer == text) {
                    self.steers.remove(at);
                }
                let mut events = vec![self.queue()];
                // The turn ended before the steer reached it, so the message starts the next one.
                let lines = match (&self.turn, &self.thread) {
                    (None, Some(thread)) => {
                        let thread = thread.clone();
                        vec![self.turn_start(&thread, &input)]
                    }
                    _ => {
                        events.push(HarnessEvent::Notice { text: error });
                        vec![]
                    }
                };
                Read { events, lines }
            }
            Pending::Interrupt => error.map_or_else(Read::default, notice),
        }
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<HarnessEvent> {
        // A request id is the process's own, so a withdrawn ask is found whichever thread asked.
        if method == "serverRequest/resolved" {
            return self.asks.resolved(&params["requestId"]);
        }
        // A subagent's thread and a thread the pane left speak on the same process.
        if self.thread.is_none() || params["threadId"].as_str() != self.thread.as_deref() {
            return vec![];
        }
        let item = &params["item"];
        let item_id = || {
            params["itemId"]
                .as_str()
                .or(item["id"].as_str())
                .unwrap_or_default()
                .to_string()
        };
        match method {
            "turn/started" => {
                self.turn = params["turn"]["id"].as_str().map(String::from);
                vec![HarnessEvent::State { running: true }]
            }
            "item/started" => {
                let events = items::started(item, params["startedAtMs"].as_f64(), &self.cwd);
                if let (Some("fileChange"), Some(HarnessEvent::ToolStart { detail, .. })) =
                    (item["type"].as_str(), events.first())
                {
                    self.files.insert(item_id(), detail.clone());
                }
                if item["type"] == "userMessage" {
                    let text = items::user_text(item);
                    if let Some(at) = self.steers.iter().position(|steer| *steer == text) {
                        self.steers.remove(at);
                        return events.into_iter().chain([self.queue()]).collect();
                    }
                }
                events
            }
            "item/agentMessage/delta" => {
                self.streamed.insert(item_id());
                vec![HarnessEvent::MessageChunk {
                    text: params["delta"].as_str().unwrap_or_default().into(),
                }]
            }
            // A thought starts at its first text, since Codex often reasons without a summary to show.
            "item/reasoning/summaryTextDelta" | "item/reasoning/textDelta" => {
                let text = params["delta"].as_str().unwrap_or_default().to_string();
                let start = self
                    .streamed
                    .insert(item_id())
                    .then_some(HarnessEvent::ThoughtStart);
                start
                    .into_iter()
                    .chain([HarnessEvent::ThoughtChunk { text }])
                    .collect()
            }
            "item/reasoning/summaryPartAdded" if self.streamed.contains(&item_id()) => {
                vec![HarnessEvent::ThoughtChunk {
                    text: "\n\n".into(),
                }]
            }
            "item/completed" => {
                let streamed = self.streamed.remove(&item_id());
                self.files.remove(&item_id());
                let mut events = items::completed(item, params["completedAtMs"].as_f64(), streamed);
                if let Some(ask) = asks::message_questions(item) {
                    events.extend(self.asks.push(ask));
                }
                if item["type"] == "contextCompaction" {
                    events.push(HarnessEvent::Context {
                        tokens: None,
                        window: self.window,
                    });
                }
                events
            }
            "thread/tokenUsage/updated" => {
                let usage = &params["tokenUsage"];
                let count = |value: &Value| value.as_u64().and_then(|n| u32::try_from(n).ok());
                self.window = count(&usage["modelContextWindow"]);
                vec![HarnessEvent::Context {
                    tokens: count(&usage["last"]["totalTokens"]),
                    window: self.window,
                }]
            }
            "thread/settings/updated" => self.settings.updated(&params["threadSettings"]),
            "error" if params["willRetry"] != true => vec![HarnessEvent::Notice {
                text: params["error"]["message"]
                    .as_str()
                    .unwrap_or("Codex reported an error")
                    .into(),
            }],
            "turn/completed" => self.turn_completed(&params["turn"]),
            _ => vec![],
        }
    }

    fn turn_completed(&mut self, turn: &Value) -> Vec<HarnessEvent> {
        self.turn = None;
        self.streamed.clear();
        self.files.clear();
        let mut events = self.asks.turn_ended();
        events.extend(self.calls.stop());
        if turn["status"] == "failed" {
            events.push(HarnessEvent::MessageEnd {
                failed: true,
                at: None,
            });
            events.push(HarnessEvent::Notice {
                text: turn["error"]["message"]
                    .as_str()
                    .unwrap_or("The turn failed")
                    .into(),
            });
        }
        if !self.steers.is_empty() {
            self.steers.clear();
            events.push(self.queue());
        }
        events.push(HarnessEvent::State { running: false });
        if turn["status"] == "interrupted" {
            events.push(HarnessEvent::Aborted);
        }
        events
    }
}

// How long ago, as the sidebar says it: "now", "5m", "3h", "2d".
fn age(seconds: u64) -> String {
    match seconds / 60 {
        0 => "now".into(),
        minutes @ 1..60 => format!("{minutes}m"),
        minutes @ 60..1440 => format!("{}h", minutes / 60),
        minutes => format!("{}d", minutes / 1440),
    }
}
