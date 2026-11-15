//! Commands for the simulators and the build data of the worktrees: the running simulators, shutting the others down, the build data's size and its prune.

use std::path::PathBuf;

use chord_core::{Error, Store, worktrees};
use chord_ios::RunningSimulator;
use tauri::AppHandle;

use super::blocking;

/// The booted simulators Chord created; `open` marks the open worktree's.
#[tauri::command]
#[specta::specta]
pub async fn running_simulators(
    app: AppHandle,
    open: Option<PathBuf>,
) -> Result<Vec<RunningSimulator>, String> {
    blocking(app, move |_, _| {
        chord_ios::running_simulators(open.as_deref())
    })
    .await
}

/// Shuts down every booted simulator Chord created except the worktree's own.
#[tauri::command]
#[specta::specta]
pub async fn shut_down_other_simulators(app: AppHandle, keep: PathBuf) -> Result<(), String> {
    blocking(app, move |_, _| chord_ios::shut_down_others(&keep)).await
}

/// The disk the DerivedData folders and simulators of the created worktrees take, in bytes, without the main checkouts'; a number the bindings can carry exactly up to 8 PB.
#[tauri::command]
#[specta::specta]
pub async fn build_data_size(app: AppHandle) -> Result<f64, String> {
    blocking(app, |_, store| {
        Ok(chord_ios::build_data_size(&live_worktrees(store, false)?) as f64)
    })
    .await
}

/// Removes the build data of worktrees that are gone. It does nothing while any project's worktrees cannot be listed: a project that is not mounted must not look empty.
#[tauri::command]
#[specta::specta]
pub async fn prune_build_data(app: AppHandle) -> Result<(), String> {
    blocking(app, |_, store| {
        let live = live_worktrees(store, true)?;
        let folders: Vec<PathBuf> = store
            .get()
            .projects
            .iter()
            .map(worktrees::worktrees_folder)
            .collect();
        chord_ios::prune(&folders, &live);
        Ok(())
    })
    .await
}

fn live_worktrees(store: &Store, with_main: bool) -> Result<Vec<PathBuf>, Error> {
    let mut live = Vec::new();
    for project in store.get().projects {
        live.extend(
            worktrees::list(&project.path)?
                .into_iter()
                .map(|worktree| worktree.path)
                .filter(|path| with_main || *path != project.path),
        );
    }
    Ok(live)
}
