//! Chord's iOS support: building a worktree's Xcode project, running it on the worktree's simulator or a connected device, and showing its SwiftUI preview on the worktree's preview simulator. Depends on `chord-core`, never on Tauri.

mod agent_reply;
mod agent_tools;
mod appearance;
pub mod build;
mod caches;
mod capacity;
mod compile_commands;
mod console;
mod derived_data;
mod destinations;
mod device;
mod hid;
mod hot_reload;
mod issues;
mod preview;
mod preview_list;
mod process;
mod screen_buttons;
mod screen_view;
mod screenshot;
mod service_groups;
mod simulator;
mod simulator_kit;
mod slim;
mod slot;
mod view_graph;
mod worktree_saves;
mod xcode;

pub use agent_tools::{AgentBuild, AgentTools};
pub use appearance::{
    SimulatorAppearance, SimulatorAppearanceChange, read as read_appearance, set as set_appearance,
};
pub use build::{BuildEvent, BuildPhase, Builds};
pub use caches::{
    RunningSimulator, build_data_size, forget, prune, running_simulators, shut_down_others,
    watch_simulators,
};
pub use console::ConsoleTone;
pub use destinations::{Destinations, SimulatorModels, list as list_destinations};
pub use hid::{Blocking, Button, Input, Point, TouchPhase};
pub use issues::BuildIssue;
pub use preview::{PreviewState, set_cache_folder};
pub use preview_list::list as list_previews;
pub use screen_buttons::{ButtonColors, ButtonFace, FaceIcon, FaceLook, Outline, ScreenButtons};
pub use screen_view::{
    Bezel, ScreenRect, ScreenState, hide_screen, move_screen, prepare_screen, send_input,
    set_buttons, show_screen,
};
pub use screenshot::{Screenshot, take_screenshot};
pub use service_groups::{ServiceRow, SubgroupRow};
pub use simulator::open_app;
pub use slim::service_rows;
