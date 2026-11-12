//! Commands for the app itself: quit it, keep the Mac awake.

use chord_core::harness::sessions::Sessions;
use tauri::{AppHandle, Manager};

use super::blocking;

/// Stops every agent and ends the app once the user confirmed the quit. Panes stay saved as running, so they start again on launch.
#[tauri::command]
#[specta::specta]
pub async fn quit_app(app: AppHandle) -> Result<(), String> {
    blocking(app, |app, _| {
        app.state::<Sessions>().stop_all();
        app.exit(0);
        Ok(())
    })
    .await
}

/// Saves the setting and applies it; a missing `caffeinate` fails before anything is saved, even with no agent working.
#[tauri::command]
#[specta::specta]
pub async fn set_keep_awake(app: AppHandle, enabled: bool) -> Result<(), String> {
    blocking(app, move |app, store| {
        app.state::<Sessions>().keep_awake(enabled)?;
        store.update(|state| state.keep_awake = enabled)
    })
    .await
}
