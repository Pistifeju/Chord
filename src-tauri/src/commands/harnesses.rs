//! Commands for the harnesses: check which ones the login shell has, their versions and sign-in states, and the default for new panes; and read each one's models.

use std::collections::BTreeMap;

use chord_core::ModelOption;
use chord_core::harness::installed::{self, Checks};
use chord_core::harness::models;
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

/// Reads every harness's models without opening a session, saves them and returns every saved list.
#[tauri::command]
#[specta::specta]
pub async fn refresh_models(app: AppHandle) -> Result<BTreeMap<String, Vec<ModelOption>>, String> {
    blocking(app, |_, store| models::refresh(store)).await
}
