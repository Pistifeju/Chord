use super::{DialogAnswer, HarnessEvent, numbered};

pub(crate) const DIALOG: &str = "chord-resume";

pub(crate) struct Session {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) age: u64,
}

#[derive(Default)]
pub(crate) struct Resume(Vec<String>);

impl Resume {
    pub(crate) fn offer(&mut self, sessions: Vec<Session>) -> HarnessEvent {
        let labels: Vec<String> = sessions
            .iter()
            .map(|session| format!("{} · {}", session.label, age(session.age)))
            .collect();
        self.0 = sessions.into_iter().map(|session| session.id).collect();
        numbered::select(
            DIALOG,
            "Resume Session",
            &labels,
            "This worktree has no other session to resume",
        )
    }

    pub(crate) fn pick(&mut self, answer: &DialogAnswer) -> Option<String> {
        numbered::pick(std::mem::take(&mut self.0), answer)
    }
}

fn age(seconds: u64) -> String {
    match seconds / 60 {
        0 => "now".into(),
        minutes @ 1..60 => format!("{minutes}m"),
        minutes @ 60..1440 => format!("{}h", minutes / 60),
        minutes => format!("{}d", minutes / 1440),
    }
}
