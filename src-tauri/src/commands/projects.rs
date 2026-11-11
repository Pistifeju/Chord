//! Commands for projects and their worktrees: load, add, remove, list, create, delete, git counts.

use std::path::PathBuf;

use chord_core::{
    Error, GitCounts, Project, Removal, State, Worktree, git_counts, projects, worktrees,
};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn load_state(app: AppHandle) -> Result<State, String> {
    blocking(app, |_, store| Ok(store.get())).await
}

#[tauri::command]
#[specta::specta]
pub async fn pick_project(app: AppHandle) -> Result<Option<Project>, String> {
    blocking(app, |app, store| {
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Add Project")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let folder = folder
            .into_path()
            .map_err(|error| Error::Message(error.to_string()))?;
        projects::add(store, &folder).map(Some)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn add_project(app: AppHandle, folder: PathBuf) -> Result<Project, String> {
    blocking(app, move |_, store| projects::add(store, &folder)).await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_project(app: AppHandle, project: PathBuf) -> Result<(), String> {
    blocking(app, move |_, store| projects::remove(store, &project)).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_worktrees(app: AppHandle, project: PathBuf) -> Result<Vec<Worktree>, String> {
    blocking(app, move |_, store| {
        worktrees::list(&projects::find(store, &project)?.path)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn check_branch_name(
    app: AppHandle,
    project: PathBuf,
    name: String,
) -> Result<Option<String>, String> {
    blocking(app, move |_, store| {
        worktrees::check_branch(&projects::find(store, &project)?, &name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_worktree(
    app: AppHandle,
    project: PathBuf,
    branch: String,
) -> Result<Worktree, String> {
    blocking(app, move |_, store| {
        worktrees::create(&projects::find(store, &project)?, &branch)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn remove_worktree(
    app: AppHandle,
    project: PathBuf,
    worktree: PathBuf,
    force: bool,
) -> Result<Removal, String> {
    blocking(app, move |_, store| {
        worktrees::remove(&projects::find(store, &project)?, &worktree, force)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn read_git_counts(
    app: AppHandle,
    worktrees: Vec<PathBuf>,
) -> Result<Vec<GitCounts>, String> {
    blocking(app, move |_, _| git_counts::read_all(&worktrees)).await
}
