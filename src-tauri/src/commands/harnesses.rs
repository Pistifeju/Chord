//! Commands for the harnesses: check which ones the login shell has, their versions and sign-in states, and the default for new panes.

use chord_core::harness::installed::{self, Checks};
use tauri::AppHandle;

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn check_harnesses(app: AppHandle) -> Result<Checks, String> {
    blocking(app, |_, store| {
        Ok(installed::checks(store.get().default_harness.as_deref()))
    })
    .await
}
