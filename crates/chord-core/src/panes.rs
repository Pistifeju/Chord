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

impl WorktreePanes {
    pub fn pane_ids(&self) -> Vec<String> {
        self.sessions
            .iter()
            .flat_map(|session| session.layout.pane_ids())
            .collect()
    }
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

impl Layout {
    fn pane_ids(&self) -> Vec<String> {
        match self {
            Layout::Pane { id } => vec![id.clone()],
            Layout::Split { children, .. } => children.iter().flat_map(Layout::pane_ids).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    /// Children sit side by side.
    Row,
    /// Children sit one above the other.
    Column,
}
