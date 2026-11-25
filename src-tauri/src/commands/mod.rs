pub mod app;
pub mod browser;
pub mod build;
pub mod changes;
pub mod chat;
pub mod files;
pub mod harnesses;
pub mod preview;
pub mod projects;
pub mod simulator;
pub mod state;
pub mod terminal;

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
