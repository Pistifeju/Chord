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

    pub fn remove_worktree(
        &self,
        project: &Project,
        worktree: &Path,
        force: bool,
    ) -> Result<Removal, Error> {
        if let Some(refusal) = worktrees::refusal(worktree, force)? {
            return Ok(refusal);
        }
        self.terminals.stop(worktree);
        self.sessions.stop(worktree);
        let branch = match worktrees::remove_folder(project, worktree, force)? {
            Ok(branch) => branch,
            Err(refusal) => return Ok(refusal),
        };
        self.sessions.forget(worktree)?;
        let kept_branch = branch.and_then(|name| worktrees::delete_merged_branch(project, name));
        self.builds.forget(worktree);
        Ok(Removal::Removed { kept_branch })
    }

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
