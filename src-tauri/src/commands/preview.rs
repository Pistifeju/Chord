//! Commands for a worktree's Preview view: open it on the worktree's preview simulator with its state channel, close it, list the worktree's `#Preview`s and pick one.

use std::path::PathBuf;

use chord_core::{PreviewPick, RunDestination};
use chord_ios::{Builds, PreviewState};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use super::blocking;

/// Boots the worktree's preview simulator of the destination's model and shows the picked preview there, building it unless it runs already. The channel carries the preview's states until the view opens again.
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

/// The view closed: saves stop rebuilding its preview, and its next open catches up.
#[tauri::command]
#[specta::specta]
pub fn close_preview(builds: State<Builds>, worktree: PathBuf) {
    builds.close_preview(&worktree);
}

/// Every `#Preview` in the worktree's Swift files, by file.
#[tauri::command]
#[specta::specta]
pub async fn list_previews(app: AppHandle, worktree: PathBuf) -> Result<Vec<PreviewPick>, String> {
    blocking(app, move |_, _| chord_ios::list_previews(&worktree)).await
}

/// Shows another `#Preview` in the worktree's open Preview view.
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
