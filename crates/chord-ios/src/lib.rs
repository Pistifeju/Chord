//! Chord's iOS support: building a worktree's Xcode project and running it on the worktree's simulator. Depends on `chord-core`, never on Tauri.

pub mod build;
mod derived_data;
mod issues;
mod process;
mod simulator;
mod slot;
mod xcode;

pub use build::{BuildEvent, BuildPhase, Builds};
pub use issues::BuildIssue;
