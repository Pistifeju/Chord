//! The tool calls a run left open, ended as stopped when the run settles or the agent dies.

use super::{HarnessEvent, ToolOutcome};

// A run that settles or dies leaves calls the harness never closed; they end as stopped so none shows as running.
#[derive(Default)]
pub(crate) struct OpenCalls(Vec<String>);

impl OpenCalls {
    pub(crate) fn track(&mut self, events: Vec<HarnessEvent>) -> Vec<HarnessEvent> {
        let mut out = Vec::new();
        for event in events {
            match &event {
                HarnessEvent::ToolStart { id, .. } => self.0.push(id.clone()),
                HarnessEvent::ToolEnd { id, .. } => self.0.retain(|open| open != id),
                HarnessEvent::State { running: false } => out.extend(self.stop()),
                _ => {}
            }
            out.push(event);
        }
        out
    }

    pub(crate) fn stop(&mut self) -> Vec<HarnessEvent> {
        self.0
            .drain(..)
            .map(|id| HarnessEvent::ToolEnd {
                id,
                outcome: ToolOutcome::Stopped,
                images: vec![],
                ms: None,
                changes: None,
            })
            .collect()
    }
}
