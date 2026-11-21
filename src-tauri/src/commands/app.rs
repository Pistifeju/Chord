//! Commands for the app itself: quit it with its agents and shells, find a newer release and restart into it, keep the Mac awake, open a chat link.

use chord_core::harness::sessions::Sessions;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use super::blocking;
use crate::running::Running;

/// Stops every agent, shell and build and starts the shutdown of Chord's booted simulators. Panes stay saved as running, so they start again on launch.
fn stop_work(app: &AppHandle) {
    Running::of(app).stop_all();
    chord_ios::shut_down_all();
}

/// Stops the app's work and ends the app once the user confirmed the quit.
#[tauri::command]
#[specta::specta]
pub async fn quit_app(app: AppHandle) -> Result<(), String> {
    blocking(app, |app, _| {
        stop_work(app);
        app.exit(0);
        Ok(())
    })
    .await
}

/// The release newer than this build, if the release feed has one. A dev build has no app bundle to replace, so it never asks.
async fn newer_release(app: &AppHandle) -> Result<Option<Update>, String> {
    if app.config().identifier.ends_with(".dev") {
        return Ok(None);
    }
    let updater = app.updater().map_err(|error| error.to_string())?;
    updater.check().await.map_err(|error| error.to_string())
}

/// The version of a newer release, or none.
#[tauri::command]
#[specta::specta]
pub async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    Ok(newer_release(&app).await?.map(|update| update.version))
}

/// Downloads the newer release over the installed app, then stops the app's work and starts the new version. A failed download stops nothing. `on_percent` gets the download's percentage as it grows, when the server tells the size, and 100 once the download ended.
#[tauri::command]
#[specta::specta]
pub async fn install_update(app: AppHandle, on_percent: Channel<u8>) -> Result<(), String> {
    let Some(update) = newer_release(&app).await? else {
        return Ok(());
    };
    let mut received = 0;
    let mut sent = 0;
    update
        .download_and_install(
            |chunk, total| {
                received += chunk as u64;
                // 99 at most: 100 says the download ended, which the second closure reports.
                let percent = total
                    .map_or(0, |total| received * 100 / total.max(1))
                    .min(99) as u8;
                if percent > sent {
                    sent = percent;
                    let _ = on_percent.send(percent);
                }
            },
            || {
                let _ = on_percent.send(100);
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    blocking(app, |app, _| -> Result<(), chord_core::Error> {
        stop_work(app);
        app.restart()
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

/// A link tried to load in the app's own web view; the window opens it like a chat link.
#[derive(Serialize, specta::Type, tauri_specta::Event, Clone)]
pub struct LinkRequested(pub String);

/// Opens a link of the chat in the default app.
#[tauri::command]
#[specta::specta]
pub async fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    blocking(app, move |_, _| open_in_default_app(&url)).await
}

/// Opens a link in the default app. The text comes from agent output, so only web and mail links open.
pub fn open_in_default_app(url: &str) -> Result<(), chord_core::Error> {
    let lower = url.to_ascii_lowercase();
    if !["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
    {
        return Err(chord_core::Error::Message(format!("Cannot open {url}")));
    }
    let status = std::process::Command::new("open")
        .arg("--")
        .arg(url)
        .status()?;
    if !status.success() {
        return Err(chord_core::Error::Message(format!("Cannot open {url}")));
    }
    Ok(())
}
