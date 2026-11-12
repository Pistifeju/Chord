//! The running agents, one per pane: start, send, abort, stop.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::pi::Session;
use super::{Capability, HarnessEvent};
use crate::Error;

#[derive(Default)]
pub struct Sessions(Mutex<HashMap<String, (PathBuf, Session)>>);

impl Sessions {
    fn sessions(&self) -> MutexGuard<'_, HashMap<String, (PathBuf, Session)>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
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

    /// Starts the pane's agent in the worktree and replaces the one the pane had.
    pub fn start(
        &self,
        binary: &str,
        worktree: PathBuf,
        pane: String,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Vec<Capability>, Error> {
        let session = Session::start(binary, &worktree, on_event)?;
        let capabilities = session.capabilities();
        // The lock is free again before the old session drops: dropping kills pi and waits for it.
        let replaced = self.sessions().insert(pane, (worktree, session));
        drop(replaced);
        Ok(capabilities)
    }

    pub fn send(&self, pane: &str, text: &str) -> Result<(), Error> {
        self.with(pane, |session| session.send(text))
    }

    pub fn abort(&self, pane: &str) -> Result<(), Error> {
        self.with(pane, Session::abort)
    }

    pub fn stop_pane(&self, pane: &str) {
        let session = self.sessions().remove(pane);
        drop(session);
    }

    /// Returns once every agent in the worktree has ended, so git can remove its directory.
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
}
