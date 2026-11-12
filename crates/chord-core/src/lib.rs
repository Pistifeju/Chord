//! Chord's core: everything that runs without Tauri.

mod awake;
mod error;
mod git;
pub mod git_counts;
pub mod harness;
pub mod images;
mod panes;
pub mod projects;
mod state;
pub mod worktrees;

pub use error::Error;
pub use git_counts::GitCounts;
pub use harness::{Capability, DialogAnswer, DialogAsk, HarnessEvent, Image, Status};
pub use panes::{Direction, Layout, PaneSession, WorktreePanes};
pub use projects::Project;
pub use state::{Appearance, Shortcut, Sidebar, State, StateChange, Store, Theme};
pub use worktrees::{KeptBranch, Removal, Worktree};
