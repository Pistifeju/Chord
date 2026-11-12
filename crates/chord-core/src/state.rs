//! The saved app state and the store that writes it to `state.json`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
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
    /// The panes whose agent was started and not stopped; they start again on launch.
    pub running_panes: BTreeSet<String>,
}

/// One change the frontend saves.
#[derive(Deserialize, Type, Debug)]
#[serde(rename_all = "camelCase")]
pub enum StateChange {
    SelectedWorktree(Option<PathBuf>),
    SidebarHidden(bool),
    ProjectFolded {
        project: PathBuf,
        folded: bool,
    },
    Theme(Theme),
    TextPercent(u8),
    Shortcuts(BTreeMap<String, Option<Shortcut>>),
    /// `None` forgets the worktree's layouts.
    Panes {
        worktree: PathBuf,
        panes: Option<WorktreePanes>,
    },
}

impl State {
    pub fn apply(&mut self, change: StateChange) {
        match change {
            StateChange::SelectedWorktree(worktree) => self.selected = worktree,
            StateChange::SidebarHidden(hidden) => self.sidebar.hidden = hidden,
            StateChange::ProjectFolded { project, folded } => {
                self.sidebar.set_folded(project, folded)
            }
            StateChange::Theme(theme) => self.appearance.theme = theme,
            StateChange::TextPercent(percent) => self.appearance.set_text_percent(percent),
            StateChange::Shortcuts(shortcuts) => self.shortcuts = shortcuts,
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

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Sidebar {
    pub hidden: bool,
    pub folded: Vec<PathBuf>,
}

impl Sidebar {
    fn set_folded(&mut self, project: PathBuf, folded: bool) {
        self.folded.retain(|path| *path != project);
        if folded {
            self.folded.push(project);
        }
    }
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
}

impl Appearance {
    pub const MIN_TEXT_PERCENT: u8 = 80;
    pub const MAX_TEXT_PERCENT: u8 = 150;

    fn set_text_percent(&mut self, percent: u8) {
        self.text_percent = percent.clamp(Self::MIN_TEXT_PERCENT, Self::MAX_TEXT_PERCENT);
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: Theme::default(),
            text_percent: 100,
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
        // shortcut: writes on every change, debounce once a feature saves while you type or drag.
        let temp = self.file.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_vec_pretty(&next)?)?;
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
