use chord_core::{State, StateChange};
use tauri::AppHandle;

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn load_state(app: AppHandle) -> Result<State, String> {
    blocking(app, |_, store| Ok(store.get())).await
}

#[tauri::command]
#[specta::specta]
pub async fn change_state(app: AppHandle, change: StateChange) -> Result<(), String> {
    blocking(app, move |_, store| {
        store.update(|state| state.apply(change))
    })
    .await
}
