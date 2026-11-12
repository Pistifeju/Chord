//! Commands for a pane's chat: start its agent, send a message, abort the run, stop the agent.

use std::path::PathBuf;

use chord_core::harness::pi::DEFAULT_BINARY;
use chord_core::harness::sessions::Sessions;
use chord_core::{Capability, DialogAnswer, Error, HarnessEvent, Image, images};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use super::blocking;
use crate::notifications::{self, ShownPanes};

#[tauri::command]
#[specta::specta]
pub async fn start_chat(
    app: AppHandle,
    worktree: PathBuf,
    pane: String,
    on_event: Channel<HarnessEvent>,
) -> Result<Vec<Capability>, String> {
    blocking(app, move |app, _| {
        let (notified_worktree, notified_pane) = (worktree.clone(), pane.clone());
        app.state::<Sessions>().start(
            DEFAULT_BINARY,
            worktree,
            pane,
            move |event| {
                let _ = on_event.send(event);
            },
            {
                let app = app.clone();
                move |status, last_message| {
                    notifications::announce(
                        &app,
                        notified_worktree.clone(),
                        notified_pane.clone(),
                        &status,
                        last_message,
                    );
                }
            },
        )
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn send_message(
    app: AppHandle,
    pane: String,
    text: String,
    images: Vec<Image>,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Sessions>().send(&pane, &text, &images)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn answer_dialog(
    app: AppHandle,
    pane: String,
    id: String,
    answer: DialogAnswer,
) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Sessions>().answer_dialog(&pane, &id, &answer)
    })
    .await
}

/// Asks for image files with one of the extensions; no pick gives an empty list.
#[tauri::command]
#[specta::specta]
pub async fn pick_images(app: AppHandle, extensions: Vec<String>) -> Result<Vec<PathBuf>, String> {
    blocking(app, move |app, _| {
        let extensions: Vec<&str> = extensions.iter().map(String::as_str).collect();
        app.dialog()
            .file()
            .set_title("Attach Images")
            .add_filter("Images", &extensions)
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(|file| {
                file.into_path()
                    .map_err(|error| Error::Message(error.to_string()))
            })
            .collect()
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn read_image_file(app: AppHandle, path: PathBuf) -> Result<String, String> {
    blocking(app, move |_, _| images::read_base64(&path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn abort_run(app: AppHandle, pane: String) -> Result<(), String> {
    blocking(app, move |app, _| app.state::<Sessions>().abort(&pane)).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_chat(app: AppHandle, pane: String) -> Result<(), String> {
    blocking(app, move |app, _| app.state::<Sessions>().stop_pane(&pane)).await
}

/// Tells Rust which panes the window shows, so a notification skips a pane the user is looking at.
#[tauri::command]
#[specta::specta]
pub fn show_panes(shown: State<ShownPanes>, panes: Vec<String>) {
    if let Ok(mut current) = shown.0.lock() {
        *current = panes;
    }
}
