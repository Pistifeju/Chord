//! Commands for a worktree's files: list one folder.

use std::path::PathBuf;

use chord_core::{FileEntry, files};

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn list_folder(
    app: tauri::AppHandle,
    worktree: PathBuf,
    folder: String,
) -> Result<Vec<FileEntry>, String> {
    blocking(app, move |_, _| files::list(&worktree, &folder)).await
}
