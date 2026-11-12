//! The running agents, one per worktree until panes arrive: start, send, abort, stop.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::pi::Session;
use super::{Capability, HarnessEvent};
use crate::Error;

#[derive(Default)]
pub struct Sessions(Mutex<HashMap<PathBuf, Session>>);

impl Sessions {
    fn sessions(&self) -> MutexGuard<'_, HashMap<PathBuf, Session>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn with(
        &self,
        worktree: &Path,
        act: impl FnOnce(&Session) -> Result<(), Error>,
    ) -> Result<(), Error> {
        match self.sessions().get(worktree) {
            Some(session) => act(session),
            None => Err(Error::Message("The agent is not running".into())),
        }
    }

    /// Starts an agent in the worktree and replaces the one running there.
    pub fn start(
        &self,
        binary: &str,
        worktree: PathBuf,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Vec<Capability>, Error> {
        let session = Session::start(binary, &worktree, on_event)?;
        let capabilities = session.capabilities();
        // The lock is free again before the old session drops: dropping kills pi and waits for it.
        let replaced = self.sessions().insert(worktree, session);
        drop(replaced);
        Ok(capabilities)
    }

    pub fn send(&self, worktree: &Path, text: &str) -> Result<(), Error> {
        self.with(worktree, |session| session.send(text))
    }

    pub fn abort(&self, worktree: &Path) -> Result<(), Error> {
        self.with(worktree, Session::abort)
    }

    /// Returns once the agent's process has ended, so git can remove its directory.
    pub fn stop(&self, worktree: &Path) {
        let session = self.sessions().remove(worktree);
        drop(session);
    }
}
