use std::path::PathBuf;

use chord_core::{Error, StateChange};
use chord_ios::{
    Bezel, Button, Input, RunningSimulator, ScreenButtons, ScreenRect, ScreenState, Screenshot,
    ServiceRow, SimulatorAppearance, SimulatorAppearanceChange,
};
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Webview, Window};
use tauri_plugin_dialog::DialogExt;
use tauri_specta::Event;

use super::blocking;

#[tauri::command]
#[specta::specta]
pub async fn running_simulators(
    app: AppHandle,
    open: Option<PathBuf>,
) -> Result<Vec<RunningSimulator>, String> {
    blocking(app, move |_, _| {
        chord_ios::running_simulators(open.as_deref())
    })
    .await
}

#[derive(Serialize, specta::Type, tauri_specta::Event, Clone)]
pub struct SimulatorsChanged;

#[tauri::command]
#[specta::specta]
pub async fn watch_simulators(app: AppHandle) -> Result<bool, String> {
    let unavailable = tauri::async_runtime::spawn_blocking(chord_ios::prepare_screen)
        .await
        .map_err(|error| error.to_string())?;
    if unavailable.is_some() {
        return Ok(false);
    }
    let (sent, watching) = std::sync::mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = sent.send(chord_ios::watch_simulators(move || {
            let _ = SimulatorsChanged.emit(&handle);
        }));
    })
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || watching.recv().unwrap_or(false))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn shut_down_other_simulators(app: AppHandle, keep: PathBuf) -> Result<(), String> {
    blocking(app, move |_, _| chord_ios::shut_down_others(&keep)).await
}

#[tauri::command]
#[specta::specta]
pub async fn build_data_size(app: AppHandle) -> Result<f64, String> {
    blocking(app, |_, store| {
        Ok(chord_ios::build_data_size(&store.get().projects)? as f64)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn prune_build_data(app: AppHandle) -> Result<(), String> {
    blocking(app, |_, store| chord_ios::prune(&store.get().projects)).await
}

#[tauri::command]
#[specta::specta]
pub async fn simulator_service_groups(app: AppHandle) -> Result<Vec<ServiceRow>, String> {
    blocking(app, |_, store| {
        Ok(chord_ios::service_rows(&store.get().kept_services))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn show_simulator_screen(
    window: Webview,
    worktree: PathBuf,
    preview: bool,
    rect: ScreenRect,
    bezel: Bezel,
    on_state: Channel<ScreenState>,
) -> Result<(), String> {
    let turn = chord_ios::begin_show(rect, bezel);
    if !preview
        && let Some(state) = tauri::async_runtime::spawn_blocking(chord_ios::prepare_screen)
            .await
            .map_err(|error| error.to_string())?
    {
        return on_state.send(state).map_err(|error| error.to_string());
    }
    window
        .with_webview(move |webview| {
            let send = move |state| {
                let _ = on_state.send(state);
            };
            unsafe { chord_ios::show_screen(webview.inner(), turn, &worktree, preview, send) };
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn move_simulator_screen(app: AppHandle, rect: ScreenRect, bezel: Bezel) -> Result<(), String> {
    app.run_on_main_thread(chord_ios::move_screen(rect, bezel))
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn set_simulator_buttons(app: AppHandle, buttons: ScreenButtons) -> Result<(), String> {
    app.run_on_main_thread(move || chord_ios::set_buttons(buttons))
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn hide_simulator_screen(app: AppHandle) -> Result<(), String> {
    app.run_on_main_thread(chord_ios::hide_screen())
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn send_simulator_input(app: AppHandle, worktree: PathBuf, input: Input) -> Result<(), String> {
    app.run_on_main_thread(move || {
        if let Some(blocking) = chord_ios::send_input(&worktree, input) {
            tauri::async_runtime::spawn_blocking(move || blocking.run());
        }
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn press_simulator_button(
    app: AppHandle,
    worktree: PathBuf,
    button: Button,
) -> Result<(), String> {
    let (sent, run) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sent.send(chord_ios::send_input(&worktree, Input::Button { button }));
    })
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || match run.recv() {
        Ok(Some(blocking)) => blocking.run().map_err(|error| error.to_string()),
        _ => Ok(()),
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
#[specta::specta]
pub async fn open_simulator_app(app: AppHandle) -> Result<(), String> {
    blocking(app, |_, _| chord_ios::open_app(true)).await
}

#[tauri::command]
#[specta::specta]
pub async fn take_simulator_screenshot(
    app: AppHandle,
    worktree: PathBuf,
    preview: bool,
) -> Result<Screenshot, String> {
    blocking(app, move |_, store| {
        let folder = store.get().screenshot_folder;
        chord_ios::take_screenshot(&worktree, preview, folder.as_deref())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn record_simulator(
    app: AppHandle,
    worktree: PathBuf,
    preview: bool,
    on_started: Channel<()>,
) -> Result<Screenshot, String> {
    blocking(app, move |_, store| {
        let folder = store.get().screenshot_folder;
        chord_ios::record(&worktree, preview, folder.as_deref(), &|| {
            let _ = on_started.send(());
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn stop_simulator_recording(
    app: AppHandle,
    worktree: PathBuf,
    preview: bool,
) -> Result<(), String> {
    blocking(app, move |_, _| {
        chord_ios::stop_recording(&worktree, preview);
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn simulator_appearance(
    app: AppHandle,
    worktree: PathBuf,
    preview: bool,
) -> Result<SimulatorAppearance, String> {
    blocking(app, move |_, _| {
        chord_ios::read_appearance(&worktree, preview)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn set_simulator_appearance(
    app: AppHandle,
    worktree: PathBuf,
    preview: bool,
    change: SimulatorAppearanceChange,
) -> Result<(), String> {
    blocking(app, move |_, _| {
        chord_ios::set_appearance(&worktree, preview, &change)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn pick_screenshot_folder(app: AppHandle) -> Result<Option<PathBuf>, String> {
    blocking(app, |app, store| {
        let Some(folder) = app
            .dialog()
            .file()
            .set_title("Screenshots and Recordings Folder")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let folder = folder
            .into_path()
            .map_err(|error| Error::Message(error.to_string()))?;
        store.update(|state| state.apply(StateChange::ScreenshotFolder(Some(folder.clone()))))?;
        Ok(Some(folder))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn drag_file_out(
    app: AppHandle,
    window: Window,
    file: PathBuf,
    preview: PathBuf,
) -> Result<(), String> {
    let (sent, started) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sent.send(drag::start_drag(
            &window,
            drag::DragItem::Files(vec![file]),
            drag::Image::File(preview),
            |_, _| {},
            drag::Options::default(),
        ));
    })
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || started.recv())
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}
