//! What a harness tells the chat: the events every adapter emits.

pub mod agent_tools;
mod asks;
mod calls;
pub mod claude;
pub mod codex;
mod earlier;
pub mod installed;
pub mod models;
mod numbered;
pub mod pi;
mod process;
mod relaunch;
mod resume;
mod session_commands;
mod session_dialogs;
pub mod sessions;
mod shell;
pub mod spend;
mod subagents;

pub use process::login_shell;

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

use crate::Error;

/// One harness Chord knows: what it is called, how to get it, and the oldest version Chord was tested with.
pub struct Harness {
    /// The name a pane saves.
    pub name: &'static str,
    pub title: &'static str,
    /// Found on the login shell's PATH.
    pub binary: &'static str,
    pub install: &'static str,
    /// None for a harness without one sign-in for all its providers.
    pub sign_in: Option<SignIn>,
    pub upgrade: &'static str,
    pub oldest_tested: (u32, u32, u32),
    /// Chord has its adapter, so `start` runs it.
    pub starts: bool,
}

pub struct SignIn {
    /// The arguments of the command that exits with 0 when the user is signed in.
    pub check: &'static [&'static str],
    pub step: &'static str,
}

/// Every harness, in the order a new pane's default takes them.
pub const HARNESSES: [Harness; 3] = [pi::HARNESS, codex::HARNESS, claude::HARNESS];

/// One running agent of a pane. Each harness adapter implements it in `harness/<name>/`, emits `HarnessEvent`s and keeps its own records private; `start` names it. Dropping the agent ends its process and returns once it has ended.
pub trait Agent: Send {
    /// What this harness can do; the UI hides a control whose capability is missing.
    fn capabilities(&self) -> Vec<Capability>;
    /// Sends the user's message, or runs it as a session command. `held_elsewhere` are the sessions other panes hold, which a resume must not open.
    /// Returns whether the message took the images; a session command leaves them for the next message.
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<bool, Error>;
    /// Answers the dialog the agent opened with `id`.
    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error>;
    /// Switches to the model with `id` from its `Models` list and, when given, to one of its thinking levels; without one the harness keeps or clamps its own.
    fn pick_model(&self, id: &str, level: Option<&str>) -> Result<(), Error>;
    /// Stops the run; the agent keeps running.
    fn abort(&self) -> Result<(), Error>;
    /// Ends the agent and reports it as exited, for a pane that stays.
    fn end_reported(self: Box<Self>);
}

/// Starts the agent of the harness named `harness` in `dir`. `session` is what the harness reported in `SessionInfo`, to open it again; `data_dir` is a folder the adapter may write its own files to. `tools` is the socket of the build and run tools, for an agent that gets them. `on_event` hears every event, from the adapter's own thread.
pub fn start(
    harness: &str,
    dir: &Path,
    session: Option<&str>,
    data_dir: &Path,
    tools: Option<&Path>,
    on_event: impl Fn(HarnessEvent) + Send + 'static,
) -> Result<Box<dyn Agent>, Error> {
    match harness {
        pi::NAME => Ok(Box::new(pi::Session::start(
            dir, session, data_dir, tools, on_event,
        )?)),
        codex::NAME => codex::start(dir, session, tools, on_event),
        claude::NAME => claude::start(dir, session, tools, on_event),
        other => Err(Error::Message(format!(
            "Chord cannot run the harness \"{other}\""
        ))),
    }
}

/// What a harness can do; the adapter returns its list when a session starts, and the UI hides a control whose capability is missing.
#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Streaming replies, a composer and stopping a run.
    Chat,
    /// Images sent with a message.
    Attachments,
    /// A message sent while the agent runs steers the run.
    Steer,
    /// The model and the thinking level can be read and picked.
    Models,
    /// The size of the model's context window and how much of it the session fills.
    Context,
    /// Background subagent runs, reported as a lineage.
    Subagents,
}

/// One subscription usage window: its label, e.g. "5h 42%", and when it resets, in ms since the epoch; none when the harness does not say.
#[derive(Serialize, Type, Debug, Clone, PartialEq)]
pub struct UsageWindow {
    pub label: String,
    pub resets_at: Option<f64>,
}

impl UsageWindow {
    /// The window named `name`, e.g. "5h", with `percent` of it used.
    pub(crate) fn used(name: &str, percent: f64, resets_at: Option<f64>) -> Self {
        UsageWindow {
            label: format!("{name} {}%", percent.round()),
            resets_at,
        }
    }
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubagentState {
    Running,
    Done,
    Error,
}

/// One row of the subagent lineage: a background run, or a step or agent inside it; `depth` is 0 for a run, 1 for what it holds, and so on.
#[derive(Serialize, Type, Debug, Clone, PartialEq)]
pub struct SubagentNode {
    pub id: String,
    pub depth: u32,
    pub name: String,
    pub state: SubagentState,
    /// The tool it runs right now.
    pub tool: Option<String>,
    /// Epoch milliseconds.
    pub started_at: Option<f64>,
    /// Epoch milliseconds, set once it ended.
    pub ended_at: Option<f64>,
}

/// One model a harness offers. `id` goes back in a pick; `provider` is set when the harness reaches models through several; `levels` are its thinking levels as the harness names them, lowest first, empty for a model that does not think; `default_level` is the one it starts on, when the harness says.
#[derive(Serialize, Deserialize, Type, Debug, Clone, PartialEq)]
pub struct ModelOption {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    pub levels: Vec<String>,
    pub default_level: Option<String>,
}

/// A model and thinking level the user picked for a pane whose harness is not running yet.
#[derive(Deserialize, Type, Debug, Clone, PartialEq)]
pub struct ModelPick {
    pub id: String,
    pub level: Option<String>,
}

/// An image in a message: base64 data and its MIME type.
#[derive(Serialize, Deserialize, Type, Debug, Clone, PartialEq)]
pub struct Image {
    pub data: String,
    pub mime_type: String,
}

/// The lines a call added to and removed from a file.
#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
pub struct LineChanges {
    pub added: u32,
    pub removed: u32,
}

/// Where a slash command is defined.
#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SlashKind {
    Skill,
    Prompt,
    Extension,
    /// A command of the harness itself that acts on the pane's session, such as picking the model or resuming another session; the adapter runs it when `send` gets it.
    Session,
}

/// A command the user can type after a slash; `name` has no slash. `origin` names the package it comes from; it is empty for a local file.
#[derive(Serialize, Type, Debug, Clone, PartialEq)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub kind: SlashKind,
    pub origin: Option<String>,
}

/// What a dialog asks.
#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogAsk {
    /// `current` is the option now in effect, which the dialog marks. `detail` is the command or file the ask is about; `descriptions` holds one line per option, empty when the options have none; `multi` takes several options, answered with `Values`.
    Select {
        options: Vec<String>,
        current: Option<String>,
        detail: Option<String>,
        descriptions: Vec<String>,
        multi: bool,
    },
    /// A select the user opened from the composer bar; it floats over the composer as a menu, with a filter row only when `filter`.
    Menu {
        options: Vec<String>,
        current: Option<String>,
        filter: bool,
    },
    /// `detail` is the shell command the confirm is about, shown as code in place of `message`. With a `url`, the buttons read Open and Decline, and Open opens the link before it answers.
    Confirm {
        message: String,
        detail: Option<String>,
        url: Option<String>,
    },
    Input {
        placeholder: String,
    },
}

/// The user's answer to a dialog.
#[derive(Deserialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogAnswer {
    /// The chosen option or the entered text.
    Value {
        value: String,
    },
    /// The options picked in a `multi` select.
    Values {
        values: Vec<String>,
    },
    Confirmed {
        confirmed: bool,
    },
    Cancelled,
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Done,
    Failed,
    Stopped,
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HarnessEvent {
    /// A user message the agent took in, whether it started the run or steered it; `at` is when it was sent, in epoch milliseconds, when the agent says.
    UserMessageChunk {
        text: String,
        images: Vec<Image>,
        at: Option<f64>,
    },
    /// Text of the reply that is streaming now.
    MessageChunk { text: String },
    /// The agent replaced the session's earlier messages with this summary to free the context; `tokens_before` is how many tokens they filled, when the agent says.
    Compaction {
        summary: String,
        tokens_before: Option<u32>,
    },
    /// The agent began a new block of reasoning.
    ThoughtStart,
    /// Text of the reasoning block that is streaming now.
    ThoughtChunk { text: String },
    /// A tool call began; `title` is what it does (Read, Search, List, Run, Edit, Write, a tool's name) and `detail` its path, command or arguments. `command` says `detail` is a shell command. `changes` holds the lines a write puts in its file, known before it runs.
    ToolStart {
        id: String,
        title: String,
        detail: String,
        command: bool,
        changes: Option<LineChanges>,
    },
    /// A tool call ended; `Stopped` means the run ended or was aborted before it reported. `images` are the ones the tool returned; `ms` is how long it ran, missing when the agent did not measure it; `changes` the lines an edit added and removed.
    ToolEnd {
        id: String,
        outcome: ToolOutcome,
        images: Vec<Image>,
        ms: Option<u32>,
        changes: Option<LineChanges>,
    },
    /// The model the agent runs and, when that model reasons, its thinking level; sent when a session starts and after each pick. `id` is the model's id in `Models`, when the harness says.
    Model {
        id: Option<String>,
        name: String,
        thinking: Option<String>,
    },
    /// Every model the harness offers; sent once it lists them.
    Models {
        /// The `name` of the harness's entry in `HARNESSES`.
        harness: &'static str,
        models: Vec<ModelOption>,
    },
    /// How many tokens the session fills of the model's context window; `tokens` is missing right after a compaction, until a reply measures it again; `window` is missing when the model has none, and until the harness tells it.
    Context {
        tokens: Option<u32>,
        window: Option<u32>,
    },
    /// The streaming reply is complete; `failed` when it ended in an error; `at` is when the reply was written, in epoch milliseconds, when the agent says.
    MessageEnd { failed: bool, at: Option<f64> },
    /// The agent opened a dialog and waits for an answer; `id` goes back with it.
    Dialog {
        id: String,
        title: String,
        ask: DialogAsk,
    },
    /// The commands the agent offers after a slash.
    Commands { commands: Vec<SlashCommand> },
    /// The subscription usage windows of the harness's account, shortest first; the whole list each time, empty when none applies, as for an API key.
    Usage {
        /// The `name` of the harness's entry in `HARNESSES`.
        harness: &'static str,
        windows: Vec<UsageWindow>,
    },
    /// The background subagent runs as a lineage, each node before its children; the whole list each time, empty when none applies.
    Subagents { nodes: Vec<SubagentNode> },
    /// The steering messages the agent has not taken in yet, oldest first; the whole queue each time.
    Queue { steering: Vec<String> },
    /// The agent started or finished work.
    State { running: bool },
    /// The user stopped the run; follows the run's `State { running: false }`.
    Aborted,
    /// Something the user should read: a refused command or a failed reply.
    Notice { text: String },
    /// The session the pane's agent holds: what the harness opens it again by.
    SessionInfo { id: String },
    /// The transcript starts over; the session's saved messages follow as the events a live reply produces, up to `HistoryEnd`.
    HistoryStart,
    /// The saved messages are all in; what follows is live.
    HistoryEnd,
    /// Text the harness puts in the composer, as the message a fork or a tree step went back to.
    Draft { text: String },
    /// The session was written to a file for reading outside the app.
    Exported { path: String },
    /// The harness process ended; `code` is missing when a signal ended it.
    Exited { code: Option<i32> },
    /// The pane's status changed or a reply ended; `last_message` is the text of the last reply; `changed` is false when a reply end repeats the status. Core adds it, no adapter emits it.
    Status {
        status: Status,
        last_message: String,
        changed: bool,
    },
    /// The session's permission mode and whether it still asks before it acts; sent when a session starts and after each pick, as `Model` is.
    Permissions { mode: String, asks: bool },
    /// The agent went on past its open dialog. The adapter emits it.
    DialogClosed,
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Status {
    Running,
    /// A dialog is open, or the last message ends with a question.
    Waiting,
    Done,
    Error,
    Exited {
        code: Option<i32>,
    },
}

/// What `status_for` remembers of one agent between events.
#[derive(Default)]
pub struct StatusTracker {
    running: bool,
    ran: bool,
    dialog: bool,
    failed: bool,
    exited: Option<Option<i32>>,
    reply_open: bool,
    last_message: String,
    reported: Option<Status>,
}

impl StatusTracker {
    pub fn last_message(&self) -> &str {
        &self.last_message
    }

    fn current(&self) -> Option<Status> {
        if let Some(code) = self.exited {
            Some(Status::Exited { code })
        } else if self.running {
            Some(if self.dialog {
                Status::Waiting
            } else {
                Status::Running
            })
        } else if !self.ran {
            None
        } else if self.failed {
            Some(Status::Error)
        } else if self.dialog
            || self
                .last_message
                .trim_end()
                .ends_with(['?', '？', '؟', '\u{37e}'])
        {
            Some(Status::Waiting)
        } else {
            Some(Status::Done)
        }
    }
}

/// A status to report. A message end reports the status again for its fresh last message; `changed` is false then.
#[derive(Debug, PartialEq)]
pub struct StatusReport {
    pub status: Status,
    pub changed: bool,
}

/// Folds an event into the agent's status; the status to report, or none when nothing changed. The only function that decides a status.
pub fn status_for(tracker: &mut StatusTracker, event: &HarnessEvent) -> Option<StatusReport> {
    match event {
        HarnessEvent::State { running: true } => {
            tracker.running = true;
            tracker.ran = true;
            tracker.failed = false;
        }
        HarnessEvent::State { running: false } => tracker.running = false,
        HarnessEvent::Dialog { .. } => tracker.dialog = true,
        HarnessEvent::DialogClosed => tracker.dialog = false,
        HarnessEvent::MessageChunk { text } => {
            if !tracker.reply_open {
                tracker.reply_open = true;
                tracker.last_message.clear();
            }
            tracker.last_message.push_str(text);
        }
        HarnessEvent::MessageEnd { failed, .. } => {
            tracker.reply_open = false;
            tracker.failed |= failed;
        }
        HarnessEvent::Exited { code } => tracker.exited = Some(*code),
        // Another session's runs are not this one's: the status starts over until it runs.
        HarnessEvent::HistoryStart => {
            // The view keeps a dialog open across a history, so the tracker keeps it too, to close it later.
            *tracker = StatusTracker {
                dialog: tracker.dialog,
                ..StatusTracker::default()
            };
            return None;
        }
        _ => return None,
    }
    let current = tracker.current();
    let changed = current != tracker.reported;
    let report = changed || matches!(event, HarnessEvent::MessageEnd { .. });
    tracker.reported = current;
    current
        .filter(|_| report)
        .map(|status| StatusReport { status, changed })
}

/// `path` inside `cwd` as relative to it; any other path as it is.
pub(crate) fn relative(path: &str, cwd: &Path) -> String {
    Path::new(path).strip_prefix(cwd).map_or_else(
        |_| path.into(),
        |inside| inside.to_string_lossy().into_owned(),
    )
}

/// A JSON string as it is; any other value as an empty string.
pub(crate) fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().into()
}

/// A JSON count that fits a `u32`.
pub(crate) fn count(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|count| u32::try_from(count).ok())
}

/// The callback an adapter's reader thread hands its events to.
pub(crate) type OnEvent = Arc<Mutex<dyn Fn(HarnessEvent) + Send>>;

/// A record's `timestamp`, ISO 8601 in UTC, as epoch milliseconds.
pub(crate) fn timestamp(value: &Value) -> Option<f64> {
    let stamp = value.as_str()?.strip_suffix('Z')?;
    let (date, time) = stamp.split_once('T')?;
    let mut date = date.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (date.next()??, date.next()??, date.next()??);
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let mut clock = clock.split(':').map(|part| part.parse::<i64>().ok());
    let (hour, minute, second) = (clock.next()??, clock.next()??, clock.next()??);
    let millis = format!("{fraction:0<3}").get(..3)?.parse::<i64>().ok()?;
    // Days since 1970-01-01 in the proleptic Gregorian calendar, by Howard Hinnant's days_from_civil.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_cycle - 719_468;
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second;
    Some((seconds * 1_000 + millis) as f64)
}

/// A level as its menu label shows it: "high" reads "High".
pub(crate) fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last_status(events: &[HarnessEvent]) -> Option<Status> {
        let mut tracker = StatusTracker::default();
        events.iter().fold(None, |last, event| {
            status_for(&mut tracker, event)
                .map(|report| report.status)
                .or(last)
        })
    }

    fn reply(text: &str) -> [HarnessEvent; 2] {
        [
            HarnessEvent::MessageChunk { text: text.into() },
            HarnessEvent::MessageEnd {
                failed: false,
                at: None,
            },
        ]
    }

    fn run(mut middle: Vec<HarnessEvent>) -> Vec<HarnessEvent> {
        middle.insert(0, HarnessEvent::State { running: true });
        middle.push(HarnessEvent::State { running: false });
        middle
    }

    #[test]
    fn a_settled_run_is_done() {
        assert_eq!(
            last_status(&run(reply("Fixed.").into())),
            Some(Status::Done)
        );
    }

    #[test]
    fn a_last_message_that_ends_with_a_question_waits() {
        assert_eq!(
            last_status(&run(reply("Which one do you want?  ").into())),
            Some(Status::Waiting)
        );
    }

    #[test]
    fn an_open_dialog_waits_until_the_agent_goes_on() {
        let mut events = vec![
            HarnessEvent::State { running: true },
            HarnessEvent::Dialog {
                id: "u1".into(),
                title: "Pick".into(),
                ask: DialogAsk::Confirm {
                    message: "".into(),
                    detail: None,
                    url: None,
                },
            },
        ];
        events.push(HarnessEvent::ToolEnd {
            id: "a".into(),
            outcome: ToolOutcome::Done,
            images: vec![],
            ms: None,
            changes: None,
        });
        assert_eq!(last_status(&events), Some(Status::Waiting));
        events.push(HarnessEvent::DialogClosed);
        assert_eq!(last_status(&events), Some(Status::Running));
    }

    #[test]
    fn a_dialog_stays_open_across_a_history_until_the_agent_goes_on() {
        let mut tracker = StatusTracker::default();
        for event in [
            HarnessEvent::Dialog {
                id: "u1".into(),
                title: "Clear".into(),
                ask: DialogAsk::Confirm {
                    message: "".into(),
                    detail: None,
                    url: None,
                },
            },
            HarnessEvent::Queue { steering: vec![] },
            HarnessEvent::HistoryStart,
        ] {
            status_for(&mut tracker, &event);
        }
        assert!(tracker.dialog);
        status_for(&mut tracker, &HarnessEvent::DialogClosed);
        assert!(!tracker.dialog);
    }

    #[test]
    fn a_failed_reply_is_an_error_once_the_run_settles() {
        let events = run(vec![HarnessEvent::MessageEnd {
            failed: true,
            at: None,
        }]);
        assert_eq!(last_status(&events), Some(Status::Error));
    }

    #[test]
    fn an_exit_reports_its_code() {
        let events = run(reply("Done.").into())
            .into_iter()
            .chain([HarnessEvent::Exited { code: Some(3) }])
            .collect::<Vec<_>>();
        assert_eq!(last_status(&events), Some(Status::Exited { code: Some(3) }));
    }

    #[test]
    fn a_session_that_opens_has_no_status_until_it_runs() {
        let mut tracker = StatusTracker::default();
        for event in run(reply("Fixed.").into()) {
            status_for(&mut tracker, &event);
        }
        let replayed = [HarnessEvent::HistoryStart]
            .into_iter()
            .chain(reply("An older reply?"))
            .filter_map(|event| status_for(&mut tracker, &event))
            .collect::<Vec<_>>();
        assert_eq!(replayed, vec![]);
        assert_eq!(tracker.last_message(), "An older reply?");
    }

    #[test]
    fn a_status_repeated_at_a_message_end_is_not_a_change() {
        let mut tracker = StatusTracker::default();
        let reports = run(reply("Fixed.").into())
            .into_iter()
            .chain([HarnessEvent::MessageEnd {
                failed: false,
                at: None,
            }])
            .filter_map(|event| status_for(&mut tracker, &event))
            .map(|report| (report.status, report.changed))
            .collect::<Vec<_>>();
        assert_eq!(
            reports,
            vec![
                (Status::Running, true),
                (Status::Running, false),
                (Status::Done, true),
                (Status::Done, false),
            ]
        );
    }

    #[test]
    fn an_agent_that_never_ran_has_no_status() {
        assert_eq!(last_status(&[HarnessEvent::ThoughtStart]), None);
    }
}
