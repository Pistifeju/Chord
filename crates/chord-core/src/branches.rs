//! A worktree's local branches: list them, switch to one, make a new one.

use std::path::Path;

use serde::Serialize;
use specta::Type;

use crate::{Error, git, worktrees};

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct Branch {
    pub name: String,
    /// Another worktree has this branch checked out, so git will not switch to it here.
    pub held_elsewhere: bool,
}

/// Local branches, the most recent commit first.
pub fn list(worktree: &Path) -> Result<Vec<Branch>, Error> {
    let real = worktree.canonicalize()?;
    let held: Vec<String> = worktrees::entries(worktree)?
        .into_iter()
        .filter(|entry| entry.worktree.path != real)
        .filter_map(|entry| entry.worktree.branch)
        .collect();
    let names = git::run(
        worktree,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short)",
            "refs/heads",
        ],
    )?;
    Ok(names
        .lines()
        .filter(|name| !name.is_empty())
        .map(|name| Branch {
            name: name.to_string(),
            held_elsewhere: held.iter().any(|branch| branch == name),
        })
        .collect())
}

pub fn switch(worktree: &Path, branch: &str) -> Result<(), Error> {
    git::run(worktree, &["switch", branch]).map(|_| ())
}

/// Makes `name` from the worktree's current commit and switches to it.
pub fn create(worktree: &Path, name: &str) -> Result<(), Error> {
    if let Some(reason) = worktrees::branch_name_problem(worktree, name)? {
        return Err(Error::Message(reason));
    }
    git::run(worktree, &["switch", "-c", name]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each commit gets its own committer date, since commits in one second would tie.
    fn commit(repo: &Path, day: u8) -> Result<(), Error> {
        let date = format!("2026-01-{day:02}T00:00:00Z");
        let status = std::process::Command::new("git")
            .current_dir(repo)
            .env("GIT_COMMITTER_DATE", &date)
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(["-c", "commit.gpgsign=false"])
            .args(["commit", "-q", "--allow-empty", "-m", "x"])
            .status()?;
        assert!(status.success());
        Ok(())
    }

    #[test]
    fn lists_newest_first_marks_branches_other_worktrees_hold_and_switches() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-branches-{}", std::process::id()));
        let result = run(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn run(root: &Path) -> Result<(), Error> {
        let repo = root.join("app");
        std::fs::create_dir_all(&repo)?;
        git::run(&repo, &["init", "-q", "-b", "main"])?;
        commit(&repo, 1)?;
        create(&repo, "older")?;
        commit(&repo, 2)?;
        switch(&repo, "main")?;
        let other = root.join("other");
        git::run(
            &repo,
            &["worktree", "add", "-b", "newest", &other.to_string_lossy()],
        )?;
        commit(&other, 3)?;

        let listed = list(&repo)?;
        assert_eq!(
            listed,
            vec![
                Branch {
                    name: "newest".into(),
                    held_elsewhere: true
                },
                Branch {
                    name: "older".into(),
                    held_elsewhere: false
                },
                Branch {
                    name: "main".into(),
                    held_elsewhere: false
                },
            ]
        );
        assert!(switch(&repo, "newest").is_err());
        assert!(create(&repo, "main").is_err());
        assert!(create(&repo, "a b").is_err());
        create(&repo, "fresh")?;
        assert_eq!(
            git::run(&repo, &["branch", "--show-current"])?.trim(),
            "fresh"
        );
        Ok(())
    }
}
