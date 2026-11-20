//! The thread's settings and the slash commands: the models, the model and thinking level in effect, the permission mode, the skills, the picker menus, and the picks every later turn carries.

use serde_json::{Map, Value, json};

use super::permissions;
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent, SlashCommand, SlashKind, capitalized};

const MODEL_DIALOG: &str = "codex-model";
const THINKING_DIALOG: &str = "codex-thinking";
const PERMISSIONS_DIALOG: &str = "codex-permissions";

/// The commands of Codex itself that act on the thread: each name and its description.
const SESSION_COMMANDS: [(&str, &str); 4] = [
    ("model", "Pick the model"),
    ("thinking", "Pick the thinking level"),
    ("permissions", "Pick what Codex may do without asking"),
    ("resume", "Open another session of this worktree"),
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
    /// A skill runs: the text Codex reads and the skill's input.
    Skill { text: String, skill: Value },
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
    pub(super) fn read_models(&mut self, result: &Value) {
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
        let session = SESSION_COMMANDS.map(|(name, description)| SlashCommand {
            name: name.into(),
            description: description.into(),
            kind: SlashKind::Session,
            origin: None,
        });
        let skills = self.skills.iter().map(|skill| SlashCommand {
            name: skill.name.clone(),
            description: skill.description.clone(),
            kind: SlashKind::Skill,
            origin: None,
        });
        HarnessEvent::Commands {
            commands: session.into_iter().chain(skills).collect(),
        }
    }

    /// The thread's answer, whether started or resumed.
    pub(super) fn opened(&mut self, result: &Value) -> Vec<HarnessEvent> {
        self.read(
            &result["model"],
            &result["reasoningEffort"],
            &result["approvalPolicy"],
            &result["sandbox"],
        )
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
        let menu = |id: &str, title: &str, options: Vec<String>, current: Option<String>| {
            Command::Menu(HarnessEvent::Dialog {
                id: id.into(),
                title: title.into(),
                ask: DialogAsk::Menu {
                    options,
                    current,
                    filter: id == MODEL_DIALOG,
                },
            })
        };
        match name {
            "model" => Some(menu(
                MODEL_DIALOG,
                "Model",
                self.models.iter().map(|model| model.name.clone()).collect(),
                Some(self.name().into()),
            )),
            "thinking" => {
                let efforts = self
                    .current()
                    .map(|model| model.efforts.as_slice())
                    .unwrap_or(&[]);
                if efforts.is_empty() {
                    return Some(Command::Menu(HarnessEvent::Notice {
                        text: "The model does not think".into(),
                    }));
                }
                Some(menu(
                    THINKING_DIALOG,
                    "Thinking Level",
                    efforts.iter().map(|effort| capitalized(effort)).collect(),
                    self.effort.as_deref().map(capitalized),
                ))
            }
            "resume" => Some(Command::Resume),
            "permissions" => Some(menu(
                PERMISSIONS_DIALOG,
                "Permissions",
                permissions::MODES.map(String::from).to_vec(),
                Some(permissions::mode(&self.approval, &self.sandbox).0),
            )),
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
                let effort = match &self.effort {
                    Some(effort) if model.efforts.contains(effort) => Some(effort.clone()),
                    _ => model.default_effort.clone(),
                };
                self.model = model.id.clone();
                self.picks.insert("model".into(), json!(self.model));
                if effort != self.effort {
                    self.effort = effort;
                    self.picks.insert("effort".into(), json!(self.effort));
                }
                Some(vec![self.model_event()])
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
