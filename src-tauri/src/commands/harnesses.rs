use std::collections::BTreeMap;

use chord_core::ModelOption;
use chord_core::harness::installed::{self, Checks};
use chord_core::harness::models;
use chord_core::harness::spend::{self, Spend};
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

#[tauri::command]
#[specta::specta]
pub async fn refresh_models(app: AppHandle) -> Result<BTreeMap<String, Vec<ModelOption>>, String> {
    blocking(app, |_, store| models::refresh(store)).await
}

#[tauri::command]
#[specta::specta]
pub async fn read_spend(app: AppHandle, since: f64) -> Result<Spend, String> {
    blocking(app, move |_, _| Ok(spend::read(since))).await
}
