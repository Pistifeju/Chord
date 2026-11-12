//! A worktree's chat sessions and the split layout of each: the shape the state file saves.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorktreePanes {
    pub sessions: Vec<PaneSession>,
    /// The id of the session whose tab is open.
    pub active: String,
}

/// One session tab: its name, its split layout and the pane that has the focus.
#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaneSession {
    pub id: String,
    pub name: String,
    pub layout: Layout,
    pub focused: String,
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Layout {
    Pane {
        id: String,
    },
    Split {
        direction: Direction,
        /// Each child's share of the split; they add up to 1.
        sizes: Vec<f64>,
        children: Vec<Layout>,
    },
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    /// Children sit side by side.
    Row,
    /// Children sit one above the other.
    Column,
}
