use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value};

use super::asks::{self, Answered, Asks, Parsed};
use super::commands::{Command, Settings};
use super::{NAME, items, requests, usage};
use crate::Error;
use crate::harness::calls::OpenCalls;
use crate::harness::earlier::{Back, Message};
use crate::harness::relaunch;
use crate::harness::resume::Session;
use crate::harness::session_dialogs::{self, Picked, SessionDialogs};
use crate::harness::subagents::Subagents;
use crate::harness::{DialogAnswer, HarnessEvent, Image, SubagentState};

#[derive(Default)]
pub(super) struct Read {
    pub(super) events: Vec<HarnessEvent>,
    pub(super) lines: Vec<Value>,
}

#[derive(Default)]
pub(super) struct Relaunch {
    pub(super) thread: Option<String>,
    pub(super) picks: Map<String, Value>,
}

enum Pending {
    Initialize,
    Models,
    Skills,
    Thread,
    Resume { saved: bool },
    Threads { held: Vec<String> },
    Turn,
    Fork,
    Turns { back: Back },
    Revert,
    Compact,
    Steer { text: String, input: Vec<Value> },
    Interrupt,
    RateLimits,
}

pub(super) struct Reader {
    cwd: PathBuf,
    opening: Map<String, Value>,
    saved: Option<String>,
    dialogs: SessionDialogs,
    thread: Option<String>,
    thread_error: Option<String>,
    turn: Option<String>,
    next_id: u64,
    pending: HashMap<u64, Pending>,
    settings: Settings,
    streamed: HashSet<String>,
    steers: Vec<String>,
    calls: OpenCalls,
    asks: Asks,
    files: HashMap<String, String>,
    subagents: Subagents,
}

impl Reader {
    pub(super) fn new(
        cwd: &Path,
        saved: Option<&str>,
        picks: Map<String, Value>,
        instructions: Option<String>,
    ) -> Self {
        Reader {
            cwd: cwd.to_path_buf(),
            opening: requests::opening(&cwd.to_string_lossy(), instructions),
            saved: saved.map(String::from),
            dialogs: SessionDialogs::default(),
            thread: None,
            thread_error: None,
            turn: None,
            next_id: 0,
            pending: HashMap::new(),
            settings: Settings::with_picks(picks),
            streamed: HashSet::new(),
            steers: vec![],
            calls: OpenCalls::default(),
            asks: Asks::default(),
            files: HashMap::new(),
            subagents: Subagents::default(),
        }
    }

    pub(super) fn initialize(&mut self) -> Value {
        requests::initialize(self.request(Pending::Initialize))
    }

    pub(super) fn relaunch(&self, text: &str) -> Option<Relaunch> {
        match self.settings.command(text) {
            Some(Command::New) => Some(Relaunch::default()),
            Some(Command::Reload) => Some(Relaunch {
                thread: self.thread.clone().or_else(|| self.saved.clone()),
                picks: self.settings.picks().clone(),
            }),
            _ => None,
        }
    }

    pub(super) fn takes_images(&self, text: &str) -> bool {
        matches!(
            self.settings.command(text),
            None | Some(Command::Skill { .. })
        )
    }

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
            Some(Command::Menu(event)) => {
                return Ok(Read {
                    events: vec![event],
                    lines: vec![],
                });
            }
            Some(Command::Resume | Command::Back(_) | Command::Clone | Command::Compact)
                if self.turn.is_some() =>
            {
                return Err(Error::Message(
                    "Wait for the agent to finish, then change the session".into(),
                ));
            }
            Some(Command::Resume) => {
                let cwd = self.cwd.to_string_lossy().into_owned();
                let held = held_elsewhere.to_vec();
                let id = self.request(Pending::Threads { held });
                return Ok(lines(requests::thread_list(id, &cwd)));
            }
            Some(Command::Back(back)) => {
                let id = self.request(Pending::Turns { back });
                return Ok(lines(requests::thread_turns(id, &thread)));
            }
            Some(Command::Clone) => {
                let id = self.request(Pending::Fork);
                return Ok(lines(requests::thread_fork(
                    id,
                    &thread,
                    None,
                    &self.opening,
                )));
            }
            Some(Command::Compact) => {
                let id = self.request(Pending::Compact);
                return Ok(lines(requests::thread_compact(id, &thread)));
            }
            Some(Command::New | Command::Reload) => return Ok(Read::default()),
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

    pub(super) fn pick_model(&mut self, id: &str, level: Option<&str>) -> Read {
        Read {
            events: vec![self.settings.choose(id, level)],
            lines: vec![],
        }
    }

    pub(super) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Result<Read, Error> {
        if let Some(session_dialogs::Answered { events, picked }) = self.dialogs.answer(id, answer)
        {
            let line = match picked {
                Picked::Session(thread) => thread.map(|thread| {
                    let id = self.request(Pending::Resume { saved: false });
                    requests::thread_resume(id, &thread, &self.opening)
                }),
                Picked::Message(picked) => picked
                    .zip(self.thread.clone())
                    .map(|((back, message), thread)| self.go_back(back, message, &thread)),
            };
            return Ok(Read {
                events,
                lines: line.into_iter().collect(),
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
        let Answered { events, line, text } = asks::answer(&mut self.asks, id, answer);
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

    pub(super) fn interrupt(&mut self) -> Option<Value> {
        let (thread, turn) = (self.thread.clone()?, self.turn.clone()?);
        Some(requests::turn_interrupt(
            self.request(Pending::Interrupt),
            &thread,
            &turn,
        ))
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
        let mut read = match (&record["id"], record["method"].as_str()) {
            (Value::Null, Some("skills/changed")) => Read {
                events: vec![],
                lines: vec![self.skills_list()],
            },
            (Value::Null, Some("account/rateLimits/updated")) => lines(self.rate_limits()),
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

    fn resume_dialog(&mut self, result: &Value, held: &[String]) -> Read {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let threads = result["data"]
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
                let seconds = now.saturating_sub(thread["updatedAt"].as_u64().unwrap_or(now));
                free.then(|| Session {
                    id: id.into(),
                    label: preview.trim().into(),
                    age: seconds,
                })
            })
            .collect();
        Read {
            events: vec![self.dialogs.offer_sessions(threads)],
            lines: vec![],
        }
    }

    fn go_back(&mut self, back: Back, message: Message, thread: &str) -> Value {
        match (back, message.before) {
            (Back::Fork, Some(before)) => {
                let id = self.request(Pending::Fork);
                requests::thread_fork(id, thread, Some(&before), &self.opening)
            }
            (Back::Fork, None) => {
                requests::thread_start(self.request(Pending::Fork), &self.opening)
            }
            (Back::Rewind, _) => {
                requests::thread_revert(self.request(Pending::Revert), thread, &message.id)
            }
        }
    }

    fn earlier_dialog(&mut self, back: Back, result: &Value) -> Read {
        let turns: Vec<&Value> = result["data"]
            .as_array()
            .into_iter()
            .flatten()
            .rev()
            .collect();
        let messages = turns
            .iter()
            .enumerate()
            .filter_map(|(at, turn)| {
                let item = turn["items"]
                    .as_array()?
                    .iter()
                    .find(|item| item["type"] == "userMessage")?;
                Some(Message {
                    id: turn["id"].as_str()?.into(),
                    before: at
                        .checked_sub(1)
                        .and_then(|before| turns[before]["id"].as_str())
                        .map(String::from),
                    text: items::user_text(item),
                })
            })
            .collect();
        Read {
            events: vec![self.dialogs.offer_messages(back, messages)],
            lines: vec![],
        }
    }

    fn rate_limits(&mut self) -> Value {
        requests::rate_limits(self.request(Pending::RateLimits))
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
        requests::thread_start(self.request(Pending::Thread), &self.opening)
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
                Some(error) => {
                    self.thread_error = Some(error.clone());
                    Read {
                        events: vec![
                            HarnessEvent::Notice { text: error },
                            HarnessEvent::HistoryEnd,
                        ],
                        lines: vec![],
                    }
                }
                None => Read {
                    events: vec![],
                    lines: vec![
                        requests::initialized(),
                        self.skills_list(),
                        requests::model_list(self.request(Pending::Models)),
                        self.rate_limits(),
                    ],
                },
            },
            Pending::Models => {
                let models = self.settings.read_models(result);
                let line = match self.saved.take() {
                    Some(thread) => {
                        let id = self.request(Pending::Resume { saved: true });
                        requests::thread_resume(id, &thread, &self.opening)
                    }
                    None => self.thread_start(),
                };
                Read {
                    events: vec![models],
                    lines: vec![line],
                }
            }
            Pending::Skills => Read {
                events: vec![self.settings.read_skills(result)],
                lines: vec![],
            },
            Pending::Resume { saved: true } if error.is_some() => Read {
                events: vec![],
                lines: vec![self.thread_start()],
            },
            Pending::Resume { saved: false } | Pending::Fork if error.is_some() => {
                notice(error.unwrap_or_default())
            }
            Pending::Turns { back } => match error {
                Some(error) => notice(error),
                None => self.earlier_dialog(back, result),
            },
            Pending::Revert => match (error, self.thread.clone()) {
                (None, Some(thread)) => {
                    let id = self.request(Pending::Resume { saved: false });
                    lines(requests::thread_resume(id, &thread, &self.opening))
                }
                (error, _) => notice(error.unwrap_or_default()),
            },
            Pending::Compact => error.map_or_else(Read::default, notice),
            Pending::Threads { held } => match error {
                Some(error) => notice(error),
                None => self.resume_dialog(result, &held),
            },
            Pending::Thread | Pending::Fork | Pending::Resume { .. } => {
                let mut events = vec![HarnessEvent::HistoryStart];
                match (error, result["thread"]["id"].as_str()) {
                    (None, Some(thread)) => {
                        events.extend(self.subagents.clear());
                        self.thread = Some(thread.into());
                        events.push(HarnessEvent::SessionInfo { id: thread.into() });
                        events.extend(self.settings.opened(result));
                        let turns = result["thread"]["turns"].as_array();
                        if turns.is_none_or(|turns| turns.is_empty()) {
                            events.push(HarnessEvent::Context {
                                tokens: Some(0),
                                window: None,
                                estimated: false,
                            });
                        }
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
            Pending::RateLimits => Read {
                events: vec![HarnessEvent::Usage {
                    harness: NAME,
                    windows: usage::windows(&result["rateLimits"]),
                }],
                lines: vec![],
            },
        }
    }

    fn notification(&mut self, method: &str, params: &Value) -> Vec<HarnessEvent> {
        if method == "serverRequest/resolved" {
            let request = &params["requestId"];
            return self
                .asks
                .withdraw(|reply| reply.request.as_ref() == Some(request));
        }
        if let Some(events) = self.subagent(method, params) {
            return events;
        }
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
                if item["type"] == "contextCompaction" {
                    return events
                        .into_iter()
                        .chain([HarnessEvent::Compacting { active: true }])
                        .collect();
                }
                events
            }
            "item/agentMessage/delta" => {
                self.streamed.insert(item_id());
                vec![HarnessEvent::MessageChunk {
                    text: params["delta"].as_str().unwrap_or_default().into(),
                }]
            }
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
                let thread = self.thread.clone().unwrap_or_default();
                events.extend(self.activity(item, &thread));
                if let Some(ask) = asks::message_questions(item) {
                    events.extend(self.asks.push(ask));
                }
                if item["type"] == "contextCompaction" {
                    events.push(HarnessEvent::Compacting { active: false });
                }
                events
            }
            "thread/tokenUsage/updated" => {
                let usage = &params["tokenUsage"];
                let count = |value: &Value| value.as_u64().and_then(|n| u32::try_from(n).ok());
                vec![HarnessEvent::Context {
                    tokens: count(&usage["last"]["totalTokens"]),
                    window: count(&usage["modelContextWindow"]),
                    estimated: false,
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

    fn subagent(&mut self, method: &str, params: &Value) -> Option<Vec<HarnessEvent>> {
        let id = params["threadId"].as_str()?;
        if !self.subagents.knows(id) {
            return None;
        }
        let event = match method {
            "turn/started" => Some(self.subagents.start(id, None, None)),
            "item/started" => items::started(&params["item"], None, &self.cwd)
                .into_iter()
                .find_map(|event| match event {
                    HarnessEvent::ToolStart { title, .. } => Some(title),
                    _ => None,
                })
                .and_then(|title| self.subagents.tool(id, &title)),
            "item/completed" => self.activity(&params["item"], id),
            "turn/completed" => {
                let state = match params["turn"]["status"].as_str() {
                    Some("failed" | "interrupted") => SubagentState::Error,
                    _ => SubagentState::Done,
                };
                self.subagents.end(id, state)
            }
            _ => None,
        };
        Some(event.into_iter().collect())
    }

    fn activity(&mut self, item: &Value, thread: &str) -> Option<HarnessEvent> {
        if item["type"] != "subAgentActivity" {
            return None;
        }
        let id = item["agentThreadId"].as_str()?;
        match item["kind"].as_str()? {
            "started" | "interacted" => {
                let name = item["agentPath"]
                    .as_str()
                    .and_then(|path| path.rsplit('/').find(|part| !part.is_empty()));
                Some(self.subagents.start(id, Some(thread), name))
            }
            "completed" => self.subagents.end(id, SubagentState::Done),
            "interrupted" => self.subagents.end(id, SubagentState::Error),
            _ => None,
        }
    }

    fn turn_completed(&mut self, turn: &Value) -> Vec<HarnessEvent> {
        self.turn = None;
        self.streamed.clear();
        self.files.clear();
        let mut events = self.asks.withdraw(|reply| reply.request.is_some());
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

fn lines(line: Value) -> Read {
    Read {
        events: vec![],
        lines: vec![line],
    }
}

impl From<Read> for relaunch::Read<Relaunch> {
    fn from(Read { events, lines }: Read) -> Self {
        relaunch::Read {
            events,
            lines,
            relaunch: None,
        }
    }
}

impl relaunch::Reader for Reader {
    type Open = Relaunch;

    fn initialize(&mut self) -> Value {
        Reader::initialize(self)
    }

    fn feed(&mut self, line: &[u8]) -> relaunch::Read<Relaunch> {
        Reader::feed(self, line).into()
    }

    fn takes_images(&self, text: &str) -> bool {
        Reader::takes_images(self, text)
    }

    fn send(
        &mut self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[String],
    ) -> Result<relaunch::Read<Relaunch>, Error> {
        if let Some(open) = self.relaunch(text) {
            return Ok(relaunch::Read {
                relaunch: Some(open),
                ..relaunch::Read::default()
            });
        }
        Reader::send(self, text, images, held_elsewhere).map(Into::into)
    }

    fn answer(
        &mut self,
        id: &str,
        answer: &DialogAnswer,
    ) -> Result<relaunch::Read<Relaunch>, Error> {
        Reader::answer(self, id, answer).map(Into::into)
    }

    fn pick_model(&mut self, id: &str, level: Option<&str>) -> relaunch::Read<Relaunch> {
        Reader::pick_model(self, id, level).into()
    }

    fn interrupt(&mut self) -> relaunch::Read<Relaunch> {
        Reader::interrupt(self)
            .map_or_else(Read::default, lines)
            .into()
    }

    fn end(&mut self) -> Vec<HarnessEvent> {
        Reader::end(self)
    }
}
