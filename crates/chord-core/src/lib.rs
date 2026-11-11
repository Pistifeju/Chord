//! Chord's core: everything that runs without Tauri.

mod error;
mod git;
pub mod projects;
mod state;
pub mod worktrees;

pub use error::Error;
pub use projects::Project;
pub use state::{Sidebar, State, Store};
pub use worktrees::{KeptBranch, Removal, Worktree};
