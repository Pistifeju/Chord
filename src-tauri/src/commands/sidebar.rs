//! Commands that save the sidebar's shown state and its folded projects.

use std::path::PathBuf;

use tauri::AppHandle;

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn set_sidebar_hidden(app: AppHandle, hidden: bool) -> Result<(), String> {
    blocking(app, move |_, store| {
        store.update(|state| state.sidebar.hidden = hidden)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_project_folded(
    app: AppHandle,
    project: PathBuf,
    folded: bool,
) -> Result<Vec<PathBuf>, String> {
    blocking(app, move |_, store| {
        store.update(|state| state.sidebar.set_folded(project, folded))
    })
    .await
}
