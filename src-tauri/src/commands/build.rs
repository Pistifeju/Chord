//! Commands for a worktree's build: start it with its event channel, stop it.

use std::path::PathBuf;

use chord_ios::{BuildEvent, Builds};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::blocking;

/// Builds the worktree's Xcode project. The channel carries the build's phases, log and end.
#[tauri::command]
#[specta::specta]
pub async fn start_build(
    app: AppHandle,
    worktree: PathBuf,
    on_event: Channel<BuildEvent>,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Builds>().start(worktree, move |event| {
            let _ = on_event.send(event);
        });
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_build(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Builds>().stop(&worktree);
        Ok(())
    })
    .await
}
