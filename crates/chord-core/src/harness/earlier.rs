//! `/fork` and `/rewind`: the numbered select of the session's user messages, and the message a pick names.

use super::{DialogAnswer, HarnessEvent, numbered};

const FORK_DIALOG: &str = "chord-fork";
const REWIND_DIALOG: &str = "chord-rewind";

/// What a pick of an earlier message does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Back {
    /// A new session holds what came before the message; the old one stays.
    Fork,
    /// The session drops the message and every later turn.
    Rewind,
}

/// One user message the select offers.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Message {
    /// What the harness names the message by.
    pub(crate) id: String,
    /// What the harness names the point just before the message by; none for the session's first message.
    pub(crate) before: Option<String>,
    pub(crate) text: String,
}

/// The messages the open fork or rewind dialog lists, in its order.
#[derive(Default)]
pub(crate) struct Earlier {
    back: Option<Back>,
    messages: Vec<Message>,
}

impl Earlier {
    /// The select of `messages`, oldest first; a notice when there is none.
    pub(crate) fn offer(&mut self, back: Back, messages: Vec<Message>) -> HarnessEvent {
        let (id, title, verb) = match back {
            Back::Fork => (FORK_DIALOG, "Fork From", "fork from"),
            Back::Rewind => (REWIND_DIALOG, "Rewind To", "rewind to"),
        };
        let labels: Vec<String> = messages
            .iter()
            .map(|message| {
                let line = message.text.lines().find(|line| !line.trim().is_empty());
                line.unwrap_or_default().trim().to_string()
            })
            .collect();
        self.back = Some(back);
        self.messages = messages;
        numbered::select(
            id,
            title,
            &labels,
            &format!("The session has no message to {verb}"),
        )
    }

    /// Whether `id` names the fork or rewind dialog.
    pub(crate) fn owns(id: &str) -> bool {
        [FORK_DIALOG, REWIND_DIALOG].contains(&id)
    }

    /// The message the answer picked and what to do with it; none after a cancel.
    pub(crate) fn pick(&mut self, answer: &DialogAnswer) -> Option<(Back, Message)> {
        let messages = std::mem::take(&mut self.messages);
        let back = self.back.take()?;
        Some((back, numbered::pick(messages, answer)?))
    }
}
