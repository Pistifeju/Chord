//! Commands for a worktree's build: start it with its event channel, stop it, note it opened, and list the run destinations.

use std::path::PathBuf;

use serde::Serialize;
use chord_core::RunDestination;
use chord_ios::{AgentBuild, AgentTools, BuildEvent, Builds, Destinations};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::blocking;

/// The window starts an agent's build or run on this event, like ⌘B or ⌘R.
#[derive(Serialize, specta::Type, tauri_specta::Event, Clone)]
pub struct AgentBuildRequested(pub AgentBuild);

/// Builds the worktree's Xcode project, and with `run` runs the app on the destination. The channel carries the build's phases, log and end. `agent_request` is the agent's tool call the build answers.
#[tauri::command]
#[specta::specta]
pub async fn start_build(
    app: AppHandle,
    worktree: PathBuf,
    run: bool,
    destination: Option<RunDestination>,
    agent_request: Option<u32>,
    on_event: Channel<BuildEvent>,
) -> Result<(), String> {
    blocking(app, move |app, store| {
        let agent = agent_request.and_then(|request| app.state::<AgentTools>().take(request));
        let kept_services = store.get().kept_services;
        app.state::<Builds>()
            .start(worktree, run, destination, kept_services, move |event| {
                if let Some(agent) = &agent {
                    let _ = agent.send(event.clone());
                }
                let _ = on_event.send(event);
            });
        Ok(())
    })
    .await
}

/// The user opened the worktree in the window; the oldest opened worktree's simulator is the first one shut down for room.
#[tauri::command]
#[specta::specta]
pub async fn worktree_opened(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Builds>().opened(&worktree);
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_build(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| {
        app.state::<Builds>().stop(&worktree);
        Ok(())
    })
    .await
}

/// The connected devices and the simulator models to run on.
#[tauri::command]
#[specta::specta]
pub async fn list_destinations(app: AppHandle) -> Result<Destinations, String> {
    blocking(app, |_, _| chord_ios::list_destinations()).await
}
