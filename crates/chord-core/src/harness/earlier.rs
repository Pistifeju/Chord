use super::{DialogAnswer, HarnessEvent, numbered};

const FORK_DIALOG: &str = "chord-fork";
const REWIND_DIALOG: &str = "chord-rewind";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Back {
    Fork,
    Rewind,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Message {
    pub(crate) id: String,
    pub(crate) before: Option<String>,
    pub(crate) text: String,
}

#[derive(Default)]
pub(crate) struct Earlier {
    back: Option<Back>,
    messages: Vec<Message>,
}

impl Earlier {
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

    pub(crate) fn owns(id: &str) -> bool {
        [FORK_DIALOG, REWIND_DIALOG].contains(&id)
    }

    pub(crate) fn pick(&mut self, answer: &DialogAnswer) -> Option<(Back, Message)> {
        let messages = std::mem::take(&mut self.messages);
        let back = self.back.take()?;
        Some((back, numbered::pick(messages, answer)?))
    }
}
