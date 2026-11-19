//! Commands for a worktree's changes: list the changed files, read the branch's commits and files against a base, list a commit's files and the bases to pick from, read one file's diff, stage, unstage and discard files and hunks, commit the staged files or amend the last commit, read who commits and where a push goes, push the branch, and list, make, apply, pop and drop the repository's stashes and list a stash's files.

use std::path::PathBuf;

use chord_core::{
    Change, DiffSide, FileDiff, Hunk, HunkAction,
    branch_changes::{self, BaseBranches, BranchChanges},
    changes,
    commit_diff::{self, CommittedFile},
    commits,
    commits::CommitTarget,
    stashes::{self, Stash, StashAction},
};

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn list_changes(app: tauri::AppHandle, worktree: PathBuf) -> Result<Vec<Change>, String> {
    blocking(app, move |_, _| changes::list(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn read_branch(
    app: tauri::AppHandle,
    worktree: PathBuf,
    base: Option<String>,
) -> Result<Option<BranchChanges>, String> {
    blocking(app, move |_, _| {
        branch_changes::read(&worktree, base.as_deref())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_commit_files(
    app: tauri::AppHandle,
    worktree: PathBuf,
    commit: String,
) -> Result<Vec<CommittedFile>, String> {
    blocking(app, move |_, _| {
        commit_diff::commit_files(&worktree, &commit)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn list_base_branches(
    app: tauri::AppHandle,
    worktree: PathBuf,
) -> Result<BaseBranches, String> {
    blocking(app, move |_, _| branch_changes::bases(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn read_diff(
    app: tauri::AppHandle,
    worktree: PathBuf,
    file: String,
    side: DiffSide,
) -> Result<FileDiff, String> {
    blocking(app, move |_, _| changes::diff(&worktree, &file, &side)).await
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
    staged: bool,
) -> Result<(), String> {
    blocking(app, move |_, _| changes::discard(&worktree, &files, staged)).await
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

#[tauri::command]
#[specta::specta]
pub async fn commit_staged(
    app: tauri::AppHandle,
    worktree: PathBuf,
    message: String,
    amend: bool,
) -> Result<(), String> {
    blocking(app, move |_, _| commits::commit(&worktree, &message, amend)).await
}

#[tauri::command]
#[specta::specta]
pub async fn last_commit_message(
    app: tauri::AppHandle,
    worktree: PathBuf,
) -> Result<String, String> {
    blocking(app, move |_, _| commits::last_message(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn read_commit_target(
    app: tauri::AppHandle,
    worktree: PathBuf,
) -> Result<CommitTarget, String> {
    blocking(app, move |_, _| commits::target(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn push_branch(app: tauri::AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |_, _| commits::push(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_stashes(app: tauri::AppHandle, worktree: PathBuf) -> Result<Vec<Stash>, String> {
    blocking(app, move |_, _| stashes::list(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_stash_files(
    app: tauri::AppHandle,
    worktree: PathBuf,
    stash: String,
) -> Result<Vec<CommittedFile>, String> {
    blocking(app, move |_, _| stashes::files(&worktree, &stash)).await
}

#[tauri::command]
#[specta::specta]
pub async fn stash_changes(app: tauri::AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |_, _| stashes::push(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn act_on_stash(
    app: tauri::AppHandle,
    worktree: PathBuf,
    stash: String,
    action: StashAction,
) -> Result<(), String> {
    blocking(app, move |_, _| stashes::act(&worktree, &stash, action)).await
}
