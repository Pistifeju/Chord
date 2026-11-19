//! A worktree's local branches for the composer's branch menu, newest first with the other worktree that holds each, and the switch of the worktree to one.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::changes;
use crate::{Error, git};

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LocalBranch {
    pub name: String,
    /// When its last commit was made, in milliseconds since 1970.
    pub time: f64,
    /// The other worktree that has it checked out, which keeps git from switching to it here.
    pub held_by: Option<PathBuf>,
}

/// The local branches, newest commit first.
pub fn list(worktree: &Path) -> Result<Vec<LocalBranch>, Error> {
    let here = worktree.canonicalize()?;
    let refs = changes::read(
        worktree,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname)%1f%(committerdate:unix)%1f%(worktreepath)",
            "refs/heads",
        ],
    )?;
    Ok(refs.lines().filter_map(|line| parse(line, &here)).collect())
}

fn parse(line: &str, here: &Path) -> Option<LocalBranch> {
    let mut fields = line.split('\u{1f}');
    let name = fields.next()?.strip_prefix("refs/heads/")?.to_string();
    let seconds: f64 = fields.next()?.parse().unwrap_or(0.0);
    let held_by = fields
        .next()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.canonicalize().unwrap_or_else(|_| path.clone()) != here);
    Some(LocalBranch {
        name,
        time: seconds * 1000.0,
        held_by,
    })
}

/// Checks out `branch` in the worktree; git refuses when local changes would be lost or another worktree holds it.
pub fn switch(worktree: &Path, branch: &str) -> Result<(), Error> {
    git::run(worktree, &["switch", "--no-guess", "--", branch]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_branches_other_worktrees_hold_and_switches_to_a_free_one() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-branches-{}", std::process::id()));
        let result = list_and_switch(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn list_and_switch(root: &Path) -> Result<(), Error> {
        let repo = root.join("app");
        std::fs::create_dir_all(&repo)?;
        git::run(&repo, &["init", "-q", "-b", "main"])?;
        git::run(
            &repo,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "first",
            ],
        )?;
        git::run(&repo, &["branch", "free"])?;
        let other = root.join("other");
        git::run(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "taken",
                &other.to_string_lossy(),
            ],
        )?;

        let listed = list(&repo)?;
        let held = switch(&repo, "taken");
        switch(&repo, "free")?;
        let current = git::run(&repo, &["symbolic-ref", "--short", "HEAD"])?;

        let holder = |name: &str| {
            listed
                .iter()
                .find(|b| b.name == name)
                .map(|b| b.held_by.as_ref().and_then(|p| p.canonicalize().ok()))
        };
        assert_eq!(listed.len(), 3);
        assert_eq!(holder("main"), Some(None));
        assert_eq!(holder("free"), Some(None));
        assert_eq!(holder("taken"), Some(Some(other.canonicalize()?)));
        assert!(held.is_err());
        assert_eq!(current.trim(), "free");
        Ok(())
    }
}
