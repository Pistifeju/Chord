//! Commands for a worktree's terminal: start its shell, type into it, resize it.

use std::path::PathBuf;

use chord_core::{TerminalEvent, Terminals};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn start_terminal(
    app: AppHandle,
    worktree: PathBuf,
    cols: u16,
    rows: u16,
    on_event: Channel<TerminalEvent>,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Terminals>()
            .start(worktree, cols, rows, move |event| {
                let _ = on_event.send(event);
            })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn write_terminal(app: AppHandle, worktree: PathBuf, text: String) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Terminals>().write(&worktree, &text)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn resize_terminal(
    app: AppHandle,
    worktree: PathBuf,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Terminals>().resize(&worktree, cols, rows)
    })
    .await
}
