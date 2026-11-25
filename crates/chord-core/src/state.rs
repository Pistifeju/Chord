use crate::lock;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;

use crate::{Error, ModelOption, Project, WorktreePanes};

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    pub projects: Vec<Project>,
    pub sidebar: Sidebar,
    pub appearance: Appearance,
    pub keep_awake: bool,
    pub mute_notifications: bool,
    pub links_in_default_browser: bool,
    pub push_after_commit: bool,
    #[serde(deserialize_with = "valid_shortcuts")]
    #[specta(type = BTreeMap<String, Option<Shortcut>>)]
    pub shortcuts: BTreeMap<String, Option<Shortcut>>,
    pub selected: Option<PathBuf>,
    pub panes: BTreeMap<PathBuf, WorktreePanes>,
    pub side_panels: BTreeMap<PathBuf, SidePanel>,
    pub bottom_panels: BTreeMap<PathBuf, BottomPanel>,
    pub running_panes: BTreeSet<String>,
    pub agents: BTreeMap<String, PaneAgent>,
    pub default_harness: Option<String>,
    pub models: BTreeMap<String, Vec<ModelOption>>,
    pub welcomed: bool,
    pub kept_services: BTreeMap<String, bool>,
    pub run_destinations: BTreeMap<PathBuf, RunDestination>,
    pub preview_models: BTreeMap<PathBuf, RunDestination>,
    pub scheme_picks: BTreeMap<PathBuf, SchemePick>,
    pub scheme_defaults: BTreeMap<PathBuf, SchemePick>,
    pub preview_picks: BTreeMap<PathBuf, PreviewPick>,
    pub screenshot_folder: Option<PathBuf>,
    pub worktree_order: BTreeMap<PathBuf, Vec<PathBuf>>,
}

pub fn place_in(order: &[PathBuf], path: &Path) -> usize {
    order.iter().position(|o| o == path).unwrap_or(usize::MAX)
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunDestination {
    Device {
        id: String,
        name: String,
    },
    Simulator {
        model: String,
        runtime: String,
        name: String,
        os: String,
    },
    Mac {
        name: String,
    },
}

impl RunDestination {
    pub fn name(&self) -> &str {
        match self {
            RunDestination::Device { name, .. }
            | RunDestination::Simulator { name, .. }
            | RunDestination::Mac { name } => name,
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SchemePick {
    pub scheme: Option<String>,
    pub configuration: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPick {
    pub file: String,
    pub index: u32,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PaneAgent {
    pub harness: Option<String>,
    pub session: Option<String>,
}

#[derive(Deserialize, Type, Debug)]
#[serde(rename_all = "camelCase")]
pub enum StateChange {
    SelectedWorktree(Option<PathBuf>),
    SidebarHidden(bool),
    Theme(Theme),
    TextPercent(u8),
    TerminalFontSize(u8),
    EditorFontSize(u8),
    MuteNotifications(bool),
    LinksInDefaultBrowser(bool),
    PushAfterCommit(bool),
    DefaultHarness(String),
    Shortcuts(BTreeMap<String, Option<Shortcut>>),
    Panes {
        worktree: PathBuf,
        panes: Option<WorktreePanes>,
    },
    SidePanel {
        worktree: PathBuf,
        panel: SidePanel,
    },
    BottomPanel {
        worktree: PathBuf,
        panel: BottomPanel,
    },
    KeptServices {
        ids: Vec<String>,
        kept: bool,
    },
    ScreenshotFolder(Option<PathBuf>),
    ProjectOrder(Vec<PathBuf>),
    WorktreeOrder {
        project: PathBuf,
        worktrees: Vec<PathBuf>,
    },
    RunDestination {
        project: PathBuf,
        destination: Option<RunDestination>,
    },
    PreviewModel {
        project: PathBuf,
        model: Option<RunDestination>,
    },
    PreviewPick {
        worktree: PathBuf,
        pick: PreviewPick,
    },
    SchemePick {
        project: PathBuf,
        pick: SchemePick,
    },
    SchemeDefault {
        project: PathBuf,
        pick: SchemePick,
    },
}

impl State {
    pub fn apply(&mut self, change: StateChange) {
        match change {
            StateChange::SelectedWorktree(worktree) => self.selected = worktree,
            StateChange::SidebarHidden(hidden) => self.sidebar.hidden = hidden,
            StateChange::Theme(theme) => self.appearance.theme = theme,
            StateChange::TextPercent(percent) => self.appearance.text_percent = percent,
            StateChange::TerminalFontSize(size) => self.appearance.terminal_font_size = size,
            StateChange::EditorFontSize(size) => self.appearance.editor_font_size = size,
            StateChange::MuteNotifications(mute) => self.mute_notifications = mute,
            StateChange::LinksInDefaultBrowser(outside) => self.links_in_default_browser = outside,
            StateChange::PushAfterCommit(push) => self.push_after_commit = push,
            StateChange::Shortcuts(shortcuts) => self.shortcuts = shortcuts,
            StateChange::DefaultHarness(harness) => self.default_harness = Some(harness),
            StateChange::SidePanel { worktree, panel } => {
                self.side_panels.insert(worktree, panel);
            }
            StateChange::BottomPanel { worktree, panel } => {
                self.bottom_panels.insert(worktree, panel);
            }
            StateChange::KeptServices { ids, kept } => {
                for id in ids {
                    self.kept_services.insert(id, kept);
                }
            }
            StateChange::ScreenshotFolder(folder) => self.screenshot_folder = folder,
            StateChange::ProjectOrder(order) => {
                self.projects.sort_by_key(|p| place_in(&order, &p.path));
            }
            StateChange::WorktreeOrder { project, worktrees } => {
                self.worktree_order.insert(project, worktrees);
            }
            StateChange::RunDestination {
                project,
                destination,
            } => match destination {
                Some(destination) => {
                    self.run_destinations.insert(project, destination);
                }
                None => {
                    self.run_destinations.remove(&project);
                }
            },
            StateChange::PreviewModel { project, model } => match model {
                Some(model) => {
                    self.preview_models.insert(project, model);
                }
                None => {
                    self.preview_models.remove(&project);
                }
            },
            StateChange::PreviewPick { worktree, pick } => {
                self.preview_picks.insert(worktree, pick);
            }
            StateChange::SchemePick { project, pick } => {
                if pick == SchemePick::default() {
                    self.scheme_picks.remove(&project);
                } else {
                    self.scheme_picks.insert(project, pick);
                }
            }
            StateChange::SchemeDefault { project, pick } => {
                self.scheme_defaults.insert(project, pick);
            }
            StateChange::Panes { worktree, panes } => match panes {
                Some(panes) => {
                    self.panes.insert(worktree, panes);
                }
                None => {
                    self.panes.remove(&worktree);
                }
            },
        }
    }

    pub fn forget_worktree(&mut self, worktree: &Path) {
        let panes = self
            .panes
            .remove(worktree)
            .map(|saved| saved.pane_ids())
            .unwrap_or_default();
        for pane in &panes {
            self.forget_pane(pane);
        }
        self.side_panels.remove(worktree);
        self.bottom_panels.remove(worktree);
        self.preview_picks.remove(worktree);
        for order in self.worktree_order.values_mut() {
            order.retain(|path| path != worktree);
        }
    }

    pub fn forget_pane(&mut self, pane: &str) {
        self.running_panes.remove(pane);
        self.agents.remove(pane);
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SidePanel {
    pub open: bool,
    pub width: u16,
    pub view: SideView,
}

impl Default for SidePanel {
    fn default() -> Self {
        SidePanel {
            open: false,
            width: 300,
            view: SideView::Files,
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SideView {
    #[default]
    Files,
    Simulator,
    Preview,
    Changes,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BottomPanel {
    pub open: bool,
    pub height: u16,
    pub view: BottomView,
    pub side_width: u16,
}

impl Default for BottomPanel {
    fn default() -> Self {
        BottomPanel {
            open: false,
            height: 240,
            view: BottomView::Terminal,
            side_width: 480,
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum BottomView {
    #[default]
    Terminal,
    Build,
    Console,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Sidebar {
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: Theme,
    pub text_percent: u8,
    pub terminal_font_size: u8,
    pub editor_font_size: u8,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: Theme::default(),
            text_percent: 100,
            terminal_font_size: 12,
            editor_font_size: 12,
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Shortcut {
    pub key: String,
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Shortcut {
    fn is_valid(&self) -> bool {
        let named = !self.key.is_empty()
            && self.key == self.key.to_lowercase()
            && !self.key.chars().any(char::is_whitespace);
        named && (self.cmd || self.ctrl || self.alt || self.key == "escape")
    }
}

fn valid_shortcuts<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Option<Shortcut>>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    let serde_json::Value::Object(entries) = value else {
        return Ok(BTreeMap::new());
    };
    Ok(entries
        .into_iter()
        .filter_map(|(id, saved)| {
            let shortcut: Option<Shortcut> = serde_json::from_value(saved).ok()?;
            shortcut
                .as_ref()
                .is_none_or(Shortcut::is_valid)
                .then_some((id, shortcut))
        })
        .collect())
}

pub struct Store {
    file: PathBuf,
    state: Mutex<State>,
}

impl Store {
    pub fn open(file: PathBuf) -> Result<Store, Error> {
        let state = match fs::read(&file) {
            Ok(bytes) => serde_json::from_slice(&bytes).or_else(|_| {
                fs::rename(&file, file.with_extension("json.bad"))?;
                Ok::<_, Error>(State::default())
            })?,
            Err(error) if error.kind() == ErrorKind::NotFound => State::default(),
            Err(error) => return Err(error.into()),
        };
        Ok(Store {
            file,
            state: Mutex::new(state),
        })
    }

    pub fn get(&self) -> State {
        lock(&self.state).clone()
    }

    pub fn update<T>(&self, change: impl FnOnce(&mut State) -> T) -> Result<T, Error> {
        let mut state = lock(&self.state);
        let mut next = state.clone();
        let result = change(&mut next);
        if next == *state {
            return Ok(result);
        }
        let temp = self.file.with_extension("json.tmp");
        let mut written = fs::File::create(&temp)?;
        written.write_all(&serde_json::to_vec_pretty(&next)?)?;
        written.sync_all()?;
        fs::rename(&temp, &self.file)?;
        *state = next;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_saved_shortcuts_are_dropped_and_good_ones_kept() {
        let state: State = serde_json::from_str(
            r#"{"shortcuts":{
                "a":{"key":"k","cmd":true},
                "b":{"key":"K","cmd":true},
                "c":{"key":"k"},
                "d":{"key":"k","cmd":"yes"},
                "e":null,
                "f":"cmd+k"
            }}"#,
        )
        .unwrap();
        let ids: Vec<_> = state.shortcuts.keys().cloned().collect();
        assert_eq!(ids, ["a", "e"]);
        let state: State = serde_json::from_str(r#"{"shortcuts":[1]}"#).unwrap();
        assert!(state.shortcuts.is_empty());
    }
}
