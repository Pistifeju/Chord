use std::path::PathBuf;

use chord_core::{PreviewPick, RunDestination};
use chord_ios::{Builds, PreviewState};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn open_preview(
    app: AppHandle,
    worktree: PathBuf,
    destination: Option<RunDestination>,
    pick: Option<PreviewPick>,
    on_state: Channel<PreviewState>,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Builds>()
            .open_preview(worktree, destination, pick, move |state| {
                let _ = on_state.send(state);
            });
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub fn close_preview(builds: State<Builds>, worktree: PathBuf) {
    builds.close_preview(&worktree);
}

#[tauri::command]
#[specta::specta]
pub async fn list_previews(app: AppHandle, worktree: PathBuf) -> Result<Vec<PreviewPick>, String> {
    blocking(app, move |_, _| chord_ios::list_previews(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn pick_preview(
    app: AppHandle,
    worktree: PathBuf,
    pick: PreviewPick,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Builds>().pick_preview(&worktree, pick);
        Ok(())
    })
    .await
}
