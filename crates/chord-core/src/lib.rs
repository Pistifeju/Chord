//! Chord's core: everything that runs without Tauri.

mod error;
mod git;
pub mod git_counts;
pub mod projects;
mod state;
pub mod worktrees;

pub use error::Error;
pub use git_counts::GitCounts;
pub use projects::Project;
pub use state::{Appearance, Shortcut, Sidebar, State, Store, Theme};
pub use worktrees::{KeptBranch, Removal, Worktree};
