pub mod agent_tools;
mod asks;
mod calls;
pub mod claude;
pub mod codex;
mod earlier;
pub mod installed;
pub mod models;
mod numbered;
pub mod opencode;
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

pub struct Harness {
    pub name: &'static str,
    pub title: &'static str,
    pub binary: &'static str,
    pub install: &'static str,
    pub sign_in: Option<SignIn>,
    pub upgrade: &'static str,
    pub oldest_tested: (u32, u32, u32),
    pub starts: bool,
}

pub struct SignIn {
    pub check: &'static [&'static str],
    pub step: &'static str,
}

pub const HARNESSES: [Harness; 4] = [
    pi::HARNESS,
    codex::HARNESS,
    claude::HARNESS,
    opencode::HARNESS,
];

pub trait Agent: Send {
    fn capabilities(&self) -> Vec<Capability>;
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<bool, Error>;
    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error>;
    fn pick_model(&self, id: &str, level: Option<&str>) -> Result<(), Error>;
    fn abort(&self) -> Result<(), Error>;
    fn end_reported(self: Box<Self>);
}

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
        opencode::NAME => opencode::start(dir, session, tools, on_event),
        other => Err(Error::Message(format!(
            "Chord cannot run the harness \"{other}\""
        ))),
    }
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Chat,
    Attachments,
    Steer,
    Models,
    Context,
    Subagents,
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
pub struct UsageWindow {
    pub label: String,
    pub resets_at: Option<f64>,
}

impl UsageWindow {
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

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
pub struct SubagentNode {
    pub id: String,
    pub depth: u32,
    pub name: String,
    pub state: SubagentState,
    pub tool: Option<String>,
    pub started_at: Option<f64>,
    pub ended_at: Option<f64>,
}

#[derive(Serialize, Deserialize, Type, Debug, Clone, PartialEq)]
pub struct ModelOption {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    pub levels: Vec<String>,
    pub default_level: Option<String>,
}

#[derive(Deserialize, Type, Debug, Clone, PartialEq)]
pub struct ModelPick {
    pub id: String,
    pub level: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Debug, Clone, PartialEq)]
pub struct Image {
    pub data: String,
    pub mime_type: String,
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
pub struct LineChanges {
    pub added: u32,
    pub removed: u32,
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SlashKind {
    Skill,
    Prompt,
    Extension,
    Session,
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub kind: SlashKind,
    pub origin: Option<String>,
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogAsk {
    Select {
        options: Vec<String>,
        current: Option<String>,
        detail: Option<String>,
        descriptions: Vec<String>,
        multi: bool,
    },
    Menu {
        options: Vec<String>,
        current: Option<String>,
        filter: bool,
    },
    Confirm {
        message: String,
        detail: Option<String>,
        url: Option<String>,
    },
    Input {
        placeholder: String,
    },
}

#[derive(Deserialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DialogAnswer {
    Value { value: String },
    Values { values: Vec<String> },
    Confirmed { confirmed: bool },
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
    UserMessageChunk {
        text: String,
        images: Vec<Image>,
        at: Option<f64>,
    },
    MessageChunk {
        text: String,
    },
    Compaction {
        summary: String,
        tokens_before: Option<u32>,
    },
    Compacting {
        active: bool,
    },
    ThoughtStart,
    ThoughtChunk {
        text: String,
    },
    ToolStart {
        id: String,
        title: String,
        detail: String,
        command: bool,
        changes: Option<LineChanges>,
    },
    ToolEnd {
        id: String,
        outcome: ToolOutcome,
        images: Vec<Image>,
        ms: Option<u32>,
        changes: Option<LineChanges>,
    },
    Model {
        id: Option<String>,
        name: String,
        thinking: Option<String>,
    },
    Models {
        harness: &'static str,
        models: Vec<ModelOption>,
    },
    Context {
        tokens: Option<u32>,
        window: Option<u32>,
        estimated: bool,
    },
    MessageEnd {
        failed: bool,
        at: Option<f64>,
    },
    Dialog {
        id: String,
        title: String,
        ask: DialogAsk,
    },
    Commands {
        commands: Vec<SlashCommand>,
    },
    Usage {
        harness: &'static str,
        windows: Vec<UsageWindow>,
    },
    Subagents {
        nodes: Vec<SubagentNode>,
    },
    Queue {
        steering: Vec<String>,
    },
    State {
        running: bool,
    },
    Aborted,
    Notice {
        text: String,
    },
    SessionInfo {
        id: String,
    },
    HistoryStart,
    HistoryEnd,
    Draft {
        text: String,
    },
    Exported {
        path: String,
    },
    Exited {
        code: Option<i32>,
    },
    Status {
        status: Status,
        last_message: String,
        changed: bool,
    },
    Permissions {
        mode: String,
        asks: bool,
    },
    DialogClosed,
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Status {
    Running,
    Waiting,
    Done,
    Error,
    Exited { code: Option<i32> },
}

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

#[derive(Debug, PartialEq)]
pub struct StatusReport {
    pub status: Status,
    pub changed: bool,
}

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
        HarnessEvent::HistoryStart => {
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

pub(crate) fn relative(path: &str, cwd: &Path) -> String {
    Path::new(path).strip_prefix(cwd).map_or_else(
        |_| path.into(),
        |inside| inside.to_string_lossy().into_owned(),
    )
}

pub(crate) fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().into()
}

pub(crate) fn image(block: &Value) -> Option<Image> {
    (block["type"] == "image").then_some(())?;
    Some(Image {
        data: block["data"].as_str()?.into(),
        mime_type: block["mimeType"].as_str()?.into(),
    })
}

pub(crate) fn images(blocks: &Value) -> Vec<Image> {
    blocks
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(image)
        .collect()
}

pub(crate) fn count(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|count| u32::try_from(count).ok())
}

pub(crate) type OnEvent = Arc<Mutex<dyn Fn(HarnessEvent) + Send>>;

pub(crate) fn timestamp(value: &Value) -> Option<f64> {
    let stamp = value.as_str()?.strip_suffix('Z')?;
    let (date, time) = stamp.split_once('T')?;
    let mut date = date.split('-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (date.next()??, date.next()??, date.next()??);
    let (clock, fraction) = time.split_once('.').unwrap_or((time, ""));
    let mut clock = clock.split(':').map(|part| part.parse::<i64>().ok());
    let (hour, minute, second) = (clock.next()??, clock.next()??, clock.next()??);
    let millis = format!("{fraction:0<3}").get(..3)?.parse::<i64>().ok()?;
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    let days = era * 146_097 + of_cycle - 719_468;
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second;
    Some((seconds * 1_000 + millis) as f64)
}

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
