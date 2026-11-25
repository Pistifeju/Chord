//! The thread's settings and the slash commands: the models, the model and thinking level in effect, the permission mode, the skills, the picker menus, and the picks every later turn carries.

use serde_json::{Map, Value, json};

use super::NAME;
use super::permissions;
use crate::harness::earlier::Back;
use crate::harness::session_commands::{
    self, CLONE, COMPACT_PLAIN, Entry, FORK, MODEL, NEW, Picker, RESUME, REWIND, THINKING,
};
use crate::harness::{
    DialogAnswer, HarnessEvent, ModelOption, SlashCommand, SlashKind, capitalized,
};

const MODEL_DIALOG: &str = "codex-model";
const THINKING_DIALOG: &str = "codex-thinking";
const PERMISSIONS_DIALOG: &str = "codex-permissions";

/// The commands of Codex itself that act on the thread: each name and its description.
const SESSION_COMMANDS: [Entry; 10] = [
    MODEL,
    THINKING,
    ("permissions", "Pick what Codex may do without asking"),
    NEW,
    RESUME,
    FORK,
    CLONE,
    REWIND,
    (
        "reload",
        "Restart Codex on this session, with its config, skills and MCP servers read again",
    ),
    COMPACT_PLAIN,
];

struct Model {
    id: String,
    name: String,
    efforts: Vec<String>,
    default_effort: Option<String>,
}

struct Skill {
    name: String,
    description: String,
    path: String,
}

/// What a typed command becomes.
pub(super) enum Command {
    /// A picker menu opens.
    Menu(HarnessEvent),
    /// The user picks another thread of the worktree to open.
    Resume,
    /// A new Codex process opens a new thread.
    New,
    /// A new Codex process opens the same thread.
    Reload,
    /// The user picks an earlier message to go back to.
    Back(Back),
    Clone,
    Compact,
    /// A skill runs: the text Codex reads and the skill's input.
    Skill {
        text: String,
        skill: Value,
    },
}

#[derive(Default)]
pub(super) struct Settings {
    models: Vec<Model>,
    model: String,
    effort: Option<String>,
    approval: Value,
    sandbox: Value,
    /// The `workspaceWrite` policy the thread opened with, so a pick of Workspace keeps its roots and network setting.
    workspace: Option<Value>,
    /// The user's picks, sent with every turn so they hold whatever Codex keeps between turns.
    picks: Map<String, Value>,
    skills: Vec<Skill>,
}

impl Settings {
    /// Reads `model/list`; without it a model shows its id.
    pub(super) fn read_models(&mut self, result: &Value) -> HarnessEvent {
        self.models = list(&result["data"])
            .filter_map(|model| {
                let id = model["id"].as_str()?;
                Some(Model {
                    id: id.into(),
                    name: model["displayName"].as_str().unwrap_or(id).into(),
                    efforts: list(&model["supportedReasoningEfforts"])
                        .filter_map(|option| option["reasoningEffort"].as_str().map(String::from))
                        .collect(),
                    default_effort: model["defaultReasoningEffort"].as_str().map(String::from),
                })
            })
            .collect();
        HarnessEvent::Models {
            harness: NAME,
            models: self
                .models
                .iter()
                .map(|model| ModelOption {
                    id: model.id.clone(),
                    name: model.name.clone(),
                    provider: None,
                    levels: model.efforts.clone(),
                    default_level: model.default_effort.clone(),
                })
                .collect(),
        }
    }

    /// Switches to the model `id` for the next turn, at `level`, else at the current effort when the model has it, else at its default. Before `model/list` answers, the effort stays.
    pub(super) fn choose(&mut self, id: &str, level: Option<&str>) -> HarnessEvent {
        let model = self.models.iter().find(|model| model.id == id);
        let effort = match (level, model) {
            (Some(level), _) => Some(level.to_string()),
            (None, Some(model)) => match &self.effort {
                Some(effort) if model.efforts.contains(effort) => Some(effort.clone()),
                _ => model.default_effort.clone(),
            },
            (None, None) => self.effort.clone(),
        };
        self.model = id.into();
        self.picks.insert("model".into(), json!(self.model));
        if effort != self.effort {
            self.effort = effort;
            self.picks.insert("effort".into(), json!(self.effort));
        }
        self.model_event()
    }

    /// Reads `skills/list`: the session commands, then each enabled skill.
    pub(super) fn read_skills(&mut self, result: &Value) -> HarnessEvent {
        self.skills = list(&result["data"])
            .flat_map(|entry| list(&entry["skills"]))
            .filter(|skill| skill["enabled"] != false)
            .filter_map(|skill| {
                Some(Skill {
                    name: skill["name"].as_str()?.into(),
                    description: skill["shortDescription"]
                        .as_str()
                        .or(skill["description"].as_str())
                        .unwrap_or_default()
                        .into(),
                    path: skill["path"].as_str()?.into(),
                })
            })
            .collect();
        let session = session_commands::listed(&SESSION_COMMANDS);
        let skills = self.skills.iter().map(|skill| SlashCommand {
            name: skill.name.clone(),
            description: skill.description.clone(),
            kind: SlashKind::Skill,
            origin: None,
        });
        HarnessEvent::Commands {
            commands: session.chain(skills).collect(),
        }
    }

    /// Settings that carry `picks`, the user's picks of an earlier process on the thread.
    pub(super) fn with_picks(picks: Map<String, Value>) -> Self {
        Settings {
            picks,
            ..Settings::default()
        }
    }

    /// The thread's answer, whether started or resumed; the user's picks override it.
    pub(super) fn opened(&mut self, result: &Value) -> Vec<HarnessEvent> {
        let picked = |key: &str, opened: &Value| self.picks.get(key).unwrap_or(opened).clone();
        let model = picked("model", &result["model"]);
        let effort = picked("effort", &result["reasoningEffort"]);
        let approval = picked("approvalPolicy", &result["approvalPolicy"]);
        let sandbox = picked("sandboxPolicy", &result["sandbox"]);
        self.read(&model, &effort, &approval, &sandbox)
    }

    /// `thread/settings/updated`.
    pub(super) fn updated(&mut self, settings: &Value) -> Vec<HarnessEvent> {
        self.read(
            &settings["model"],
            &settings["effort"],
            &settings["approvalPolicy"],
            &settings["sandboxPolicy"],
        )
    }

    /// The picks a new turn carries.
    pub(super) fn picks(&self) -> &Map<String, Value> {
        &self.picks
    }

    /// The session command or skill `text` names; none for a plain message.
    pub(super) fn command(&self, text: &str) -> Option<Command> {
        let text = text.trim();
        let (name, rest) = text
            .strip_prefix('/')?
            .split_once(' ')
            .unwrap_or((&text[1..], ""));
        match name {
            "model" => Some(Command::Menu(session_commands::menu(
                Picker::Model,
                MODEL_DIALOG,
                self.models.iter().map(|model| model.name.clone()).collect(),
                Some(self.name().into()),
            ))),
            "thinking" => {
                let efforts = self
                    .current()
                    .map(|model| model.efforts.as_slice())
                    .unwrap_or(&[]);
                Some(Command::Menu(session_commands::thinking(
                    THINKING_DIALOG,
                    efforts.iter().map(|effort| capitalized(effort)).collect(),
                    self.effort.as_deref().map(capitalized),
                )))
            }
            "resume" => Some(Command::Resume),
            "new" => Some(Command::New),
            "reload" => Some(Command::Reload),
            "fork" => Some(Command::Back(Back::Fork)),
            "rewind" => Some(Command::Back(Back::Rewind)),
            "clone" => Some(Command::Clone),
            "compact" => Some(Command::Compact),
            "permissions" => Some(Command::Menu(session_commands::menu(
                Picker::Permissions,
                PERMISSIONS_DIALOG,
                permissions::MODES.map(String::from).to_vec(),
                Some(permissions::mode(&self.approval, &self.sandbox).0),
            ))),
            _ => {
                let skill = self.skills.iter().find(|skill| skill.name == name)?;
                Some(Command::Skill {
                    text: format!("${name} {rest}").trim_end().into(),
                    skill: json!({ "type": "skill", "name": skill.name, "path": skill.path }),
                })
            }
        }
    }

    /// A picker's answer, applied to the next turn and told at once; none when `id` is no picker of the adapter.
    pub(super) fn pick(&mut self, id: &str, answer: &DialogAnswer) -> Option<Vec<HarnessEvent>> {
        if ![MODEL_DIALOG, THINKING_DIALOG, PERMISSIONS_DIALOG].contains(&id) {
            return None;
        }
        let DialogAnswer::Value { value } = answer else {
            return Some(vec![]);
        };
        match id {
            MODEL_DIALOG => {
                let Some(model) = self.models.iter().find(|model| model.name == *value) else {
                    return Some(vec![]);
                };
                let id = model.id.clone();
                Some(vec![self.choose(&id, None)])
            }
            THINKING_DIALOG => {
                let efforts = self
                    .current()
                    .map(|model| model.efforts.as_slice())
                    .unwrap_or(&[]);
                let Some(effort) = efforts.iter().find(|effort| capitalized(effort) == *value)
                else {
                    return Some(vec![]);
                };
                self.effort = Some(effort.clone());
                self.picks.insert("effort".into(), json!(self.effort));
                Some(vec![self.model_event()])
            }
            _ => {
                let Some((approval, sandbox)) =
                    permissions::policies(value, self.workspace.as_ref())
                else {
                    return Some(vec![]);
                };
                self.picks.insert("approvalPolicy".into(), approval.clone());
                self.picks.insert("sandboxPolicy".into(), sandbox.clone());
                (self.approval, self.sandbox) = (approval, sandbox);
                Some(vec![self.permissions_event()])
            }
        }
    }

    fn read(
        &mut self,
        model: &Value,
        effort: &Value,
        approval: &Value,
        sandbox: &Value,
    ) -> Vec<HarnessEvent> {
        self.model = model.as_str().unwrap_or_default().into();
        self.effort = effort.as_str().map(String::from);
        self.approval = approval.clone();
        self.sandbox = sandbox.clone();
        if sandbox["type"] == "workspaceWrite" {
            self.workspace = Some(sandbox.clone());
        }
        vec![self.model_event(), self.permissions_event()]
    }

    fn current(&self) -> Option<&Model> {
        self.models.iter().find(|model| model.id == self.model)
    }

    fn name(&self) -> &str {
        self.current().map_or(&self.model, |model| &model.name)
    }

    fn model_event(&self) -> HarnessEvent {
        HarnessEvent::Model {
            id: Some(self.model.clone()).filter(|id| !id.is_empty()),
            name: self.name().into(),
            thinking: self.effort.clone(),
        }
    }

    fn permissions_event(&self) -> HarnessEvent {
        let (mode, asks) = permissions::mode(&self.approval, &self.sandbox);
        HarnessEvent::Permissions { mode, asks }
    }
}

fn list(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}
