//! Commands for a worktree's files: list one folder, read a file, write a file, tell when a file changed, create, rename, trash and reveal one.

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

#[tauri::command]
#[specta::specta]
pub async fn read_file(
    app: tauri::AppHandle,
    worktree: PathBuf,
    file: String,
) -> Result<String, String> {
    blocking(app, move |_, _| files::read(&worktree, &file)).await
}

#[tauri::command]
#[specta::specta]
pub async fn file_modified(
    app: tauri::AppHandle,
    worktree: PathBuf,
    file: String,
) -> Result<String, String> {
    blocking(app, move |_, _| files::modified(&worktree, &file)).await
}

#[tauri::command]
#[specta::specta]
pub async fn write_file(
    app: tauri::AppHandle,
    worktree: PathBuf,
    file: String,
    expected: String,
    text: String,
) -> Result<(), String> {
    blocking(app, move |_, _| {
        files::write(&worktree, &file, &expected, &text)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_file(
    app: tauri::AppHandle,
    worktree: PathBuf,
    path: String,
    folder: bool,
) -> Result<(), String> {
    blocking(app, move |_, _| files::create(&worktree, &path, folder)).await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_file(
    app: tauri::AppHandle,
    worktree: PathBuf,
    from: String,
    to: String,
) -> Result<(), String> {
    blocking(app, move |_, _| files::rename(&worktree, &from, &to)).await
}

#[tauri::command]
#[specta::specta]
pub async fn trash_file(
    app: tauri::AppHandle,
    worktree: PathBuf,
    path: String,
) -> Result<(), String> {
    blocking(app, move |_, _| files::trash(&worktree, &path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn reveal_file(
    app: tauri::AppHandle,
    worktree: PathBuf,
    path: String,
) -> Result<(), String> {
    blocking(app, move |_, _| files::reveal(&worktree, &path)).await
}
