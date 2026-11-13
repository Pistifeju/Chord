//! A worktree's tab groups and their split layout: the shape the state file saves, and the older one-layout-per-session shape it reads.

use serde::{Deserialize, Deserializer, Serialize};
use specta::Type;

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
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

    fn group_with(&mut self, tab: &str) -> Option<&mut Vec<Tab>> {
        match self {
            Layout::Group { tabs, .. } => tabs.iter().any(|t| t.id() == tab).then_some(tabs),
            Layout::Split { children, .. } => {
                children.iter_mut().find_map(|child| child.group_with(tab))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Tab {
    /// A chat with its agent; `name` shows until its first user message names it.
    Chat { id: String, name: String },
    /// A file of the worktree, by its relative path.
    File { id: String, path: String },
}

impl Tab {
    fn id(&self) -> &str {
        match self {
            Tab::Chat { id, .. } | Tab::File { id, .. } => id,
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

/// The shape saved before tab groups: session tabs, each with its own split layout of panes.
mod sessions {
    use super::Direction;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Saved {
        pub sessions: Vec<Session>,
        pub active: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Session {
        pub id: String,
        pub name: String,
        pub layout: Layout,
        pub focused: String,
    }

    #[derive(Deserialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    pub enum Layout {
        Pane {
            id: String,
        },
        File {
            id: String,
            path: String,
        },
        Split {
            direction: Direction,
            sizes: Vec<f64>,
            children: Vec<Layout>,
        },
    }
}

fn tabs_of(name: &str, layout: sessions::Layout) -> Vec<Tab> {
    match layout {
        sessions::Layout::Pane { id } => vec![Tab::Chat {
            id,
            name: name.to_string(),
        }],
        sessions::Layout::File { id, path } => vec![Tab::File { id, path }],
        sessions::Layout::Split { children, .. } => children
            .into_iter()
            .flat_map(|child| tabs_of(name, child))
            .collect(),
    }
}

fn grouped(name: &str, layout: sessions::Layout) -> Layout {
    match layout {
        sessions::Layout::Split {
            direction,
            sizes,
            children,
        } => Layout::Split {
            direction,
            sizes,
            children: children
                .into_iter()
                .map(|child| grouped(name, child))
                .collect(),
        },
        leaf => {
            let tabs = tabs_of(name, leaf);
            let id = tabs[0].id().to_string();
            Layout::Group {
                id: format!("group-{id}"),
                tabs,
                active: id,
            }
        }
    }
}

// The open session keeps its split; every pane of the other sessions becomes a tab of the group that has the focus.
impl From<sessions::Saved> for WorktreePanes {
    fn from(saved: sessions::Saved) -> Self {
        let at = saved
            .sessions
            .iter()
            .position(|s| s.id == saved.active)
            .unwrap_or(0);
        let mut all = saved.sessions;
        let open = all.remove(at);
        let mut layout = grouped(&open.name, open.layout);
        let others: Vec<Tab> = all
            .into_iter()
            .flat_map(|s| tabs_of(&s.name, s.layout))
            .collect();
        if let Some(tabs) = layout.group_with(&open.focused) {
            tabs.extend(others);
        }
        WorktreePanes {
            layout,
            focused: open.focused,
        }
    }
}

impl<'de> Deserialize<'de> for WorktreePanes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Saved {
            Groups { layout: Layout, focused: String },
            Sessions(sessions::Saved),
        }
        Ok(match Saved::deserialize(deserializer)? {
            Saved::Groups { layout, focused } => WorktreePanes { layout, focused },
            Saved::Sessions(saved) => saved.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_sessions_become_tabs_of_the_focused_group() -> Result<(), serde_json::Error> {
        let saved = r#"{"sessions":[
            {"id":"s1","name":"Session 1","layout":{"kind":"pane","id":"a"},"focused":"a"},
            {"id":"s2","name":"Session 2","layout":{"kind":"split","direction":"row","sizes":[0.5,0.5],
              "children":[{"kind":"pane","id":"b"},{"kind":"file","id":"f","path":"x.ts"}]},"focused":"f"}
        ],"active":"s2"}"#;
        let panes: WorktreePanes = serde_json::from_str(saved)?;
        let group = |id: &str, tabs: Vec<Tab>, active: &str| Layout::Group {
            id: id.into(),
            tabs,
            active: active.into(),
        };
        let chat = |id: &str, name: &str| Tab::Chat {
            id: id.into(),
            name: name.into(),
        };
        let file = Tab::File {
            id: "f".into(),
            path: "x.ts".into(),
        };
        assert_eq!(
            panes,
            WorktreePanes {
                layout: Layout::Split {
                    direction: Direction::Row,
                    sizes: vec![0.5, 0.5],
                    children: vec![
                        group("group-b", vec![chat("b", "Session 2")], "b"),
                        group("group-f", vec![file, chat("a", "Session 1")], "f"),
                    ],
                },
                focused: "f".into(),
            }
        );
        Ok(())
    }
}
