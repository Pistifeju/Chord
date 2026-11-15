//! Chord's iOS support: building a worktree's Xcode project. Depends on `chord-core`, never on Tauri.

pub mod build;
mod derived_data;
mod issues;
mod slot;
mod xcode;

pub use build::{BuildEvent, BuildPhase, Builds};
pub use issues::BuildIssue;
