//! Chord's core: everything that runs without Tauri.

mod awake;
pub mod changes;
mod error;
pub mod files;
mod git;
pub mod git_counts;
pub mod harness;
pub mod images;
mod lock;
mod panes;
pub mod projects;
mod state;
pub mod terminal;
pub mod worktrees;

pub use changes::{Change, FileDiff};
pub use error::Error;
pub use files::FileEntry;
pub use git_counts::GitCounts;
pub use harness::{
    Capability, DialogAnswer, DialogAsk, HarnessEvent, Image, SlashCommand, Status, login_shell,
};
pub use lock::lock;
pub use panes::{DiffSide, Direction, Layout, Tab, WorktreePanes};
pub use projects::Project;
pub use state::{
    Appearance, BottomPanel, BottomView, PaneAgent, PreviewPick, RunDestination, SchemePick,
    Shortcut, SidePanel, SideView, Sidebar, State, StateChange, Store, Theme,
};
pub use terminal::{TerminalEvent, Terminals};
pub use worktrees::{KeptBranch, Removal, Worktree, xcode_bundles};
