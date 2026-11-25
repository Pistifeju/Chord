use std::collections::HashMap;

use serde_json::Value;

use super::NAME;
use crate::harness::session_commands::{
    self, CLONE, COMPACT_PLAIN, Entry, MODEL, NEW, Picker, RESUME, THINKING,
};
use crate::harness::{
    DialogAnswer, HarnessEvent, ModelOption, SlashCommand, SlashKind, capitalized, text,
};

const MODEL_DIALOG: &str = "opencode-model";
const THINKING_DIALOG: &str = "opencode-thinking";

const SESSION_COMMANDS: [Entry; 7] = [
    MODEL,
    THINKING,
    NEW,
    RESUME,
    CLONE,
    (
        "reload",
        "Restart OpenCode on this session, with its config and MCP servers read again",
    ),
    COMPACT_PLAIN,
];

pub(super) enum Command {
    Menu(HarnessEvent),
    New,
    Resume,
    Reload,
    Clone,
    Compact,
}

#[derive(Default)]
struct Choice {
    id: String,
    current: Option<String>,
    values: Vec<(String, String)>,
}

pub(super) type Pick = (String, String);

#[derive(Default)]
pub(super) struct Settings {
    model: Choice,
    effort: Option<Choice>,
    levels: HashMap<String, Vec<String>>,
    commands: Vec<SlashCommand>,
}

impl Settings {
    pub(super) fn read(&mut self, options: &Value) -> Vec<HarnessEvent> {
        let find = |category: &str| {
            options
                .as_array()?
                .iter()
                .find(|option| option["category"] == category)
                .map(choice)
        };
        let Some(model) = find("model") else {
            return vec![];
        };
        self.model = model;
        self.effort = find("thought_level");
        if let Some(current) = &self.model.current {
            let levels = self.effort.as_ref().map(values).unwrap_or_default();
            self.levels.insert(current.clone(), levels);
        }
        vec![self.models(), self.model_event()]
    }

    pub(super) fn learn(&mut self, model: &str, options: &Value) {
        let effort = options
            .as_array()
            .into_iter()
            .flatten()
            .find(|option| option["category"] == "thought_level");
        let levels = effort.map(|effort| values(&choice(effort)));
        self.levels.insert(model.into(), levels.unwrap_or_default());
    }

    pub(super) fn model_ids(&self) -> Vec<String> {
        self.model.values.iter().map(|(id, _)| id.clone()).collect()
    }

    pub(super) fn read_commands(&mut self, list: &Value) -> HarnessEvent {
        self.commands = list
            .as_array()
            .into_iter()
            .flatten()
            .map(|command| SlashCommand {
                name: text(&command["name"]),
                description: text(&command["description"]),
                kind: SlashKind::Prompt,
                origin: None,
            })
            .filter(|command| command.name != "compact")
            .collect();
        self.commands_event()
    }

    pub(super) fn commands_event(&self) -> HarnessEvent {
        HarnessEvent::Commands {
            commands: session_commands::listed(&SESSION_COMMANDS)
                .chain(self.commands.iter().cloned())
                .collect(),
        }
    }

    pub(super) fn command(&self, text: &str) -> Option<Command> {
        match text.split_whitespace().next()?.strip_prefix('/')? {
            "model" => Some(Command::Menu(session_commands::menu(
                Picker::Model,
                MODEL_DIALOG,
                self.model
                    .values
                    .iter()
                    .map(|(_, name)| name.clone())
                    .collect(),
                self.model
                    .current
                    .as_deref()
                    .map(|id| self.named(id).into()),
            ))),
            "thinking" => {
                let effort = self.effort.as_ref();
                Some(Command::Menu(session_commands::thinking(
                    THINKING_DIALOG,
                    effort
                        .map(values)
                        .unwrap_or_default()
                        .iter()
                        .map(|level| capitalized(level))
                        .collect(),
                    effort
                        .and_then(|effort| effort.current.as_deref())
                        .map(capitalized),
                )))
            }
            "new" => Some(Command::New),
            "resume" => Some(Command::Resume),
            "reload" => Some(Command::Reload),
            "clone" => Some(Command::Clone),
            "compact" => Some(Command::Compact),
            _ => None,
        }
    }

    pub(super) fn pick(&self, id: &str, answer: &DialogAnswer) -> Option<Option<Pick>> {
        if ![MODEL_DIALOG, THINKING_DIALOG].contains(&id) {
            return None;
        }
        let DialogAnswer::Value { value } = answer else {
            return Some(None);
        };
        Some(if id == MODEL_DIALOG {
            let picked = self.model.values.iter().find(|(_, name)| name == value);
            picked.and_then(|(id, _)| self.model_pick(id))
        } else {
            let levels = self.effort.as_ref().map(values).unwrap_or_default();
            let picked = levels
                .into_iter()
                .find(|level| capitalized(level) == *value);
            picked.and_then(|level| self.effort_pick(&level))
        })
    }

    pub(super) fn model_pick(&self, id: &str) -> Option<Pick> {
        (!self.model.id.is_empty()).then(|| (self.model.id.clone(), id.into()))
    }

    pub(super) fn effort_pick(&self, level: &str) -> Option<Pick> {
        let effort = self.effort.as_ref()?;
        effort
            .values
            .iter()
            .any(|(value, _)| value == level)
            .then(|| (effort.id.clone(), level.into()))
    }

    pub(super) fn models(&self) -> HarnessEvent {
        HarnessEvent::Models {
            harness: NAME,
            models: self
                .model
                .values
                .iter()
                .map(|(id, name)| ModelOption {
                    id: id.clone(),
                    name: short(id, name).into(),
                    provider: id.split_once('/').map(|(provider, _)| provider.into()),
                    levels: self.levels.get(id).cloned().unwrap_or_default(),
                    default_level: None,
                })
                .collect(),
        }
    }

    fn model_event(&self) -> HarnessEvent {
        let id = self.model.current.clone();
        HarnessEvent::Model {
            name: id
                .as_deref()
                .map(|id| short(id, self.named(id)).into())
                .unwrap_or_default(),
            id,
            thinking: self
                .effort
                .as_ref()
                .and_then(|effort| effort.current.clone()),
        }
    }

    fn named<'a>(&'a self, id: &'a str) -> &'a str {
        self.model
            .values
            .iter()
            .find(|(value, _)| value == id)
            .map_or(id, |(_, name)| name)
    }
}

fn short<'a>(id: &str, name: &'a str) -> &'a str {
    let provider = id.split_once('/').map_or("", |(provider, _)| provider);
    name.strip_prefix(provider)
        .and_then(|name| name.strip_prefix('/'))
        .unwrap_or(name)
}

fn choice(option: &Value) -> Choice {
    Choice {
        id: text(&option["id"]),
        current: option["currentValue"].as_str().map(String::from),
        values: option["options"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|entry| match entry["options"].as_array() {
                Some(group) => group.iter().collect(),
                None => vec![entry],
            })
            .filter_map(|value| {
                let id = value["value"].as_str()?;
                Some((id.into(), value["name"].as_str().unwrap_or(id).into()))
            })
            .collect(),
    }
}

fn values(choice: &Choice) -> Vec<String> {
    choice
        .values
        .iter()
        .map(|(value, _)| value.clone())
        .collect()
}
