//! Commands for the simulators and the build data of the worktrees: the running simulators, shutting the others down, the build data's size and its prune, and the Simulator view's screen and input.

use std::path::PathBuf;

use serde::Serialize;
use chord_core::{Error, StateChange};
use chord_ios::{
    Bezel, Button, Input, RunningSimulator, ScreenButtons, ScreenRect, ScreenState, Screenshot,
    ServiceRow, SimulatorAppearance, SimulatorAppearanceChange,
};
use tauri::ipc::Channel;
use tauri::{AppHandle, Webview, Window};
use tauri_plugin_dialog::DialogExt;
use tauri_specta::Event;

use super::blocking;

/// The booted simulators Chord created; `open` marks the open worktree's.
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

/// A simulator was added, removed, booted or shut down: the window reads the running ones again.
#[derive(Serialize, specta::Type, tauri_specta::Event, Clone)]
pub struct SimulatorsChanged;

/// Sends `SimulatorsChanged` from now on. False when the simulator frameworks did not load, so the window asks on a timer instead.
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

/// Shuts down every booted simulator Chord created except the worktree's own.
#[tauri::command]
#[specta::specta]
pub async fn shut_down_other_simulators(app: AppHandle, keep: PathBuf) -> Result<(), String> {
    blocking(app, move |_, _| chord_ios::shut_down_others(&keep)).await
}

/// The disk the DerivedData folders and simulators of the created worktrees take, in bytes, without the main checkouts'; a number the bindings can carry exactly up to 8 PB.
#[tauri::command]
#[specta::specta]
pub async fn build_data_size(app: AppHandle) -> Result<f64, String> {
    blocking(app, |_, store| {
        Ok(chord_ios::build_data_size(&store.get().projects)? as f64)
    })
    .await
}

/// Removes the build data of worktrees that are gone. It does nothing while any project's worktrees cannot be listed: a project that is not mounted must not look empty.
#[tauri::command]
#[specta::specta]
pub async fn prune_build_data(app: AppHandle) -> Result<(), String> {
    blocking(app, |_, store| chord_ios::prune(&store.get().projects)).await
}

/// The service groups Settings lists, each with whether it stays on.
#[tauri::command]
#[specta::specta]
pub async fn simulator_service_groups(app: AppHandle) -> Result<Vec<ServiceRow>, String> {
    blocking(app, |_, store| {
        Ok(chord_ios::service_rows(&store.get().kept_services))
    })
    .await
}

/// Shows the worktree's simulator, or with `preview` its preview simulator, under the webview at `rect`, the screen's CSS pixels in the window, in `bezel`; the channel carries its state.
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
    let unavailable = tauri::async_runtime::spawn_blocking(chord_ios::prepare_screen)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(state) = unavailable {
        return on_state.send(state).map_err(|error| error.to_string());
    }
    window
        .with_webview(move |webview| {
            let send = move |state| {
                let _ = on_state.send(state);
            };
            // SAFETY: Tauri hands over the window's live WKWebView on the main thread.
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

/// Draws Home, Lock and Shake natively under the screen, as the view laid them out.
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

/// Sends a touch or key of the Simulator view to the worktree's simulator, in order: the main thread runs them one by one. Buttons go through `press_simulator_button`.
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

/// Presses Home, Lock or Shake on the worktree's simulator. Shake, and Home before CoreSimulator 1155.4, run a tool; its error comes back.
#[tauri::command]
#[specta::specta]
pub async fn press_simulator_button(
    app: AppHandle,
    worktree: PathBuf,
    button: Button,
) -> Result<(), String> {
    let (sent, run) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = sent.send(chord_ios::send_input(
            &worktree,
            Input::Button { button },
        ));
    })
    .map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn_blocking(move || match run.recv() {
        Ok(Some(blocking)) => blocking.run().map_err(|error| error.to_string()),
        _ => Ok(()),
    })
    .await
    .map_err(|error| error.to_string())?
}

/// The Simulator view's fallback when the private API did not load.
#[tauri::command]
#[specta::specta]
pub async fn open_simulator_app(app: AppHandle) -> Result<(), String> {
    blocking(app, |_, _| chord_ios::open_app(true)).await
}

/// Takes a screenshot of the worktree's simulator, or with `preview` its preview simulator, into the folder of Settings, else the Desktop.
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

/// Records the worktree's simulator, or with `preview` its preview simulator, into the folder of Settings, else the Desktop: `on_started` once it records, then the video and its preview when the recording ends.
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

/// Stops the recording of the worktree's simulator, or with `preview` its preview simulator; `record_simulator` returns the video.
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

/// The worktree's booted simulator's appearance settings, or with `preview` its preview simulator's, and the choices the Mac offers.
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

/// Changes one appearance setting of the worktree's booted simulator, or with `preview` its preview simulator.
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

/// Asks for the folder of screenshots and recordings and saves the pick; no pick keeps the setting and returns `None`.
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

/// Starts a native drag of a file, shown as `preview`, out of the window: Finder and other apps take it, and so does a chat pane.
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
