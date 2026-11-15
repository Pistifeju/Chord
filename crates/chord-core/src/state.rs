//! The saved app state and the store that writes it to `state.json`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

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
    /// Shortcuts the user changed, by command id. `None` means the command lost its key.
    #[serde(deserialize_with = "valid_shortcuts")]
    #[specta(type = BTreeMap<String, Option<Shortcut>>)]
    pub shortcuts: BTreeMap<String, Option<Shortcut>>,
    /// The worktree open when the app last ran.
    pub selected: Option<PathBuf>,
    /// Each worktree's session tabs and split layouts.
    pub panes: BTreeMap<PathBuf, WorktreePanes>,
    /// Each worktree's side panel: open or not, and its width.
    pub side_panels: BTreeMap<PathBuf, SidePanel>,
    /// Each worktree's bottom panel: open or not, and its height.
    pub bottom_panels: BTreeMap<PathBuf, BottomPanel>,
    /// The panes whose agent was started and not stopped; they start again on launch.
    pub running_panes: BTreeSet<String>,
    /// What each pane's agent runs: its harness and the session it last held.
    pub agents: BTreeMap<String, PaneAgent>,
    /// Each project's run destination; a project without one runs on the newest iPhone simulator.
    pub run_destinations: BTreeMap<PathBuf, RunDestination>,
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
    /// `None` returns the project to the default.
    RunDestination {
        project: PathBuf,
        destination: Option<RunDestination>,
    },
}

impl State {
    pub fn apply(&mut self, change: StateChange) {
        match change {
            StateChange::SelectedWorktree(worktree) => self.selected = worktree,
            StateChange::SidebarHidden(hidden) => self.sidebar.hidden = hidden,
            StateChange::Theme(theme) => self.appearance.theme = theme,
            StateChange::TextPercent(percent) => self.appearance.set_text_percent(percent),
            StateChange::TerminalFontSize(size) => self.appearance.set_terminal_font_size(size),
            StateChange::Shortcuts(shortcuts) => self.shortcuts = shortcuts,
            StateChange::SidePanel { worktree, panel } => {
                self.side_panels.insert(worktree, panel);
            }
            StateChange::BottomPanel { worktree, panel } => {
                self.bottom_panels.insert(worktree, panel);
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
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SidePanel {
    pub open: bool,
    /// In pixels.
    pub width: u16,
}

impl Default for SidePanel {
    fn default() -> Self {
        SidePanel {
            open: false,
            width: 300,
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BottomPanel {
    /// The terminal shows.
    pub open: bool,
    /// In pixels.
    pub height: u16,
    pub view: BottomView,
}

impl Default for BottomPanel {
    fn default() -> Self {
        BottomPanel {
            open: false,
            height: 240,
            view: BottomView::Terminal,
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
    /// Text size as a percent of the default.
    pub text_percent: u8,
    /// In pixels.
    pub terminal_font_size: u8,
}

impl Appearance {
    pub const MIN_TEXT_PERCENT: u8 = 80;
    pub const MAX_TEXT_PERCENT: u8 = 150;

    pub const MIN_TERMINAL_FONT_SIZE: u8 = 9;
    pub const MAX_TERMINAL_FONT_SIZE: u8 = 24;

    fn set_text_percent(&mut self, percent: u8) {
        self.text_percent = percent.clamp(Self::MIN_TEXT_PERCENT, Self::MAX_TEXT_PERCENT);
    }

    fn set_terminal_font_size(&mut self, size: u8) {
        self.terminal_font_size =
            size.clamp(Self::MIN_TERMINAL_FONT_SIZE, Self::MAX_TERMINAL_FONT_SIZE);
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: Theme::default(),
            text_percent: 100,
            terminal_font_size: 12,
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
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn update<T>(&self, change: impl FnOnce(&mut State) -> T) -> Result<T, Error> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
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
