//! The session's settings and the slash commands: the models, the model and effort in effect, the permission mode, Claude Code's commands, and the picker menus.

use serde_json::Value;

use super::{NAME, permissions};
use crate::harness::session_commands::{
    self, CLONE, COMPACT, Entry, FORK, MODEL, NEW, Picker, RESUME, REWIND, THINKING,
};
use crate::harness::{
    DialogAnswer, HarnessEvent, ModelOption, SlashCommand, SlashKind, capitalized, text,
};

const MODEL_DIALOG: &str = "claude-model";
const THINKING_DIALOG: &str = "claude-thinking";
const PERMISSIONS_DIALOG: &str = "claude-permissions";

/// Chord's commands that act on the session: each name and its description.
const SESSION_COMMANDS: [Entry; 10] = [
    MODEL,
    THINKING,
    ("permissions", "Pick what Claude Code may do without asking"),
    NEW,
    RESUME,
    FORK,
    CLONE,
    REWIND,
    (
        "reload",
        "Restart Claude Code on this session, with its settings, skills and MCP servers read again",
    ),
    // Claude Code runs it from the message text.
    COMPACT,
];

/// Claude Code's own commands the menu leaves out: Chord's `model`, `thinking` and `new` stand in for them.
const LEFT_OUT: [&str; 3] = ["model", "effort", "clear"];

/// The name of the Chord session command `typed` starts with; none for any other text.
pub(super) fn session_command(typed: &str) -> Option<&'static str> {
    let name = typed.split_whitespace().next()?.strip_prefix('/')?;
    SESSION_COMMANDS
        .iter()
        .map(|(command, _)| *command)
        .find(|command| *command == name)
}

/// What a picker's answer asks Claude Code to change.
pub(super) enum Change {
    /// The picked entry's `value`.
    Model(String),
    Effort(String),
    Mode(String),
}

/// The user's picks, which a reload's new process starts with: the model entry's `value`, the effort and the permission mode.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Picks {
    pub(super) model: Option<String>,
    pub(super) effort: Option<String>,
    pub(super) mode: Option<String>,
}

#[derive(Default)]
pub(super) struct Settings {
    picks: Picks,
    models: Vec<Value>,
    /// The `value` of the model entry the user picked, whose name the label shows.
    picked: Option<String>,
    /// The model id and effort `get_settings` applied.
    model: Option<String>,
    effort: Option<String>,
    mode: String,
}

impl Settings {
    /// Settings for a process started with `picks`, the user's picks of an earlier process on the session.
    pub(super) fn with_picks(picks: Picks) -> Self {
        Settings {
            picked: picks.model.clone(),
            picks,
            ..Settings::default()
        }
    }

    pub(super) fn picks(&self) -> &Picks {
        &self.picks
    }

    /// The `initialize` answer: the models, the commands and the permission mode.
    pub(super) fn initialized(&mut self, answer: &Value) -> Vec<HarnessEvent> {
        // `default` only repeats one of the listed models under another name.
        self.models = answer["models"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|model| model["value"] != "default")
            .cloned()
            .collect();
        let models = self
            .models
            .iter()
            .map(|model| ModelOption {
                id: text(&model["value"]),
                name: text(&model["displayName"]),
                provider: None,
                levels: levels(model),
                default_level: None,
            })
            .collect();
        vec![
            HarnessEvent::Models {
                harness: NAME,
                models,
            },
            commands(&answer["commands"]),
            self.mode_changed(text(&answer["current_permission_mode"])),
        ]
    }

    /// The model and effort `get_settings` applied.
    pub(super) fn applied(&mut self, applied: &Value) -> Option<HarnessEvent> {
        self.model = Some(applied["model"].as_str()?.into());
        self.effort = applied["effort"].as_str().map(String::from);
        let current = self.current();
        let name = current
            .and_then(|model| model["displayName"].as_str())
            .or(self.model.as_deref())?;
        let thinks = current.is_none_or(|model| model["supportsEffort"] != false);
        Some(HarnessEvent::Model {
            id: current
                .and_then(|model| model["value"].as_str())
                .map(String::from),
            name: name.into(),
            thinking: self.effort.clone().filter(|_| thinks),
        })
    }

    /// A set model took hold.
    pub(super) fn model_set(&mut self, value: String) {
        self.picked = Some(value);
    }

    pub(super) fn mode_changed(&mut self, mode: String) -> HarnessEvent {
        self.mode = mode;
        HarnessEvent::Permissions {
            mode: permissions::label(&self.mode).into(),
            asks: permissions::asks(&self.mode),
        }
    }

    /// The picker or notice a typed session command opens; none for any other text.
    pub(super) fn command(&self, typed: &str) -> Option<HarnessEvent> {
        match typed.trim() {
            "/model" => Some(session_commands::menu(
                Picker::Model,
                MODEL_DIALOG,
                self.models
                    .iter()
                    .map(|model| text(&model["displayName"]))
                    .collect(),
                self.current().map(|model| text(&model["displayName"])),
            )),
            "/thinking" => Some(session_commands::thinking(
                THINKING_DIALOG,
                self.levels()
                    .iter()
                    .map(|level| capitalized(level))
                    .collect(),
                self.effort.as_deref().map(capitalized),
            )),
            "/permissions" => {
                let auto = self
                    .current()
                    .is_some_and(|model| model["supportsAutoMode"] == true);
                let modes = permissions::picker(auto);
                let current = modes
                    .contains(&self.mode.as_str())
                    .then(|| permissions::label(&self.mode).into());
                Some(session_commands::menu(
                    Picker::Permissions,
                    PERMISSIONS_DIALOG,
                    modes
                        .iter()
                        .map(|mode| permissions::label(mode).into())
                        .collect(),
                    current,
                ))
            }
            _ => None,
        }
    }

    /// Whether `id` names one of the adapter's pickers.
    pub(super) fn owns(id: &str) -> bool {
        [MODEL_DIALOG, THINKING_DIALOG, PERMISSIONS_DIALOG].contains(&id)
    }

    /// What a picker's answer changes, kept as the user's pick; none for a cancel.
    pub(super) fn pick(&mut self, id: &str, answer: &DialogAnswer) -> Option<Change> {
        let change = self.change(id, answer)?;
        let (Change::Model(pick) | Change::Effort(pick) | Change::Mode(pick)) = &change;
        let kept = match change {
            Change::Model(_) => &mut self.picks.model,
            Change::Effort(_) => &mut self.picks.effort,
            Change::Mode(_) => &mut self.picks.mode,
        };
        *kept = Some(pick.clone());
        Some(change)
    }

    fn change(&self, id: &str, answer: &DialogAnswer) -> Option<Change> {
        let DialogAnswer::Value { value } = answer else {
            return None;
        };
        match id {
            MODEL_DIALOG => self
                .models
                .iter()
                .find(|model| model["displayName"] == value.as_str())
                .map(|model| Change::Model(text(&model["value"]))),
            // Claude Code takes any string as the effort, so only a level of the model goes out.
            THINKING_DIALOG => self
                .levels()
                .into_iter()
                .find(|level| capitalized(level) == *value)
                .map(Change::Effort),
            _ => permissions::picker(true)
                .into_iter()
                .find(|mode| permissions::label(mode) == value)
                .map(|mode| Change::Mode(mode.into())),
        }
    }

    // The picked entry, else the first that resolves to the applied model.
    fn current(&self) -> Option<&Value> {
        let picked = self.picked.as_deref();
        let model = self.model.as_deref();
        self.models
            .iter()
            .find(|entry| picked.is_some() && entry["value"].as_str() == picked)
            .or_else(|| {
                self.models.iter().find(|entry| {
                    model.is_some()
                        && (entry["resolvedModel"].as_str() == model
                            || entry["value"].as_str() == model)
                })
            })
    }

    fn levels(&self) -> Vec<String> {
        self.current().map(levels).unwrap_or_default()
    }

    /// The model entry's `value` and the effort the user picked in the agent picker, kept as the user's picks.
    pub(super) fn chose(&mut self, model: &str, effort: Option<&str>) {
        self.picks.model = Some(model.into());
        if let Some(effort) = effort {
            self.picks.effort = Some(effort.into());
        }
    }
}

fn levels(model: &Value) -> Vec<String> {
    if model["supportsEffort"] == false {
        return vec![];
    }
    model["supportedEffortLevels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|level| level.as_str().map(String::from))
        .collect()
}

/// Chord's session commands, then Claude Code's: a built-in one acts on the session, any other is a skill named after its plugin.
pub(super) fn commands(list: &Value) -> HarnessEvent {
    let session = session_commands::listed(&SESSION_COMMANDS);
    let own = list.as_array().into_iter().flatten().filter_map(|command| {
        let name = command["name"].as_str()?;
        let taken = LEFT_OUT.contains(&name)
            || SESSION_COMMANDS.iter().any(|(session, _)| *session == name);
        let builtin = command["builtin"] == true;
        (!taken).then(|| SlashCommand {
            name: name.into(),
            description: text(&command["description"]),
            kind: if builtin {
                SlashKind::Session
            } else {
                SlashKind::Skill
            },
            origin: (!builtin)
                .then(|| name.split_once(':').map(|(plugin, _)| plugin.to_string()))
                .flatten(),
        })
    });
    HarnessEvent::Commands {
        commands: session.chain(own).collect(),
    }
}
