//! Commands that save the theme and the text size.

use chord_core::Theme;
use tauri::AppHandle;

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn set_theme(app: AppHandle, theme: Theme) -> Result<(), String> {
    blocking(app, move |_, store| {
        store.update(|state| state.appearance.theme = theme)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_text_percent(app: AppHandle, percent: u8) -> Result<u8, String> {
    blocking(app, move |_, store| {
        store.update(|state| state.appearance.set_text_percent(percent))
    })
    .await
}
