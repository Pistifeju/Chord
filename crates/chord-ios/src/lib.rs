//! Chord's iOS support: building a worktree's Xcode project and running it on the worktree's simulator or a connected device. Depends on `chord-core`, never on Tauri.

mod agent_reply;
mod agent_tools;
pub mod build;
mod caches;
mod capacity;
mod console;
mod derived_data;
mod destinations;
mod device;
mod hid;
mod issues;
mod process;
mod screen_buttons;
mod screen_view;
mod service_groups;
mod simulator;
mod simulator_kit;
mod slim;
mod slot;
mod xcode;

pub use agent_tools::{AgentBuild, AgentTools};
pub use build::{BuildEvent, BuildPhase, Builds};
pub use caches::{
    RunningSimulator, build_data_size, forget, prune, running_simulators, shut_down_others,
};
pub use console::ConsoleTone;
pub use destinations::{Destinations, SimulatorModels, list as list_destinations};
pub use hid::{Blocking, Button, Input, Point, TouchPhase};
pub use issues::BuildIssue;
pub use screen_buttons::{ButtonColors, ButtonFace, FaceIcon, FaceLabel, FaceLook, ScreenButtons};
pub use screen_view::{
    Bezel, ScreenRect, ScreenState, hide_screen, move_screen, prepare_screen, send_input,
    set_buttons, show_screen,
};
pub use service_groups::{ServiceRow, SubgroupRow};
pub use slim::service_rows;
