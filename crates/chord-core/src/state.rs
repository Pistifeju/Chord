//! The saved app state and the store that writes it to `state.json`.

use crate::lock;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;

use crate::{Error, Project, WorktreePanes};

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    pub projects: Vec<Project>,
    pub sidebar: Sidebar,
    pub appearance: Appearance,
    /// Keep the Mac awake while an agent runs.
    pub keep_awake: bool,
    /// No macOS notification for a done or waiting agent.
    pub mute_notifications: bool,
    /// Web links open in the default browser instead of a browser tab.
    pub links_in_default_browser: bool,
    /// Shortcuts the user changed, by command id. `None` means the command lost its key.
    #[serde(deserialize_with = "valid_shortcuts")]
    #[specta(type = BTreeMap<String, Option<Shortcut>>)]
    pub shortcuts: BTreeMap<String, Option<Shortcut>>,
    /// The worktree open when the app last ran.
    pub selected: Option<PathBuf>,
    /// Each worktree's tab groups and their split layout.
    pub panes: BTreeMap<PathBuf, WorktreePanes>,
    /// Each worktree's side panel: open or not, and its width.
    pub side_panels: BTreeMap<PathBuf, SidePanel>,
    /// Each worktree's bottom panel: open or not, and its height.
    pub bottom_panels: BTreeMap<PathBuf, BottomPanel>,
    /// The panes whose agent was started and not stopped; they start again on launch.
    pub running_panes: BTreeSet<String>,
    /// What each pane's agent runs: its harness and the session it last held.
    pub agents: BTreeMap<String, PaneAgent>,
    /// The switches of the simulator service subgroups, by subgroup id or group id: `true` keeps it on, `false` lets a boot switch it off. A group id covers the group's subgroups without a switch of their own; with neither, the group's default applies.
    pub kept_services: BTreeMap<String, bool>,
    /// Each project's run destination; a project without one runs on the newest iPhone simulator.
    pub run_destinations: BTreeMap<PathBuf, RunDestination>,
    /// Each project's preview simulator model; a project without one previews on the newest iPhone.
    pub preview_models: BTreeMap<PathBuf, RunDestination>,
    /// Each project's scheme and build configuration for ⌘R and ⌘B; a project without one builds the default scheme in its default configuration.
    pub scheme_picks: BTreeMap<PathBuf, SchemePick>,
    /// The `#Preview` each worktree's Preview view shows.
    pub preview_picks: BTreeMap<PathBuf, PreviewPick>,
    /// Where the Simulator view's screenshots and recordings go; none puts them on the Desktop.
    pub screenshot_folder: Option<PathBuf>,
    /// Each project's worktrees in the order the user dragged them to; the ones missing follow in git's order.
    pub worktree_order: BTreeMap<PathBuf, Vec<PathBuf>>,
}

/// Where `path` sits in a saved order; a path the order misses goes after every saved one.
pub fn place_in(order: &[PathBuf], path: &Path) -> usize {
    order.iter().position(|o| o == path).unwrap_or(usize::MAX)
}

/// Where ⌘R runs the app: a connected device, or a simulator model and iOS version that exist among the user's simulators.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunDestination {
    Device {
        /// The device's hardware UDID.
        id: String,
        name: String,
    },
    Simulator {
        /// The device type identifier.
        model: String,
        /// The runtime identifier.
        runtime: String,
        name: String,
        /// "iOS 27.0".
        os: String,
    },
}

impl RunDestination {
    pub fn name(&self) -> &str {
        match self {
            RunDestination::Device { name, .. } | RunDestination::Simulator { name, .. } => name,
        }
    }
}

/// The scheme and build configuration ⌘R and ⌘B build; `None` keeps the default.
#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SchemePick {
    pub scheme: Option<String>,
    pub configuration: Option<String>,
}

/// One `#Preview` of a worktree: its file and its place among the file's previews.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewPick {
    /// Relative to the worktree, `/`-separated.
    pub file: String,
    pub index: u32,
    /// The macro's name argument; none for an unnamed preview.
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PaneAgent {
    pub harness: String,
    /// What the harness opens the session by; none until it reports one.
    pub session: Option<String>,
}

impl Default for PaneAgent {
    fn default() -> Self {
        PaneAgent {
            harness: crate::harness::pi::NAME.into(),
            session: None,
        }
    }
}

/// One change the frontend saves.
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
    Shortcuts(BTreeMap<String, Option<Shortcut>>),
    /// `None` forgets the worktree's layouts.
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
    /// `ids` are subgroup ids, or a group id for a group without subgroups; all are set in one save.
    KeptServices {
        ids: Vec<String>,
        kept: bool,
    },
    ScreenshotFolder(Option<PathBuf>),
    /// The project paths in sidebar order; a project missing from it goes last.
    ProjectOrder(Vec<PathBuf>),
    WorktreeOrder {
        project: PathBuf,
        worktrees: Vec<PathBuf>,
    },
    /// `None` returns the project to the default.
    RunDestination {
        project: PathBuf,
        destination: Option<RunDestination>,
    },
    /// `None` returns the project to the default.
    PreviewModel {
        project: PathBuf,
        model: Option<RunDestination>,
    },
    PreviewPick {
        worktree: PathBuf,
        pick: PreviewPick,
    },
    /// A pick of all defaults returns the project to the default.
    SchemePick {
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
            StateChange::Shortcuts(shortcuts) => self.shortcuts = shortcuts,
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

    /// The worktree is gone: everything saved under it and its panes goes. Project-keyed picks stay.
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

    /// A pane that is gone no longer starts on launch, and its next start opens a new session.
    pub fn forget_pane(&mut self, pane: &str) {
        self.running_panes.remove(pane);
        self.agents.remove(pane);
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SidePanel {
    pub open: bool,
    /// In pixels.
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

/// The view the side panel shows.
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
    /// The terminal shows.
    pub open: bool,
    /// In pixels.
    pub height: u16,
    pub view: BottomView,
    /// The view column's width beside the terminal, in pixels.
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

/// The view beside the terminal; `Terminal` means none.
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
    /// Text size as a percent of the default. The frontend's steps (`settings/appearance.ts`) keep both sizes in range.
    pub text_percent: u8,
    /// In pixels.
    pub terminal_font_size: u8,
    /// In pixels.
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
    /// The lowercase `KeyboardEvent.key`.
    pub key: String,
    pub cmd: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Shortcut {
    /// A key that is not typing: a lowercase, visible key with cmd, ctrl or alt, or escape alone.
    fn is_valid(&self) -> bool {
        let named = !self.key.is_empty()
            && self.key == self.key.to_lowercase()
            && !self.key.chars().any(char::is_whitespace);
        named && (self.cmd || self.ctrl || self.alt || self.key == "escape")
    }
}

/// Entries that are not valid shortcuts are dropped, so those commands load with their defaults.
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
                // A file Chord cannot read stays beside the new one instead of being overwritten.
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
        // shortcut: writes on every change, debounce once a feature saves while you type or drag.
        let temp = self.file.with_extension("json.tmp");
        let mut written = fs::File::create(&temp)?;
        written.write_all(&serde_json::to_vec_pretty(&next)?)?;
        // On disk before the rename, so a power loss never leaves an empty state file.
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
