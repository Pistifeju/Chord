//! The session's settings and the slash commands: the models, the model and effort in effect, the permission mode, Claude Code's commands, and the picker menus.

use serde_json::Value;

use super::permissions;
use crate::harness::{
    DialogAnswer, DialogAsk, HarnessEvent, SlashCommand, SlashKind, capitalized, text,
};

const MODEL_DIALOG: &str = "claude-model";
const THINKING_DIALOG: &str = "claude-thinking";
const PERMISSIONS_DIALOG: &str = "claude-permissions";

/// Chord's commands that act on the session: each name and its description.
const SESSION_COMMANDS: [(&str, &str); 4] = [
    ("model", "Pick the model"),
    ("thinking", "Pick the thinking level"),
    ("permissions", "Pick what Claude Code may do without asking"),
    ("resume", "Open another session of this worktree"),
];

/// Claude Code's own commands the menu leaves out: Chord's `model` and `thinking` stand in for the first two.
const LEFT_OUT: [&str; 3] = ["model", "effort", "clear"];

/// What a picker's answer asks Claude Code to change.
pub(super) enum Change {
    /// The picked entry's `value`.
    Model(String),
    Effort(String),
    Mode(String),
}

#[derive(Default)]
pub(super) struct Settings {
    models: Vec<Value>,
    /// The `value` of the model entry the user picked, whose name the label shows.
    picked: Option<String>,
    /// The model id and effort `get_settings` applied.
    model: Option<String>,
    effort: Option<String>,
    mode: String,
}

impl Settings {
    /// The `initialize` answer: the models, the commands and the permission mode.
    pub(super) fn initialized(&mut self, answer: &Value) -> Vec<HarnessEvent> {
        self.models = answer["models"].as_array().cloned().unwrap_or_default();
        vec![
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
        let menu = |id: &str, title: &str, options: Vec<String>, current: Option<String>| {
            HarnessEvent::Dialog {
                id: id.into(),
                title: title.into(),
                ask: DialogAsk::Menu {
                    options,
                    current,
                    filter: id == MODEL_DIALOG,
                },
            }
        };
        match typed.trim() {
            "/model" => Some(menu(
                MODEL_DIALOG,
                "Model",
                self.models
                    .iter()
                    .map(|model| text(&model["displayName"]))
                    .collect(),
                self.current().map(|model| text(&model["displayName"])),
            )),
            "/thinking" => {
                let levels = self.levels();
                if levels.is_empty() {
                    return Some(HarnessEvent::Notice {
                        text: "The model does not think".into(),
                    });
                }
                Some(menu(
                    THINKING_DIALOG,
                    "Thinking Level",
                    levels.iter().map(|level| capitalized(level)).collect(),
                    self.effort.as_deref().map(capitalized),
                ))
            }
            "/permissions" => {
                let auto = self
                    .current()
                    .is_some_and(|model| model["supportsAutoMode"] == true);
                let modes = permissions::picker(auto);
                let current = modes
                    .contains(&self.mode.as_str())
                    .then(|| permissions::label(&self.mode).into());
                Some(menu(
                    PERMISSIONS_DIALOG,
                    "Permissions",
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

    /// What a picker's answer changes; none for a cancel.
    pub(super) fn pick(&self, id: &str, answer: &DialogAnswer) -> Option<Change> {
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

    // The picked entry, else the first other than `default` that resolves to the applied model.
    fn current(&self) -> Option<&Value> {
        let picked = self.picked.as_deref();
        let model = self.model.as_deref();
        self.models
            .iter()
            .find(|entry| picked.is_some() && entry["value"].as_str() == picked)
            .or_else(|| {
                self.models.iter().find(|entry| {
                    entry["value"] != "default"
                        && model.is_some()
                        && (entry["resolvedModel"].as_str() == model
                            || entry["value"].as_str() == model)
                })
            })
    }

    fn levels(&self) -> Vec<String> {
        self.current()
            .filter(|model| model["supportsEffort"] != false)
            .and_then(|model| model["supportedEffortLevels"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|level| level.as_str().map(String::from))
            .collect()
    }
}

/// Chord's session commands, then Claude Code's: a built-in one acts on the session, any other is a skill named after its plugin.
pub(super) fn commands(list: &Value) -> HarnessEvent {
    let session = SESSION_COMMANDS.map(|(name, description)| SlashCommand {
        name: name.into(),
        description: description.into(),
        kind: SlashKind::Session,
        origin: None,
    });
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
        commands: session.into_iter().chain(own).collect(),
    }
}
