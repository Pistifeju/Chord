use super::earlier::{Back, Earlier, Message};
use super::resume::{self, Resume, Session};
use super::{DialogAnswer, HarnessEvent};

#[derive(Default)]
pub(crate) struct SessionDialogs {
    resume: Resume,
    earlier: Earlier,
}

pub(crate) enum Picked {
    Session(Option<String>),
    Message(Option<(Back, Message)>),
}

pub(crate) struct Answered {
    pub(crate) events: Vec<HarnessEvent>,
    pub(crate) picked: Picked,
}

impl SessionDialogs {
    pub(crate) fn offer_sessions(&mut self, sessions: Vec<Session>) -> HarnessEvent {
        self.resume.offer(sessions)
    }

    pub(crate) fn offer_messages(&mut self, back: Back, messages: Vec<Message>) -> HarnessEvent {
        self.earlier.offer(back, messages)
    }

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
