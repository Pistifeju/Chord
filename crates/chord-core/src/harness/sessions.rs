//! The running agents, one per pane: start, send, abort, stop, and the saved facts that follow them: the running flag and the session file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::pi::{Session, session_file};
use super::{
    Capability, DialogAnswer, HarnessEvent, Image, Status, StatusReport, StatusTracker, status_for,
};
use crate::awake::Awake;
use crate::{Error, State, Store, WorktreePanes};

pub struct Sessions {
    running: Mutex<HashMap<String, (PathBuf, Session)>>,
    store: Arc<Store>,
    awake: Arc<Awake>,
    dir: PathBuf,
}

impl Sessions {
    /// `dir` holds the session files and the extension the harness loads.
    pub fn new(store: Arc<Store>, dir: PathBuf) -> Sessions {
        let awake = Arc::new(Awake::new(store.get().keep_awake));
        Sessions {
            running: Mutex::default(),
            store,
            awake,
            dir,
        }
    }

    fn sessions(&self) -> MutexGuard<'_, HashMap<String, (PathBuf, Session)>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn with(
        &self,
        pane: &str,
        act: impl FnOnce(&Session) -> Result<(), Error>,
    ) -> Result<(), Error> {
        match self.sessions().get(pane) {
            Some((_, session)) => act(session),
            None => Err(Error::Message("The agent is not running".into())),
        }
    }

    /// Starts the pane's agent in the worktree, replaces the one the pane had, and saves the pane as running.
    /// `on_status_change` hears a status once per change, with the agent's last message, before `on_event` gets it.
    pub fn start(
        &self,
        binary: &str,
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
        let session = Session::start(
            binary,
            &worktree,
            &held_file(&self.store.get(), &self.dir, &pane)?,
            &self.dir,
            move |event| {
                let mut watch = watch.lock().unwrap_or_else(PoisonError::into_inner);
                let report = watch.observe(&event);
                on_event(event);
                if let Some(StatusReport { status, changed }) = report {
                    let last_message = watch.tracker.last_message();
                    if changed {
                        on_status_change(status, last_message);
                    }
                    on_event(HarnessEvent::Status {
                        status,
                        last_message: last_message.to_string(),
                    });
                }
            },
        )?;
        let capabilities = session.capabilities();
        // The lock is free again before the old session drops: dropping kills pi and waits for it.
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
            .map(|other| held_file(&state, &self.dir, &other))
            .collect::<Result<Vec<_>, _>>()?;
        self.with(pane, |session| session.send(text, images, &held_elsewhere))
    }

    pub fn answer_dialog(&self, pane: &str, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        self.with(pane, |session| session.answer_dialog(id, answer))
    }

    pub fn abort(&self, pane: &str) -> Result<(), Error> {
        self.with(pane, Session::abort)
    }

    /// The pane is gone: its agent stops and no longer starts on launch. Its session files stay for `/resume`.
    pub fn stop_pane(&self, pane: &str) -> Result<(), Error> {
        let session = self.sessions().remove(pane);
        drop(session);
        self.awake.set_working(pane, false);
        self.store.update(|state| drop_pane(state, pane))
    }

    /// Returns once every agent has ended, so none outlives the app. The panes stay saved as running.
    pub fn stop_all(&self) {
        let stopped = std::mem::take(&mut *self.sessions());
        drop(stopped);
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
        stopped
            .into_iter()
            .for_each(|(_, session)| session.end_reported());
    }

    /// The worktree is gone: its layouts and running flags go with it. Its session files stay for `/resume`.
    pub fn forget(&self, worktree: &Path) -> Result<(), Error> {
        self.stop(worktree);
        self.store.update(|state| {
            let panes = state
                .panes
                .remove(worktree)
                .map(|saved| saved.pane_ids())
                .unwrap_or_default();
            for pane in &panes {
                drop_pane(state, pane);
            }
        })
    }
}

// A pane that is gone no longer starts on launch, and its next start opens a new session file.
fn drop_pane(state: &mut State, pane: &str) {
    state.running_panes.remove(pane);
    state.session_files.remove(pane);
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
            HarnessEvent::SessionInfo { file, .. } => {
                let _ = self
                    .store
                    .update(|state| state.session_files.insert(self.pane.clone(), file.clone()));
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

// The session the pane's agent last held, or the file a pane held before Chord saved which one.
fn held_file(state: &State, dir: &Path, pane: &str) -> Result<PathBuf, Error> {
    match state.session_files.get(pane) {
        Some(file) => Ok(file.clone()),
        None => session_file(dir, pane),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layout, PaneSession, WorktreePanes};

    #[test]
    fn forgetting_a_worktree_drops_its_layout_and_running_flags_and_keeps_its_session_files()
    -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-forget-{}", std::process::id()));
        let dir = root.join("sessions");
        std::fs::create_dir_all(&dir)?;
        let store = Arc::new(Store::open(root.join("state.json"))?);
        let pane = |id: &str| Layout::Pane { id: id.into() };
        store.update(|state| {
            state.panes.insert(
                PathBuf::from("/w"),
                WorktreePanes {
                    sessions: vec![PaneSession {
                        id: "s".into(),
                        name: "Session 1".into(),
                        layout: Layout::Split {
                            direction: crate::Direction::Row,
                            sizes: vec![0.5, 0.5],
                            children: vec![pane("a"), pane("b")],
                        },
                        focused: "a".into(),
                    }],
                    active: "s".into(),
                },
            );
            state
                .running_panes
                .extend(["a".to_string(), "other".to_string()]);
            state
                .session_files
                .insert("b".into(), dir.join("resumed.jsonl"));
        })?;
        std::fs::write(dir.join("a.jsonl"), "")?;
        std::fs::write(dir.join("resumed.jsonl"), "")?;
        std::fs::write(dir.join("other.jsonl"), "")?;

        let sessions = Sessions::new(Arc::clone(&store), dir.clone());
        sessions.forget(Path::new("/w"))?;

        let state = store.get();
        let left_files = (
            dir.join("a.jsonl").exists(),
            dir.join("resumed.jsonl").exists(),
            dir.join("other.jsonl").exists(),
        );
        let _ = std::fs::remove_dir_all(&root);
        assert!(state.panes.is_empty());
        assert_eq!(
            state.running_panes.into_iter().collect::<Vec<_>>(),
            ["other"]
        );
        assert!(state.session_files.is_empty());
        assert_eq!(left_files, (true, true, true));
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
            file: PathBuf::from("/s/a.jsonl"),
        });
        watch.observe(&HarnessEvent::Exited { code: Some(0) });

        let state = store.get();
        let _ = std::fs::remove_dir_all(&root);
        assert!(state.running_panes.is_empty());
        assert_eq!(
            state.session_files.get("a"),
            Some(&PathBuf::from("/s/a.jsonl"))
        );
        Ok(())
    }

    #[test]
    fn a_closed_pane_no_longer_starts_on_launch_and_keeps_its_session_file() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-close-{}", std::process::id()));
        let dir = root.join("sessions");
        std::fs::create_dir_all(&dir)?;
        let store = Arc::new(Store::open(root.join("state.json"))?);
        store.update(|state| {
            state.running_panes.insert("a".into());
            state
                .session_files
                .insert("a".into(), dir.join("resumed.jsonl"));
        })?;
        std::fs::write(dir.join("resumed.jsonl"), "")?;

        Sessions::new(Arc::clone(&store), dir.clone()).stop_pane("a")?;

        let state = store.get();
        let kept = dir.join("resumed.jsonl").exists();
        let _ = std::fs::remove_dir_all(&root);
        assert!(state.running_panes.is_empty());
        assert!(state.session_files.is_empty());
        assert!(kept);
        Ok(())
    }
}
