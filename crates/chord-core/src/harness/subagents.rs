use std::time::{SystemTime, UNIX_EPOCH};

use super::{HarnessEvent, SubagentNode, SubagentState};

const UNNAMED: &str = "subagent";

struct Run {
    parent: Option<String>,
    node: SubagentNode,
}

#[derive(Default)]
pub(crate) struct Subagents(Vec<Run>);

impl Subagents {
    pub(crate) fn knows(&self, id: &str) -> bool {
        self.0.iter().any(|run| run.node.id == id)
    }

    pub(crate) fn start(
        &mut self,
        id: &str,
        parent: Option<&str>,
        name: Option<&str>,
    ) -> HarnessEvent {
        let now = now();
        match self.0.iter_mut().find(|run| run.node.id == id) {
            Some(run) => {
                if let Some(name) = name {
                    run.node.name = name.into();
                }
                run.node.state = SubagentState::Running;
                run.node.ended_at = None;
            }
            None => self.0.push(Run {
                parent: parent.filter(|parent| self.knows(parent)).map(String::from),
                node: SubagentNode {
                    id: id.into(),
                    depth: 0,
                    name: name.unwrap_or(UNNAMED).into(),
                    state: SubagentState::Running,
                    tool: None,
                    started_at: now,
                    ended_at: None,
                },
            }),
        }
        self.event()
    }

    pub(crate) fn tool(&mut self, id: &str, tool: &str) -> Option<HarnessEvent> {
        let run = self.0.iter_mut().find(|run| run.node.id == id)?;
        if run.node.state != SubagentState::Running || run.node.tool.as_deref() == Some(tool) {
            return None;
        }
        run.node.tool = Some(tool.into());
        Some(self.event())
    }

    pub(crate) fn end(&mut self, id: &str, state: SubagentState) -> Option<HarnessEvent> {
        let run = self.0.iter_mut().find(|run| run.node.id == id)?;
        if run.node.state != SubagentState::Running {
            return None;
        }
        run.node.state = state;
        run.node.ended_at = now();
        Some(self.event())
    }

    pub(crate) fn clear(&mut self) -> Option<HarnessEvent> {
        (!self.0.is_empty()).then(|| {
            self.0.clear();
            self.event()
        })
    }

    #[cfg(test)]
    pub(crate) fn lineage(
        events: &[HarnessEvent],
    ) -> Vec<(u32, String, SubagentState, Option<String>)> {
        match events.last() {
            Some(HarnessEvent::Subagents { nodes }) => nodes
                .iter()
                .map(|node| (node.depth, node.name.clone(), node.state, node.tool.clone()))
                .collect(),
            _ => vec![],
        }
    }

    fn event(&self) -> HarnessEvent {
        fn walk(runs: &[Run], parent: Option<&str>, depth: u32, nodes: &mut Vec<SubagentNode>) {
            for run in runs.iter().filter(|run| run.parent.as_deref() == parent) {
                nodes.push(SubagentNode {
                    depth,
                    ..run.node.clone()
                });
                walk(runs, Some(&run.node.id), depth + 1, nodes);
            }
        }
        let mut nodes = vec![];
        walk(&self.0, None, 0, &mut nodes);
        HarnessEvent::Subagents { nodes }
    }
}

fn now() -> Option<f64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|since| since.as_millis() as f64)
}
