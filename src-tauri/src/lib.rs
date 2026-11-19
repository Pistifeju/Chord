//! App entry: builds the Tauri app, lists its commands and runs it.

mod commands;
mod notifications;
mod running;

use std::path::PathBuf;
use std::sync::Arc;

use chord_core::harness::sessions::Sessions;
use chord_core::{Store, Terminals};
use chord_ios::{AgentTools, Builds};
use tauri::{Emitter, Manager};
use tauri_specta::{Event, collect_commands, collect_events};

pub fn ipc() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::new()
        .events(collect_events![
            commands::build::AgentBuildRequested,
            commands::simulator::SimulatorsChanged,
            commands::app::LinkRequested,
            commands::browser::BrowserChanged
        ])
        .commands(collect_commands![
            commands::state::load_state,
            commands::state::change_state,
            commands::projects::pick_project,
            commands::projects::add_project,
            commands::projects::remove_project,
            commands::projects::list_worktrees,
            commands::projects::check_branch_name,
            commands::projects::check_branch_rename,
            commands::projects::rename_branch,
            commands::projects::create_worktree,
            commands::projects::remove_worktree,
            commands::projects::read_git_counts,
            commands::chat::start_chat,
            commands::chat::send_message,
            commands::chat::answer_dialog,
            commands::chat::pick_images,
            commands::chat::read_image_file,
            commands::chat::abort_run,
            commands::chat::stop_chat,
            commands::chat::show_panes,
            commands::files::list_folder,
            commands::files::list_files,
            commands::files::read_file,
            commands::files::write_file,
            commands::files::file_modified,
            commands::files::create_file,
            commands::files::rename_file,
            commands::files::trash_file,
            commands::files::reveal_file,
            commands::changes::list_changes,
            commands::changes::read_branch,
            commands::changes::list_commit_files,
            commands::changes::list_base_branches,
            commands::changes::read_diff,
            commands::changes::stage_files,
            commands::changes::unstage_files,
            commands::changes::discard_files,
            commands::changes::apply_hunk,
            commands::changes::commit_staged,
            commands::changes::read_commit_target,
            commands::changes::last_commit_message,
            commands::changes::push_branch,
            commands::terminal::start_terminal,
            commands::terminal::write_terminal,
            commands::terminal::resize_terminal,
            commands::build::start_build,
            commands::build::stop_build,
            commands::build::clean_build,
            commands::build::worktree_opened,
            commands::build::list_destinations,
            commands::build::list_schemes,
            commands::preview::open_preview,
            commands::preview::close_preview,
            commands::preview::list_previews,
            commands::preview::pick_preview,
            commands::simulator::running_simulators,
            commands::simulator::watch_simulators,
            commands::simulator::shut_down_other_simulators,
            commands::simulator::build_data_size,
            commands::simulator::prune_build_data,
            commands::simulator::simulator_service_groups,
            commands::simulator::show_simulator_screen,
            commands::simulator::move_simulator_screen,
            commands::simulator::set_simulator_buttons,
            commands::simulator::hide_simulator_screen,
            commands::simulator::send_simulator_input,
            commands::simulator::press_simulator_button,
            commands::simulator::open_simulator_app,
            commands::simulator::take_simulator_screenshot,
            commands::simulator::record_simulator,
            commands::simulator::stop_simulator_recording,
            commands::simulator::simulator_appearance,
            commands::simulator::set_simulator_appearance,
            commands::simulator::pick_screenshot_folder,
            commands::simulator::drag_file_out,
            commands::app::quit_app,
            commands::app::set_keep_awake,
            commands::app::open_link,
            commands::browser::show_browser,
            commands::browser::move_browser,
            commands::browser::hide_browser,
            commands::browser::close_browser,
            commands::browser::browse,
        ])
}

/// The socket the agents' build and run tools call. The temp folder keeps the path under the 104 bytes a socket path may hold.
fn tools_socket() -> PathBuf {
    std::env::temp_dir().join(format!("chord-{}.sock", std::process::id()))
}

/// Keeps the app's own web view on the app: a link that would load another page in it, from a right-click Open Link or a dropped link, goes to the window, which opens it like a chat link. Browser tabs navigate freely.
fn link_guard() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("link-guard")
        .on_navigation(|webview, url| {
            if webview.label() != "main" {
                return true;
            }
            let dev = webview.config().build.dev_url.as_ref();
            let own = url.scheme() == "tauri"
                || url.host_str() == Some("tauri.localhost")
                || dev.is_some_and(|dev| dev.origin() == url.origin());
            if !own {
                let _ = commands::app::LinkRequested(url.to_string()).emit(webview);
            }
            own
        })
        .build()
}

pub fn run() {
    let ipc = ipc();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(link_guard())
        .invoke_handler(ipc.invoke_handler())
        .setup(move |app| {
            ipc.mount_events(app);
            chord_ios::set_cache_folder(&app.path().app_cache_dir()?);
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let store = Arc::new(Store::open(dir.join("state.json"))?);
            let sessions = Sessions::new(Arc::clone(&store), dir.join("sessions"));
            let socket = tools_socket();
            let tools = AgentTools::default();
            let handle = app.handle().clone();
            // Without the socket the agents work on without the tools.
            let sessions = match tools.listen(&socket, move |call| {
                let _ = commands::build::AgentBuildRequested(call).emit(&handle);
            }) {
                Ok(()) => sessions.with_tools(socket),
                Err(_) => sessions,
            };
            app.manage(sessions);
            app.manage(tools);
            app.manage(store);
            app.manage(Terminals::default());
            app.manage(Builds::default());
            app.manage(notifications::ShownPanes::default());
            notifications::listen_for_clicks(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let _ = std::fs::remove_file(tools_socket());
            }
            // Closing the window asks like Quit does: the window quits at once or shows its sheet.
            if let tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::CloseRequested { api, .. },
                ..
            } = event
            {
                api.prevent_close();
                let _ = app.emit("quit-requested", ());
            }
        });
}

#[cfg(test)]
mod tests {
    use specta_typescript::Typescript;

    #[test]
    fn bindings_match_the_commands() -> Result<(), Box<dyn std::error::Error>> {
        let committed = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/ipc/bindings.ts");
        let fresh =
            std::env::temp_dir().join(format!("chord-bindings-{}.ts", std::process::id()));
        super::ipc().export(
            Typescript::default()
                .header("// Generated by tauri-specta from the Rust commands. Never edit."),
            &fresh,
        )?;
        let generated = std::fs::read_to_string(&fresh)?;
        std::fs::remove_file(&fresh)?;
        if std::env::var_os("UPDATE_BINDINGS").is_some() {
            std::fs::write(committed, &generated)?;
        }
        let current = std::fs::read_to_string(committed).unwrap_or_default();
        assert!(
            current == generated,
            "src/ipc/bindings.ts is stale. Run `npm run bindings`."
        );
        Ok(())
    }
}
