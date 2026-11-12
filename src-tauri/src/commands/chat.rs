//! Commands for a worktree's chat: start the agent, send a message, abort the run, stop the agent.

use std::path::PathBuf;

use chord_core::harness::pi::DEFAULT_BINARY;
use chord_core::harness::sessions::Sessions;
use chord_core::{Capability, HarnessEvent};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn start_chat(
    app: AppHandle,
    worktree: PathBuf,
    on_event: Channel<HarnessEvent>,
) -> Result<Vec<Capability>, String> {
    blocking(app, move |app, _| {
        app.state::<Sessions>()
            .start(DEFAULT_BINARY, worktree, move |event| {
                let _ = on_event.send(event);
            })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn send_message(app: AppHandle, worktree: PathBuf, text: String) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Sessions>().send(&worktree, &text)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn abort_run(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| app.state::<Sessions>().abort(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_chat(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Sessions>().stop(&worktree);
        Ok(())
    })
    .await
}
