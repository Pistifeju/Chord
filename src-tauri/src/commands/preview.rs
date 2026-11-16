//! Commands for a worktree's Preview view: open it on the worktree's preview simulator with its state channel, and close it.

use std::path::PathBuf;

use chord_core::RunDestination;
use chord_ios::{Builds, PreviewState};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

use super::blocking;

/// Boots the worktree's preview simulator of the destination's model and shows the preview there, building it unless it runs already. The channel carries the preview's states until the view opens again.
#[tauri::command]
#[specta::specta]
pub async fn open_preview(
    app: AppHandle,
    worktree: PathBuf,
    destination: Option<RunDestination>,
    on_state: Channel<PreviewState>,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Builds>()
            .open_preview(worktree, destination, move |state| {
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
