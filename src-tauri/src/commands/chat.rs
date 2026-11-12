//! Commands for a worktree's chat: start the agent, send a message, abort the run, stop the agent.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use chord_core::harness::pi::{DEFAULT_BINARY, Session};
use chord_core::{Capability, Error, HarnessEvent};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::blocking;

/// The running agents, one per worktree until panes arrive.
#[derive(Default)]
pub struct Chats(Mutex<HashMap<PathBuf, Session>>);

impl Chats {
    fn sessions(&self) -> std::sync::MutexGuard<'_, HashMap<PathBuf, Session>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn with(
        &self,
        worktree: &PathBuf,
        act: impl FnOnce(&Session) -> Result<(), Error>,
    ) -> Result<(), Error> {
        match self.sessions().get(worktree) {
            Some(session) => act(session),
            None => Err(Error::Message("The agent is not running".into())),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn start_chat(
    app: AppHandle,
    worktree: PathBuf,
    on_event: Channel<HarnessEvent>,
) -> Result<Vec<Capability>, String> {
    blocking(app, move |app, _| {
        let session = Session::start(DEFAULT_BINARY, &worktree, move |event| {
            let _ = on_event.send(event);
        })?;
        let capabilities = session.capabilities();
        // The lock is free again before the old session drops: dropping kills pi and waits for it.
        let replaced = app.state::<Chats>().sessions().insert(worktree, session);
        drop(replaced);
        Ok(capabilities)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn send_message(app: AppHandle, worktree: PathBuf, text: String) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Chats>()
            .with(&worktree, |session| session.send(&text))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn abort_run(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Chats>().with(&worktree, Session::abort)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_chat(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| {
        let session = app.state::<Chats>().sessions().remove(&worktree);
        drop(session);
        Ok(())
    })
    .await
}
