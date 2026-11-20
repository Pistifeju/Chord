//! `/resume`: the numbered select of a worktree's other sessions, and the session a pick names.

use super::{DialogAnswer, HarnessEvent, numbered};

pub(crate) const DIALOG: &str = "chord-resume";

/// One session the select offers.
pub(crate) struct Session {
    pub(crate) id: String,
    pub(crate) label: String,
    /// Seconds since its last change.
    pub(crate) age: u64,
}

/// The sessions the open resume dialog lists, in its order.
#[derive(Default)]
pub(crate) struct Resume(Vec<String>);

impl Resume {
    /// The select of `sessions`, newest first; a notice when there is none.
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

    /// The session the answer picked; none after a cancel.
    pub(crate) fn pick(&mut self, answer: &DialogAnswer) -> Option<String> {
        numbered::pick(std::mem::take(&mut self.0), answer)
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
