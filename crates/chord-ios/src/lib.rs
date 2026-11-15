//! Chord's iOS support: building a worktree's Xcode project and running it on the worktree's simulator or a connected device. Depends on `chord-core`, never on Tauri.

pub mod build;
mod caches;
mod capacity;
mod console;
mod derived_data;
mod destinations;
mod device;
mod issues;
mod process;
mod simulator;
mod slot;
mod xcode;

pub use build::{BuildEvent, BuildPhase, Builds};
pub use caches::{
    RunningSimulator, build_data_size, forget, prune, running_simulators, shut_down_others,
};
pub use console::ConsoleTone;
pub use destinations::{Destinations, SimulatorModels, list as list_destinations};
pub use issues::BuildIssue;
