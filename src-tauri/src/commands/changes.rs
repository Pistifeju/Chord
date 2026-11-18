//! Commands for a worktree's changes: list the changed files, read one file's diff, stage, unstage and discard files and hunks.

use std::path::PathBuf;

use chord_core::{Change, DiffSide, FileDiff, Hunk, HunkAction, changes};

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

#[tauri::command]
#[specta::specta]
pub async fn stage_files(
    app: tauri::AppHandle,
    worktree: PathBuf,
    files: Vec<String>,
) -> Result<(), String> {
    blocking(app, move |_, _| changes::stage(&worktree, &files)).await
}

#[tauri::command]
#[specta::specta]
pub async fn unstage_files(
    app: tauri::AppHandle,
    worktree: PathBuf,
    files: Vec<String>,
) -> Result<(), String> {
    blocking(app, move |_, _| changes::unstage(&worktree, &files)).await
}

#[tauri::command]
#[specta::specta]
pub async fn discard_files(
    app: tauri::AppHandle,
    worktree: PathBuf,
    files: Vec<String>,
) -> Result<(), String> {
    blocking(app, move |_, _| changes::discard(&worktree, &files)).await
}

#[tauri::command]
#[specta::specta]
pub async fn apply_hunk(
    app: tauri::AppHandle,
    worktree: PathBuf,
    file: String,
    hunk: Hunk,
    action: HunkAction,
) -> Result<(), String> {
    blocking(app, move |_, _| {
        changes::apply_hunk(&worktree, &file, &hunk, action)
    })
    .await
}
