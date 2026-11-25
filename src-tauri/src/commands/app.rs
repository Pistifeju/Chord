use chord_core::harness::sessions::Sessions;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use super::blocking;
use crate::running::Running;

fn stop_work(app: &AppHandle) {
    Running::of(app).stop_all();
    chord_ios::shut_down_all();
}

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

async fn newer_release(app: &AppHandle) -> Result<Option<Update>, String> {
    if app.config().identifier.ends_with(".dev") {
        return Ok(None);
    }
    let updater = app.updater().map_err(|error| error.to_string())?;
    updater.check().await.map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    Ok(newer_release(&app).await?.map(|update| update.version))
}

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

#[tauri::command]
#[specta::specta]
pub async fn set_keep_awake(app: AppHandle, enabled: bool) -> Result<(), String> {
    blocking(app, move |app, store| {
        app.state::<Sessions>().keep_awake(enabled)?;
        store.update(|state| state.keep_awake = enabled)
    })
    .await
}

#[derive(Serialize, specta::Type, tauri_specta::Event, Clone)]
pub struct LinkRequested(pub String);

#[tauri::command]
#[specta::specta]
pub async fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    blocking(app, move |_, _| open_in_default_app(&url)).await
}

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
