use std::path::PathBuf;

use chord_core::{RunDestination, SchemePick};
use chord_ios::{AgentBuild, AgentTools, BuildEvent, Builds, Destinations, MacSupport, Schemes};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager};

use super::blocking;

#[derive(Serialize, specta::Type, tauri_specta::Event, Clone)]
pub struct AgentBuildRequested(pub AgentBuild);

#[tauri::command]
#[specta::specta]
pub async fn start_build(
    app: AppHandle,
    worktree: PathBuf,
    run: bool,
    destination: Option<RunDestination>,
    pick: SchemePick,
    agent_request: Option<u32>,
    on_event: Channel<BuildEvent>,
) -> Result<(), String> {
    blocking(app, move |app, store| {
        let agent = agent_request.and_then(|request| app.state::<AgentTools>().take(request));
        let kept_services = store.get().kept_services;
        app.state::<Builds>().start(
            worktree,
            run,
            destination,
            pick,
            kept_services,
            move |event| {
                if let Some(agent) = &agent {
                    let _ = agent.send(event.clone());
                }
                let _ = on_event.send(event);
            },
        );
        Ok(())
    })
    .await
}

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

#[tauri::command]
#[specta::specta]
pub async fn clean_build(app: AppHandle, worktree: PathBuf) -> Result<(), String> {
    blocking(app, move |app, _| app.state::<Builds>().clean(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_destinations(app: AppHandle) -> Result<Destinations, String> {
    blocking(app, |_, _| chord_ios::list_destinations()).await
}

#[tauri::command]
#[specta::specta]
pub async fn list_schemes(app: AppHandle, worktree: PathBuf) -> Result<Schemes, String> {
    blocking(app, move |_, _| chord_ios::list_schemes(&worktree)).await
}

#[tauri::command]
#[specta::specta]
pub async fn mac_support(app: AppHandle, worktree: PathBuf) -> Result<MacSupport, String> {
    blocking(app, move |_, _| Ok(chord_ios::mac_support(&worktree))).await
}
