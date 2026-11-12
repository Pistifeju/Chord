//! The running agents, one per pane: start, send, abort, stop, and the saved facts that follow them: the running flag and the session file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::pi::{Session, session_file};
use super::{Capability, HarnessEvent, StatusTracker, status_for};
use crate::{Error, Store};

pub struct Sessions {
    running: Mutex<HashMap<String, (PathBuf, Session)>>,
    store: Arc<Store>,
    dir: PathBuf,
}

impl Sessions {
    /// `dir` holds one session file per pane.
    pub fn new(store: Arc<Store>, dir: PathBuf) -> Sessions {
        Sessions {
            running: Mutex::default(),
            store,
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
    pub fn start(
        &self,
        binary: &str,
        worktree: PathBuf,
        pane: String,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Vec<Capability>, Error> {
        let store = Arc::clone(&self.store);
        let exiting = pane.clone();
        let tracker = Mutex::new(StatusTracker::default());
        let session = Session::start(
            binary,
            &worktree,
            &session_file(&self.dir, &pane)?,
            move |event| {
                // An agent that ended on its own does not come back on launch.
                if matches!(event, HarnessEvent::Exited { .. }) {
                    let _ = store.update(|state| state.running_panes.remove(&exiting));
                }
                let mut tracker = tracker.lock().unwrap_or_else(PoisonError::into_inner);
                let status = status_for(&mut tracker, &event);
                on_event(event);
                if let Some(status) = status {
                    on_event(HarnessEvent::Status {
                        status,
                        last_message: tracker.last_message().to_string(),
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

    pub fn send(&self, pane: &str, text: &str) -> Result<(), Error> {
        self.with(pane, |session| session.send(text))
    }

    pub fn abort(&self, pane: &str) -> Result<(), Error> {
        self.with(pane, Session::abort)
    }

    /// The pane is gone: its agent stops, it no longer starts on launch, and its session file goes.
    pub fn stop_pane(&self, pane: &str) -> Result<(), Error> {
        let file = session_file(&self.dir, pane)?;
        let session = self.sessions().remove(pane);
        drop(session);
        self.store
            .update(|state| state.running_panes.remove(pane))?;
        let _ = std::fs::remove_file(file);
        Ok(())
    }

    /// Returns once every agent has ended, so none outlives the app. The panes stay saved as running.
    pub fn stop_all(&self) {
        let stopped = std::mem::take(&mut *self.sessions());
        drop(stopped);
    }

    /// Returns once every agent in the worktree has ended, so git can remove its directory. Its saved panes stay, in case git refuses.
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
        }
        drop(stopped);
    }

    /// The worktree is gone: its layouts, running flags and session files go with it.
    pub fn forget(&self, worktree: &Path) -> Result<(), Error> {
        self.stop(worktree);
        let panes = self.store.update(|state| {
            let panes = state
                .panes
                .remove(worktree)
                .map(|saved| saved.pane_ids())
                .unwrap_or_default();
            for pane in &panes {
                state.running_panes.remove(pane);
            }
            panes
        })?;
        for pane in panes {
            let _ = std::fs::remove_file(session_file(&self.dir, &pane)?);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layout, PaneSession, WorktreePanes};

    #[test]
    fn forgetting_a_worktree_drops_its_layout_running_flags_and_session_files() -> Result<(), Error>
    {
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
        })?;
        std::fs::write(dir.join("a.jsonl"), "")?;
        std::fs::write(dir.join("other.jsonl"), "")?;

        let sessions = Sessions::new(Arc::clone(&store), dir.clone());
        sessions.forget(Path::new("/w"))?;

        let state = store.get();
        let left_files = (
            dir.join("a.jsonl").exists(),
            dir.join("other.jsonl").exists(),
        );
        let _ = std::fs::remove_dir_all(&root);
        assert!(state.panes.is_empty());
        assert_eq!(
            state.running_panes.into_iter().collect::<Vec<_>>(),
            ["other"]
        );
        assert_eq!(left_files, (false, true));
        Ok(())
    }
}
