//! What a harness tells the chat: the events every adapter emits.

pub mod pi;
pub mod sessions;
mod shell;

use serde::{Deserialize, Serialize};
use specta::Type;

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
}

/// An image in a message: base64 data and its MIME type.
#[derive(Serialize, Deserialize, Type, Debug, Clone, PartialEq)]
pub struct Image {
    pub data: String,
    pub mime_type: String,
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
    /// A user message the agent took in, whether it started the run or steered it.
    UserMessageChunk { text: String, images: Vec<Image> },
    /// Text of the reply that is streaming now.
    MessageChunk { text: String },
    /// The agent began a new block of reasoning.
    ThoughtStart,
    /// Text of the reasoning block that is streaming now.
    ThoughtChunk { text: String },
    /// A tool call began; `title` is what it does (Read, Search, List, Run, Edit, Write, a tool's name) and `detail` its path, command or arguments.
    ToolStart {
        id: String,
        title: String,
        detail: String,
    },
    /// A tool call ended; `Stopped` means the run ended or was aborted before it reported. `images` are the ones the tool returned.
    ToolEnd {
        id: String,
        outcome: ToolOutcome,
        images: Vec<Image>,
    },
    /// The streaming reply is complete; `failed` when it ended in an error.
    MessageEnd { failed: bool },
    /// The agent opened a dialog and waits for an answer.
    Dialog,
    /// The steering messages the agent has not taken in yet, oldest first; the whole queue each time.
    Queue { steering: Vec<String> },
    /// The agent started or finished work.
    State { running: bool },
    /// Something the user should read: a refused command or a failed reply.
    Notice { text: String },
    /// The harness process ended; `code` is missing when a signal ended it.
    Exited { code: Option<i32> },
    /// The pane's status changed or a reply ended; `last_message` is the text of the last reply. Core adds it, no adapter emits it.
    Status {
        status: Status,
        last_message: String,
    },
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

/// Folds an event into the agent's status; the status to report, or none when nothing changed. The only function that decides a status.
pub fn status_for(tracker: &mut StatusTracker, event: &HarnessEvent) -> Option<Status> {
    match event {
        HarnessEvent::State { running: true } => {
            tracker.running = true;
            tracker.ran = true;
            tracker.dialog = false;
            tracker.failed = false;
        }
        HarnessEvent::State { running: false } => tracker.running = false,
        HarnessEvent::Dialog => tracker.dialog = true,
        HarnessEvent::MessageChunk { text } => {
            if !tracker.reply_open {
                tracker.reply_open = true;
                tracker.last_message.clear();
            }
            tracker.last_message.push_str(text);
            tracker.dialog = false;
        }
        HarnessEvent::MessageEnd { failed } => {
            tracker.reply_open = false;
            tracker.failed |= failed;
        }
        HarnessEvent::ThoughtStart
        | HarnessEvent::ThoughtChunk { .. }
        | HarnessEvent::ToolStart { .. }
        | HarnessEvent::ToolEnd { .. } => tracker.dialog = false,
        HarnessEvent::Exited { code } => tracker.exited = Some(*code),
        _ => return None,
    }
    let current = tracker.current();
    let report = current != tracker.reported || matches!(event, HarnessEvent::MessageEnd { .. });
    tracker.reported = current;
    current.filter(|_| report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn last_status(events: &[HarnessEvent]) -> Option<Status> {
        let mut tracker = StatusTracker::default();
        events
            .iter()
            .fold(None, |last, event| status_for(&mut tracker, event).or(last))
    }

    fn reply(text: &str) -> [HarnessEvent; 2] {
        [
            HarnessEvent::MessageChunk { text: text.into() },
            HarnessEvent::MessageEnd { failed: false },
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
        let mut events = vec![HarnessEvent::State { running: true }, HarnessEvent::Dialog];
        assert_eq!(last_status(&events), Some(Status::Waiting));
        events.push(HarnessEvent::ToolEnd {
            id: "a".into(),
            outcome: ToolOutcome::Done,
            images: vec![],
        });
        assert_eq!(last_status(&events), Some(Status::Running));
    }

    #[test]
    fn a_failed_reply_is_an_error_once_the_run_settles() {
        let events = run(vec![HarnessEvent::MessageEnd { failed: true }]);
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
    fn an_agent_that_never_ran_has_no_status() {
        assert_eq!(last_status(&[HarnessEvent::ThoughtStart]), None);
    }
}
