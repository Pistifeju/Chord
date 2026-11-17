//! macOS notifications for an agent that is done or waits, and the click that opens its pane.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chord_core::{Status, Store, worktrees};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

/// The event a click on a notification sends the window: the worktree and the pane to open.
const OPENED_EVENT: &str = "notification-opened";
const BODY_CHARS: usize = 140;

/// The panes the window shows now, as the frontend last reported them.
#[derive(Default)]
pub struct ShownPanes(pub Mutex<Vec<String>>);

fn is_watched(app: &AppHandle, pane: &str) -> bool {
    let focused = app
        .get_webview_window("main")
        .and_then(|window| window.is_focused().ok())
        .unwrap_or(false);
    focused
        && app
            .state::<ShownPanes>()
            .0
            .lock()
            .is_ok_and(|shown| shown.iter().any(|id| id == pane))
}

/// Lets a click on a notification bring the window forward and tell it which pane to open.
pub fn listen_for_clicks(app: &AppHandle) -> tauri_plugin_notification::Result<()> {
    let handle = app.clone();
    app.notification().on_action(move |performed| {
        let extra = performed.notification().map(|shown| shown.extra());
        let field = |key: &str| {
            extra
                .and_then(|extra| extra.get(key))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };
        let (Some(worktree), Some(pane)) = (field("worktree"), field("pane")) else {
            return;
        };
        if let Some(window) = handle.get_webview_window("main") {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
        let _ = handle.emit(OPENED_EVENT, (worktree, pane));
    })
}

/// Notifies for a done or waiting agent, naming its project and branch. Never fails the agent.
pub fn announce(app: &AppHandle, worktree: PathBuf, pane: String, status: &Status, message: &str) {
    let headline = match status {
        Status::Done => "Done",
        Status::Waiting => "Awaiting input",
        _ => return,
    };
    if is_watched(app, &pane) {
        return;
    }
    let body = body(headline, message);
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<Arc<Store>>().get();
        if state.mute_notifications {
            return;
        }
        let (project, branch) = names(&state.projects, &worktree);
        let _ = app
            .notification()
            .builder()
            .title(format!("{project} · {branch}"))
            .body(body)
            .extra("worktree", worktree.to_string_lossy())
            .extra("pane", pane)
            .show();
    });
}

fn body(headline: &str, message: &str) -> String {
    let line = message.lines().find(|line| !line.trim().is_empty());
    match line {
        Some(line) => {
            let short: String = line.trim().chars().take(BODY_CHARS).collect();
            format!("{headline}: {short}")
        }
        None => headline.to_string(),
    }
}

// The owning project's name and the worktree's branch, or its folder name when it has none.
fn names(projects: &[chord_core::Project], worktree: &Path) -> (String, String) {
    for project in projects {
        let Ok(list) = worktrees::list(&project.path) else {
            continue;
        };
        if let Some(found) = list.iter().find(|candidate| candidate.path == worktree) {
            let branch = found.branch.clone().unwrap_or_else(|| folder(worktree));
            return (project.name.clone(), branch);
        }
    }
    (folder(worktree), folder(worktree))
}

fn folder(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
