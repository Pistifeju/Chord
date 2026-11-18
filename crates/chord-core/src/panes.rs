//! A worktree's tab groups and their split layout, in the shape the state file saves.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorktreePanes {
    pub layout: Layout,
    /// The id of the tab that has the focus: the open tab of one group.
    pub focused: String,
}

impl WorktreePanes {
    pub fn pane_ids(&self) -> Vec<String> {
        self.layout.tab_ids()
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Layout {
    /// Tabs that share one place, each with its own tab in the group's bar; `active` shows.
    Group {
        id: String,
        tabs: Vec<Tab>,
        active: String,
    },
    Split {
        direction: Direction,
        /// Each child's share of the split; they add up to 1.
        sizes: Vec<f64>,
        children: Vec<Layout>,
    },
}

impl Layout {
    fn tab_ids(&self) -> Vec<String> {
        match self {
            Layout::Group { tabs, .. } => tabs.iter().map(|tab| tab.id().to_string()).collect(),
            Layout::Split { children, .. } => children.iter().flat_map(Layout::tab_ids).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Tab {
    /// A chat with its agent; `name` shows until its first user message names it.
    Chat { id: String, name: String },
    /// A file of the worktree, by its relative path; a `preview` tab gives its place to the next file opened in its group.
    File {
        id: String,
        path: String,
        #[serde(default)]
        preview: bool,
    },
    /// A web page in the window's own browser; `title` is the page's last title.
    Browser {
        id: String,
        url: String,
        #[serde(default)]
        title: String,
    },
}

impl Tab {
    fn id(&self) -> &str {
        match self {
            Tab::Chat { id, .. } | Tab::File { id, .. } | Tab::Browser { id, .. } => id,
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
