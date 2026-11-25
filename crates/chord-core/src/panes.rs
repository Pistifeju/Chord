use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorktreePanes {
    pub layout: Layout,
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
    Group {
        id: String,
        tabs: Vec<Tab>,
        active: String,
    },
    Split {
        direction: Direction,
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
    Chat {
        id: String,
        name: String,
    },
    File {
        id: String,
        path: String,
        #[serde(default)]
        preview: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        diff: Option<DiffSide>,
    },
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

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum DiffSide {
    Staged,
    Unstaged,
    Branch(String),
    Commit(String),
    Stash(String),
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Row,
    Column,
}
