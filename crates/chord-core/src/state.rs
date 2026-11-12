//! The saved app state and the store that writes it to `state.json`.

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{Error, Project};

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct State {
    pub projects: Vec<Project>,
    pub sidebar: Sidebar,
    /// The worktree open when the app last ran.
    pub selected: Option<PathBuf>,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Sidebar {
    pub hidden: bool,
    pub folded: Vec<PathBuf>,
}

impl Sidebar {
    pub fn set_folded(&mut self, project: PathBuf, folded: bool) -> Vec<PathBuf> {
        self.folded.retain(|path| *path != project);
        if folded {
            self.folded.push(project);
        }
        self.folded.clone()
    }
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
