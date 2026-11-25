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
    COMPACT,
];

const LEFT_OUT: [&str; 3] = ["model", "effort", "clear"];

pub(super) fn session_command(typed: &str) -> Option<&'static str> {
    let name = typed.split_whitespace().next()?.strip_prefix('/')?;
    SESSION_COMMANDS
        .iter()
        .map(|(command, _)| *command)
        .find(|command| *command == name)
}

pub(super) enum Change {
    Model(String),
    Effort(String),
    Mode(String),
}

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
    picked: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    mode: String,
}

impl Settings {
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

    pub(super) fn initialized(&mut self, answer: &Value) -> Vec<HarnessEvent> {
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

    pub(super) fn owns(id: &str) -> bool {
        [MODEL_DIALOG, THINKING_DIALOG, PERMISSIONS_DIALOG].contains(&id)
    }

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
