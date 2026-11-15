//! Commands for the app itself: quit it with its agents and shells, keep the Mac awake, open a chat link.

use chord_core::Terminals;
use chord_core::harness::sessions::Sessions;
use chord_ios::Builds;
use tauri::{AppHandle, Manager};

use super::blocking;

/// Stops every agent, shell and build and ends the app once the user confirmed the quit. Panes stay saved as running, so they start again on launch.
#[tauri::command]
#[specta::specta]
pub async fn quit_app(app: AppHandle) -> Result<(), String> {
    blocking(app, |app, _| {
        app.state::<Sessions>().stop_all();
        app.state::<Terminals>().stop_all();
        app.state::<Builds>().stop_all();
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

/// Opens a link of the chat in the default app. The text comes from agent output, so only web and mail links open.
#[tauri::command]
#[specta::specta]
pub async fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    blocking(app, move |_, _| {
        let lower = url.to_ascii_lowercase();
        if !["http://", "https://", "mailto:"]
            .iter()
            .any(|scheme| lower.starts_with(scheme))
        {
            return Err(chord_core::Error::Message(format!("Cannot open {url}")));
        }
        let status = std::process::Command::new("open")
            .arg("--")
            .arg(&url)
            .status()?;
        if !status.success() {
            return Err(chord_core::Error::Message(format!("Cannot open {url}")));
        }
        Ok(())
    })
    .await
}
