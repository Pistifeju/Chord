use std::path::PathBuf;

use chord_core::{
    Error, GitCounts, Project, Removal, Store, Worktree,
    branches::{self, Deletion, LocalBranch},
    git_counts, projects, worktrees,
};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use super::blocking;
use crate::running::Running;

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
    blocking(app, move |app, store| {
        Running::of(app).stop_project(&project);
        projects::remove(store, &project)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_worktrees(app: AppHandle, project: PathBuf) -> Result<Vec<Worktree>, String> {
    blocking(app, move |_, store| {
        ordered_worktrees(store, &projects::find(store, &project)?)
    })
    .await
}

fn ordered_worktrees(store: &Store, project: &Project) -> Result<Vec<Worktree>, Error> {
    let order = store
        .get()
        .worktree_order
        .remove(&project.path)
        .unwrap_or_default();
    Ok(worktrees::in_order(worktrees::list(&project.path)?, &order))
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
pub async fn check_new_branch(
    app: AppHandle,
    project: PathBuf,
    name: String,
) -> Result<Option<String>, String> {
    blocking(app, move |_, store| {
        worktrees::check_new_branch(&projects::find(store, &project)?, &name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn create_branch(app: AppHandle, worktree: PathBuf, name: String) -> Result<(), String> {
    blocking(app, move |_, _| branches::create(&worktree, &name)).await
}

#[tauri::command]
#[specta::specta]
pub async fn check_branch_rename(
    app: AppHandle,
    project: PathBuf,
    branch: String,
    name: String,
) -> Result<Option<String>, String> {
    blocking(app, move |_, store| {
        worktrees::check_branch_rename(&projects::find(store, &project)?, &branch, &name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn rename_branch(
    app: AppHandle,
    project: PathBuf,
    branch: String,
    name: String,
) -> Result<(), String> {
    blocking(app, move |_, store| {
        worktrees::rename_branch(&projects::find(store, &project)?, &branch, &name)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_branches(app: AppHandle, worktree: PathBuf) -> Result<Vec<LocalBranch>, String> {
    blocking(app, move |_, _| branches::list(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn switch_branch(
    app: AppHandle,
    worktree: PathBuf,
    branch: String,
) -> Result<(), String> {
    blocking(app, move |_, _| branches::switch(&worktree, &branch)).await
}

#[tauri::command]
#[specta::specta]
pub async fn delete_branch(
    app: AppHandle,
    worktree: PathBuf,
    branch: String,
    force: bool,
) -> Result<Deletion, String> {
    blocking(app, move |_, _| branches::delete(&worktree, &branch, force)).await
}

#[tauri::command]
#[specta::specta]
pub async fn create_worktree(
    app: AppHandle,
    project: PathBuf,
    branch: String,
) -> Result<Worktree, String> {
    blocking(app, move |_, store| {
        let project = projects::find(store, &project)?;
        let mut order: Vec<PathBuf> = ordered_worktrees(store, &project)?
            .into_iter()
            .map(|w| w.path)
            .collect();
        let created = worktrees::create(&project, &branch)?;
        let new: Vec<PathBuf> = worktrees::list(&project.path)?
            .into_iter()
            .map(|w| w.path)
            .filter(|path| !order.contains(path))
            .collect();
        order.extend(new);
        store.update(|state| {
            state.worktree_order.insert(project.path.clone(), order);
        })?;
        Ok(created)
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
    blocking(app, move |app, store| {
        Running::of(app).remove_worktree(&projects::find(store, &project)?, &worktree, force)
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
