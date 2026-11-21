//! The session dialogs a harness without its own session list opens, `/resume`, `/fork` and `/rewind`, and what an answer to one of them picked.

use super::earlier::{Back, Earlier, Message};
use super::resume::{self, Resume, Session};
use super::{DialogAnswer, HarnessEvent};

/// The open resume, fork or rewind dialog's entries.
#[derive(Default)]
pub(crate) struct SessionDialogs {
    resume: Resume,
    earlier: Earlier,
}

/// What an answer to a session dialog picked.
pub(crate) enum Picked {
    /// The session `/resume` opens; none after a cancel.
    Session(Option<String>),
    /// The message `/fork` or `/rewind` goes back to; none after a cancel.
    Message(Option<(Back, Message)>),
}

/// An answer a session dialog took: the dialog closes, a picked message goes back to the composer, and what the answer picked.
pub(crate) struct Answered {
    pub(crate) events: Vec<HarnessEvent>,
    pub(crate) picked: Picked,
}

impl SessionDialogs {
    /// The resume select of `sessions`.
    pub(crate) fn offer_sessions(&mut self, sessions: Vec<Session>) -> HarnessEvent {
        self.resume.offer(sessions)
    }

    /// The fork or rewind select of `messages`.
    pub(crate) fn offer_messages(&mut self, back: Back, messages: Vec<Message>) -> HarnessEvent {
        self.earlier.offer(back, messages)
    }

    /// The answer to the session dialog `id`; none when `id` names another dialog.
    pub(crate) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Option<Answered> {
        let picked = if id == resume::DIALOG {
            Picked::Session(self.resume.pick(answer))
        } else if Earlier::owns(id) {
            Picked::Message(self.earlier.pick(answer))
        } else {
            return None;
        };
        let mut events = vec![HarnessEvent::DialogClosed];
        if let Picked::Message(Some((_, message))) = &picked {
            events.push(HarnessEvent::Draft {
                text: message.text.clone(),
            });
        }
        Some(Answered { events, picked })
    }
}
