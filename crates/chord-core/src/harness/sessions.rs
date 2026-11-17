//! The running agents, one per pane: start, send, abort, stop, and the saved facts that follow them: the running flag, the harness and the session.

use crate::lock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use super::{
    Agent, Capability, DialogAnswer, HarnessEvent, Image, Status, StatusReport, StatusTracker,
    status_for,
};
use crate::awake::Awake;
use crate::{Error, Store, WorktreePanes, xcode_bundles};

/// Each pane's agent and the worktree it runs in.
type Running = HashMap<String, (PathBuf, Box<dyn Agent>)>;

pub struct Sessions {
    running: Mutex<Running>,
    store: Arc<Store>,
    awake: Arc<Awake>,
    dir: PathBuf,
    tools: Option<PathBuf>,
}

impl Sessions {
    /// `dir` holds the files the harness adapters write for themselves.
    pub fn new(store: Arc<Store>, dir: PathBuf) -> Sessions {
        let awake = Arc::new(Awake::new(store.get().keep_awake));
        Sessions {
            running: Mutex::default(),
            store,
            awake,
            dir,
            tools: None,
        }
    }

    /// Agents in a worktree with an Xcode project get the build and run tools, whose calls go to `socket`.
    pub fn with_tools(self, socket: PathBuf) -> Sessions {
        Sessions {
            tools: Some(socket),
            ..self
        }
    }

    fn sessions(&self) -> MutexGuard<'_, Running> {
        lock(&self.running)
    }

    fn with(
        &self,
        pane: &str,
        act: impl FnOnce(&dyn Agent) -> Result<(), Error>,
    ) -> Result<(), Error> {
        match self.sessions().get(pane) {
            Some((_, session)) => act(session.as_ref()),
            None => Err(Error::Message("The agent is not running".into())),
        }
    }

    /// Starts the pane's agent in the worktree with the harness the pane saved, replaces the one the pane had, and saves the pane as running.
    /// `on_status_change` hears a status once per change, with the agent's last message, before `on_event` gets it.
    pub fn start(
        &self,
        worktree: PathBuf,
        pane: String,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
        on_status_change: impl Fn(Status, &str) + Send + 'static,
    ) -> Result<Vec<Capability>, Error> {
        let watch = Mutex::new(PaneWatch {
            pane: pane.clone(),
            store: Arc::clone(&self.store),
            awake: Arc::clone(&self.awake),
            tracker: StatusTracker::default(),
        });
        let agent = self
            .store
            .update(|state| state.agents.entry(pane.clone()).or_default().clone())?;
        let tools = self
            .tools
            .as_deref()
            .filter(|_| !xcode_bundles(&worktree).is_empty());
        let session = super::start(
            &agent.harness,
            &worktree,
            agent.session.as_deref(),
            &self.dir,
            tools,
            move |event| {
                let mut watch = lock(&watch);
                let dialog_was_open = watch.tracker.dialog_open();
                let report = watch.observe(&event);
                on_event(event);
                if dialog_was_open && !watch.tracker.dialog_open() {
                    on_event(HarnessEvent::DialogClosed);
                }
                if let Some(StatusReport { status, changed }) = report {
                    let last_message = watch.tracker.last_message();
                    if changed {
                        on_status_change(status, last_message);
                    }
                    on_event(HarnessEvent::Status {
                        status,
                        last_message: last_message.to_string(),
                        changed,
                    });
                }
            },
        )?;
        let capabilities = session.capabilities();
        // The lock is free again before the old session drops: dropping kills the agent's process and waits for it.
        let replaced = self.sessions().insert(pane.clone(), (worktree, session));
        drop(replaced);
        self.store
            .update(|state| state.running_panes.insert(pane))?;
        Ok(capabilities)
    }

    pub fn send(&self, pane: &str, text: &str, images: &[Image]) -> Result<(), Error> {
        let state = self.store.get();
        let held_elsewhere = state
            .panes
            .values()
            .flat_map(WorktreePanes::pane_ids)
            .filter(|other| other != pane)
            .filter_map(|other| state.agents.get(&other)?.session.clone())
            .collect::<Vec<_>>();
        self.with(pane, |session| session.send(text, images, &held_elsewhere))
    }

    pub fn answer_dialog(&self, pane: &str, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        self.with(pane, |session| session.answer_dialog(id, answer))
    }

    pub fn abort(&self, pane: &str) -> Result<(), Error> {
        self.with(pane, |session| session.abort())
    }

    /// The pane is gone: its agent stops and no longer starts on launch. Its sessions stay for `/resume`.
    pub fn stop_pane(&self, pane: &str) -> Result<(), Error> {
        let session = self.sessions().remove(pane);
        drop(session);
        self.awake.set_working(pane, false);
        self.store.update(|state| state.forget_pane(pane))
    }

    /// Returns once every agent has ended, so none outlives the app. The panes stay saved as running.
    pub fn stop_all(&self) {
        let stopped = std::mem::take(&mut *self.sessions());
        end_at_once(stopped.into_values().map(|(_, agent)| agent), drop);
        self.awake.clear();
    }

    /// Turns the setting on or off; on fails with `MissingTool` when `caffeinate` is missing, even with no agent working.
    pub fn keep_awake(&self, enabled: bool) -> Result<(), Error> {
        self.awake.set_enabled(enabled)
    }

    /// Returns once every agent in the worktree has ended, so git can remove its directory. Its layouts stay in case git refuses, and each chat hears its agent exit.
    pub fn stop(&self, worktree: &Path) {
        let mut stopped = Vec::new();
        {
            let mut sessions = self.sessions();
            let panes: Vec<String> = sessions
                .iter()
                .filter(|(_, (dir, _))| dir == worktree)
                .map(|(pane, _)| pane.clone())
                .collect();
            stopped.extend(panes.iter().filter_map(|pane| sessions.remove(pane)));
            for pane in &panes {
                self.awake.set_working(pane, false);
            }
        }
        end_at_once(stopped.into_iter().map(|(_, agent)| agent), |agent| {
            agent.end_reported()
        });
    }

    /// The worktree is gone: its layouts, panels and running flags go with it. Its sessions stay for `/resume`.
    pub fn forget(&self, worktree: &Path) -> Result<(), Error> {
        self.stop(worktree);
        self.store.update(|state| state.forget_worktree(worktree))
    }
}

// Each agent waits out its exit on a thread of its own, so ending ten takes as long as ending the slowest.
fn end_at_once(agents: impl Iterator<Item = Box<dyn Agent>>, end: fn(Box<dyn Agent>)) {
    std::thread::scope(|scope| {
        for agent in agents {
            scope.spawn(move || end(agent));
        }
    });
}

/// What one pane's agent events change: the saved facts, keep-awake and the status.
struct PaneWatch {
    pane: String,
    store: Arc<Store>,
    awake: Arc<Awake>,
    tracker: StatusTracker,
}

impl PaneWatch {
    fn observe(&mut self, event: &HarnessEvent) -> Option<StatusReport> {
        match event {
            HarnessEvent::State { running } => self.awake.set_working(&self.pane, *running),
            // The next start opens the session the pane shows now.
            HarnessEvent::SessionInfo { id } => {
                let _ = self.store.update(|state| {
                    state.agents.entry(self.pane.clone()).or_default().session = Some(id.clone());
                });
            }
            HarnessEvent::Exited { .. } => {
                self.awake.set_working(&self.pane, false);
                // An agent that ended on its own does not come back on launch.
                let _ = self
                    .store
                    .update(|state| state.running_panes.remove(&self.pane));
            }
            _ => {}
        }
        status_for(&mut self.tracker, event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layout, PaneAgent, Tab, WorktreePanes};

    fn held(session: &str) -> PaneAgent {
        PaneAgent {
            harness: "pi".into(),
            session: Some(session.into()),
        }
    }

    #[test]
    fn forgetting_a_worktree_drops_its_layout_panels_running_flags_and_agents() -> Result<(), Error>
    {
        let root = std::env::temp_dir().join(format!("chord-forget-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let store = Arc::new(Store::open(root.join("state.json"))?);
        let group = |id: &str| Layout::Group {
            id: format!("group-{id}"),
            tabs: vec![Tab::Chat {
                id: id.into(),
                name: "Session 1".into(),
            }],
            active: id.into(),
        };
        store.update(|state| {
            state.panes.insert(
                PathBuf::from("/w"),
                WorktreePanes {
                    layout: Layout::Split {
                        direction: crate::Direction::Row,
                        sizes: vec![0.5, 0.5],
                        children: vec![group("a"), group("b")],
                    },
                    focused: "a".into(),
                },
            );
            state
                .running_panes
                .extend(["a".to_string(), "other".to_string()]);
            state.agents.insert("b".into(), held("resumed.jsonl"));
            state.agents.insert("other".into(), held("other.jsonl"));
            state
                .side_panels
                .insert(PathBuf::from("/w"), crate::SidePanel::default());
            state
                .bottom_panels
                .insert(PathBuf::from("/w"), crate::BottomPanel::default());
        })?;

        Sessions::new(Arc::clone(&store), root.clone()).forget(Path::new("/w"))?;

        let state = store.get();
        let _ = std::fs::remove_dir_all(&root);
        assert!(state.panes.is_empty());
        assert!(state.side_panels.is_empty() && state.bottom_panels.is_empty());
        assert_eq!(
            state.running_panes.into_iter().collect::<Vec<_>>(),
            ["other"]
        );
        assert_eq!(state.agents.keys().collect::<Vec<_>>(), ["other"]);
        Ok(())
    }

    #[test]
    fn an_agent_that_exits_keeps_the_session_it_showed_and_no_longer_starts_on_launch()
    -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-exit-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let store = Arc::new(Store::open(root.join("state.json"))?);
        store.update(|state| state.running_panes.insert("a".into()))?;
        let mut watch = PaneWatch {
            pane: "a".into(),
            store: Arc::clone(&store),
            awake: Arc::new(Awake::new(false)),
            tracker: StatusTracker::default(),
        };

        watch.observe(&HarnessEvent::SessionInfo {
            id: "/s/a.jsonl".into(),
        });
        watch.observe(&HarnessEvent::Exited { code: Some(0) });

        let state = store.get();
        let _ = std::fs::remove_dir_all(&root);
        assert!(state.running_panes.is_empty());
        assert_eq!(state.agents.get("a"), Some(&held("/s/a.jsonl")));
        Ok(())
    }

    #[test]
    fn a_closed_pane_no_longer_starts_on_launch_and_forgets_its_agent() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-close-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let store = Arc::new(Store::open(root.join("state.json"))?);
        store.update(|state| {
            state.running_panes.insert("a".into());
            state.agents.insert("a".into(), held("resumed.jsonl"));
        })?;

        Sessions::new(Arc::clone(&store), root.clone()).stop_pane("a")?;

        let state = store.get();
        let _ = std::fs::remove_dir_all(&root);
        assert!(state.running_panes.is_empty());
        assert!(state.agents.is_empty());
        Ok(())
    }

    #[test]
    fn a_pane_saved_with_an_unknown_harness_does_not_start() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-harness-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let store = Arc::new(Store::open(root.join("state.json"))?);
        store.update(|state| {
            state.agents.insert(
                "a".into(),
                PaneAgent {
                    harness: "nope".into(),
                    session: None,
                },
            );
        })?;

        let started = Sessions::new(Arc::clone(&store), root.clone()).start(
            PathBuf::from("/w"),
            "a".into(),
            |_| {},
            |_, _| {},
        );

        let state = store.get();
        let _ = std::fs::remove_dir_all(&root);
        assert!(started.is_err());
        assert!(state.running_panes.is_empty());
        Ok(())
    }
}
