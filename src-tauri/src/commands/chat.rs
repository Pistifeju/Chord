//! Commands for a pane's chat: start its agent, send a message, abort the run, stop the agent.

use std::path::PathBuf;

use chord_core::harness::pi::{DEFAULT_BINARY, session_file};
use chord_core::harness::sessions::Sessions;
use chord_core::{Capability, Error, HarnessEvent, Store};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::blocking;

fn sessions_dir(app: &AppHandle) -> Result<PathBuf, Error> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| Error::Message(e.to_string()))?;
    Ok(dir.join("sessions"))
}

#[tauri::command]
#[specta::specta]
pub async fn start_chat(
    app: AppHandle,
    worktree: PathBuf,
    pane: String,
    on_event: Channel<HarnessEvent>,
) -> Result<Vec<Capability>, String> {
    blocking(app, move |app, store| {
        let dir = sessions_dir(app)?;
        let handle = app.clone();
        let exiting = pane.clone();
        let capabilities = app.state::<Sessions>().start(
            DEFAULT_BINARY,
            worktree,
            pane.clone(),
            &dir,
            move |event| {
                // An agent that ended on its own does not come back on launch.
                if matches!(event, HarnessEvent::Exited) {
                    let _ = handle
                        .state::<Store>()
                        .update(|state| state.running_panes.remove(&exiting));
                }
                let _ = on_event.send(event);
            },
        )?;
        store.update(|state| state.running_panes.insert(pane))?;
        Ok(capabilities)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn send_message(app: AppHandle, pane: String, text: String) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Sessions>().send(&pane, &text)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn abort_run(app: AppHandle, pane: String) -> Result<(), String> {
    blocking(app, move |app, _| app.state::<Sessions>().abort(&pane)).await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_chat(app: AppHandle, pane: String) -> Result<(), String> {
    blocking(app, move |app, store| {
        app.state::<Sessions>().stop_pane(&pane);
        store.update(|state| state.running_panes.remove(&pane))?;
        let _ = std::fs::remove_file(session_file(&sessions_dir(app)?, &pane)?);
        Ok(())
    })
    .await
}
