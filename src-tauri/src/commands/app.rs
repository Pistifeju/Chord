//! Commands for the app itself: quit it.

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
