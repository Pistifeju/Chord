//! The repository's stashes, which every worktree shares: the list newest first, the files one holds with its untracked ones, one file's diff in it, and the stash of a worktree's changes, the apply, pop and drop of a stash.

use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::changes::{self, ChangeStatus, FileDiff};
use crate::commit_diff::{self, CommittedFile};
use crate::{Error, git};

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stash {
    /// The stash's commit, which names it while the `stash@{n}` indexes shift.
    pub hash: String,
    /// Without git's "WIP on main:" or "On main:".
    pub message: String,
    /// The branch it came from, "(no branch)" from a detached HEAD.
    pub branch: String,
    /// When it was made, in milliseconds since 1970.
    pub time: f64,
}

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum StashAction {
    Apply,
    Pop,
    Drop,
}

/// Newest first.
pub fn list(dir: &Path) -> Result<Vec<Stash>, Error> {
    Ok(parse_list(&changes::read(
        dir,
        &["stash", "list", "-z", "--format=%H%x1f%ct%x1f%gs"],
    )?))
}

/// Stashes the worktree's changes, staged, unstaged and untracked.
pub fn push(dir: &Path) -> Result<(), Error> {
    git::run(dir, &["stash", "push", "--include-untracked"]).map(drop)
}

/// A conflicting apply or pop leaves the stash in the list, as git does.
pub fn act(dir: &Path, hash: &str, action: StashAction) -> Result<(), Error> {
    if action != StashAction::Drop {
        git::run(dir, &["stash", "apply", hash])?;
    }
    if action != StashAction::Apply {
        drop_stash(dir, hash)?;
    }
    Ok(())
}

/// git drops a stash only by its `stash@{n}` index, which a stash made meanwhile shifts, so a wrong drop goes back.
fn drop_stash(dir: &Path, hash: &str) -> Result<(), Error> {
    let hashes = changes::read(dir, &["stash", "list", "--format=%H"])?;
    let index = hashes
        .lines()
        .position(|line| line == hash)
        .ok_or_else(|| Error::Message("The stash is gone.".to_string()))?;
    let output = git::run(dir, &["stash", "drop", &format!("stash@{{{index}}}")])?;
    let Some(dropped) = dropped_hash(&output).filter(|dropped| *dropped != hash) else {
        return Ok(());
    };
    let message = changes::read(dir, &["log", "-1", "--format=%s", dropped])?;
    git::run(dir, &["stash", "store", "-m", message.trim(), dropped])?;
    Err(Error::Message(
        "The stashes changed while Chord dropped one. Try again.".to_string(),
    ))
}

/// The commit in git's "Dropped stash@{0} (<hash>)".
fn dropped_hash(output: &str) -> Option<&str> {
    output.trim().rsplit_once(" (")?.1.strip_suffix(')')
}

/// The commit that holds the stash's untracked files, if it has any.
fn untracked_commit(dir: &Path, hash: &str) -> Result<Option<String>, Error> {
    git::optional(git::run(
        dir,
        &["rev-parse", "--verify", "-q", &format!("{hash}^3")],
    ))
}

/// The tracked files against the commit the stash was made on, and the untracked ones, in path order.
pub fn files(dir: &Path, hash: &str) -> Result<Vec<CommittedFile>, Error> {
    let mut files = commit_diff::files_between(dir, &format!("{hash}^1"), hash)?;
    if let Some(untracked) = untracked_commit(dir, hash)? {
        files.extend(
            commit_diff::commit_files(dir, &untracked)?
                .into_iter()
                .map(|file| CommittedFile {
                    status: ChangeStatus::Untracked,
                    ..file
                }),
        );
        files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    Ok(files)
}

pub fn diff(dir: &Path, path: &str, hash: &str) -> Result<FileDiff, Error> {
    if let Some(untracked) = untracked_commit(dir, hash)?
        && !changes::read(dir, &["ls-tree", "--name-only", &untracked, "--", path])?.is_empty()
    {
        return commit_diff::commit_diff(dir, &untracked, path);
    }
    commit_diff::diff_between(dir, &format!("{hash}^1"), hash, path)
}

/// Each record of `git stash list -z` with the fields 1F apart: hash, time in seconds, reflog subject.
fn parse_list(output: &str) -> Vec<Stash> {
    output
        .split('\0')
        .filter_map(|record| {
            let mut fields = record.trim_start_matches('\n').splitn(3, '\x1f');
            let hash = fields.next().filter(|hash| !hash.is_empty())?.to_string();
            let time = fields.next()?.parse::<f64>().ok()? * 1000.0;
            let subject = fields.next()?;
            // A ref name holds no colon, so the first ": " ends the branch.
            let (branch, message) = subject
                .strip_prefix("WIP on ")
                .or_else(|| subject.strip_prefix("On "))
                .and_then(|rest| rest.split_once(": "))
                .unwrap_or(("", subject));
            Some(Stash {
                hash,
                message: message.to_string(),
                branch: branch.to_string(),
                time,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stash_list_splits_the_branch_from_the_message() {
        let stashes = parse_list(
            "aaa\x1f1700000000\x1fWIP on main: a41c9e2 Add the menu\0\nbbb\x1f1600000000\x1fOn feature/x: Try: this\0",
        );
        let read: Vec<_> = stashes
            .iter()
            .map(|s| {
                (
                    s.hash.as_str(),
                    s.branch.as_str(),
                    s.message.as_str(),
                    s.time,
                )
            })
            .collect();
        assert_eq!(
            read,
            [
                ("aaa", "main", "a41c9e2 Add the menu", 1_700_000_000_000.0),
                ("bbb", "feature/x", "Try: this", 1_600_000_000_000.0),
            ]
        );
    }

    #[test]
    fn a_drop_names_the_stash_it_dropped() {
        assert_eq!(
            dropped_hash("Dropped stash@{0} (2f3e9c1d4b5a)\n"),
            Some("2f3e9c1d4b5a")
        );
    }

    #[test]
    fn a_stash_holds_the_untracked_files_and_pops_back() -> Result<(), Error> {
        let repo = std::env::temp_dir().join(format!("chord-stash-{}", std::process::id()));
        let result = stash_in(&repo);
        let _ = std::fs::remove_dir_all(&repo);
        result
    }

    fn stash_in(repo: &Path) -> Result<(), Error> {
        std::fs::create_dir_all(repo)?;
        git::run(repo, &["init", "-q", "-b", "main"])?;
        std::fs::write(repo.join("a.rs"), "one\n")?;
        git::run(repo, &["add", "-A"])?;
        git::run(
            repo,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "first",
            ],
        )?;
        std::fs::write(repo.join("a.rs"), "one\ntwo\n")?;
        std::fs::write(repo.join("new.rs"), "new\n")?;
        git::run(
            repo,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "stash",
                "push",
                "--include-untracked",
            ],
        )?;

        let stashes = list(repo)?;
        assert_eq!(stashes.len(), 1);
        assert_eq!(stashes[0].branch, "main");
        let held: Vec<_> = files(repo, &stashes[0].hash)?
            .into_iter()
            .map(|f| (f.path, f.status, f.added))
            .collect();
        assert_eq!(
            held,
            [
                ("a.rs".to_string(), ChangeStatus::Modified, Some(1)),
                ("new.rs".to_string(), ChangeStatus::Untracked, Some(1)),
            ]
        );
        assert_eq!(diff(repo, "new.rs", &stashes[0].hash)?.hunks.len(), 1);
        assert!(!repo.join("new.rs").exists());

        act(repo, &stashes[0].hash, StashAction::Pop)?;
        assert_eq!(list(repo)?, []);
        assert_eq!(std::fs::read_to_string(repo.join("a.rs"))?, "one\ntwo\n");
        assert!(repo.join("new.rs").exists());
        Ok(())
    }
}
