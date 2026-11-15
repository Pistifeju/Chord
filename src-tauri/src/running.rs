//! What runs for the worktrees, their agents, shells and builds, and how it stops when a worktree, a project or the app goes.

use std::path::Path;

use chord_core::harness::sessions::Sessions;
use chord_core::{Error, Project, Removal, Terminals, worktrees};
use chord_ios::Builds;
use tauri::{AppHandle, Manager};

pub struct Running<'a> {
    sessions: &'a Sessions,
    terminals: &'a Terminals,
    builds: &'a Builds,
}

impl<'a> Running<'a> {
    pub fn of(app: &'a AppHandle) -> Self {
        Self {
            sessions: app.state::<Sessions>().inner(),
            terminals: app.state::<Terminals>().inner(),
            builds: app.state::<Builds>().inner(),
        }
    }

    /// Stops the worktree's agents and shell and removes it; its build data goes only once git removed it.
    pub fn remove_worktree(
        &self,
        project: &Project,
        worktree: &Path,
        force: bool,
    ) -> Result<Removal, Error> {
        let removal = worktrees::remove(self.sessions, self.terminals, project, worktree, force)?;
        if matches!(removal, Removal::Removed { .. }) {
            self.builds.forget(worktree);
            chord_ios::forget(worktree);
        }
        Ok(removal)
    }

    /// The frontend stops the agents; the shells and builds are Rust's to end.
    pub fn stop_project(&self, project: &Path) {
        for worktree in worktrees::list(project).unwrap_or_default() {
            self.terminals.stop(&worktree.path);
            self.builds.stop(&worktree.path);
        }
    }

    pub fn stop_all(&self) {
        self.sessions.stop_all();
        self.terminals.stop_all();
        self.builds.stop_all();
    }
}
