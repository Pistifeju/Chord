//! Tauri commands, one file per feature area, and the thread they run core calls on.

pub mod app;
pub mod chat;
pub mod projects;
pub mod state;

use std::sync::Arc;

use chord_core::{Error, Store};
use tauri::{AppHandle, Manager};

pub async fn blocking<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&AppHandle, &Store) -> Result<T, Error> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || work(&app, &app.state::<Arc<Store>>()))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}
