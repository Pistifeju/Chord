//! The session commands the harnesses share: their names and descriptions, their slash command list, the picker menus they open, and the reply to a finished reload.

use super::{DialogAsk, HarnessEvent, SlashCommand, SlashKind};

/// A session command's name and description.
pub(crate) type Entry = (&'static str, &'static str);

pub(crate) const MODEL: Entry = ("model", "Pick the model");
pub(crate) const THINKING: Entry = ("thinking", "Pick the thinking level");
pub(crate) const NEW: Entry = ("new", "Start a new session");
pub(crate) const RESUME: Entry = ("resume", "Open another session of this worktree");
pub(crate) const FORK: Entry = ("fork", "Start a new session from an earlier message");
pub(crate) const CLONE: Entry = ("clone", "Copy this session into a new one");
pub(crate) const REWIND: Entry = ("rewind", "Go back to an earlier message of this session");
pub(crate) const COMPACT: Entry = (
    "compact",
    "Summarize the session to free the context, optionally with what to keep",
);
/// `/compact` for a harness whose summary takes no instructions.
pub(crate) const COMPACT_PLAIN: Entry = ("compact", "Summarize the session to free the context");

/// The notice `/thinking` gives for a model without thinking levels.
pub(crate) const NO_THINKING: &str = "The model does not think";

/// The slash commands of `entries`, in their order.
pub(crate) fn listed(entries: &[Entry]) -> impl Iterator<Item = SlashCommand> + '_ {
    entries.iter().map(|(name, description)| SlashCommand {
        name: (*name).into(),
        description: (*description).into(),
        kind: SlashKind::Session,
        origin: None,
    })
}

/// A picker menu a session command opens.
#[derive(Clone, Copy)]
pub(crate) enum Picker {
    Model,
    Thinking,
    Permissions,
}

/// The menu of `picker` with the dialog id `id`; only the model menu has a filter field.
pub(crate) fn menu(
    picker: Picker,
    id: &str,
    options: Vec<String>,
    current: Option<String>,
) -> HarnessEvent {
    HarnessEvent::Dialog {
        id: id.into(),
        title: match picker {
            Picker::Model => "Model",
            Picker::Thinking => "Thinking Level",
            Picker::Permissions => "Permissions",
        }
        .into(),
        ask: DialogAsk::Menu {
            options,
            current,
            filter: matches!(picker, Picker::Model),
        },
    }
}

/// The thinking level menu, or the notice that the model does not think when it has no level.
pub(crate) fn thinking(id: &str, options: Vec<String>, current: Option<String>) -> HarnessEvent {
    if options.is_empty() {
        return HarnessEvent::Notice {
            text: NO_THINKING.into(),
        };
    }
    menu(Picker::Thinking, id, options, current)
}

/// The reply a finished `/reload` shows.
pub(crate) fn reloaded() -> [HarnessEvent; 2] {
    [
        HarnessEvent::MessageChunk {
            text: "Reloaded extensions, skills, prompts and context files".into(),
        },
        HarnessEvent::MessageEnd {
            failed: false,
            at: None,
        },
    ]
}
