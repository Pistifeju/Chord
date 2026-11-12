//! The command that saves the shortcuts the user changed.

use std::collections::BTreeMap;

use chord_core::Shortcut;
use tauri::AppHandle;

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn set_shortcuts(
    app: AppHandle,
    shortcuts: BTreeMap<String, Option<Shortcut>>,
) -> Result<(), String> {
    blocking(app, move |_, store| {
        store.update(|state| state.shortcuts = shortcuts)
    })
    .await
}
