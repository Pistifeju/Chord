//! Commands for a worktree's changes: list the changed files, read one file's diff.

use std::path::PathBuf;

use chord_core::{Change, DiffSide, FileDiff, changes};

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn list_changes(app: tauri::AppHandle, worktree: PathBuf) -> Result<Vec<Change>, String> {
    blocking(app, move |_, _| changes::list(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn read_diff(
    app: tauri::AppHandle,
    worktree: PathBuf,
    file: String,
    side: DiffSide,
) -> Result<FileDiff, String> {
    blocking(app, move |_, _| changes::diff(&worktree, &file, side)).await
}
