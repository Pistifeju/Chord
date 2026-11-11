//! A worktree's git counts: commits ahead of and behind its upstream, and changed files.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::{Error, git, worktrees};

#[derive(Serialize, Type)]
pub struct GitCounts {
    pub path: PathBuf,
    /// Against the upstream, or the main checkout's branch without one; None on a detached HEAD.
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub changed: u32,
}

/// Counts for each folder; a folder git cannot read is left out.
pub fn read_all(paths: &[PathBuf]) -> Result<Vec<GitCounts>, Error> {
    let mut all = Vec::new();
    for path in paths {
        match read(path) {
            Ok(counts) => all.push(counts),
            Err(error @ Error::MissingTool { .. }) => return Err(error),
            Err(_) => {}
        }
    }
    Ok(all)
}

fn read(dir: &Path) -> Result<GitCounts, Error> {
    let changed = git::run(dir, &["status", "--porcelain"])?.lines().count() as u32;
    let (behind, ahead) = match count_against(dir, "@{upstream}")? {
        Some(sides) => Some(sides),
        None => match main_checkout_branch(dir)? {
            Some(base) => count_against(dir, &base)?,
            None => None,
        },
    }
    .unzip();
    Ok(GitCounts {
        path: dir.to_path_buf(),
        ahead,
        behind,
        changed,
    })
}

/// None when git cannot compare, e.g. no upstream or a detached HEAD.
fn count_against(dir: &Path, base: &str) -> Result<Option<(u32, u32)>, Error> {
    let range = format!("{base}...HEAD");
    match git::run(dir, &["rev-list", "--left-right", "--count", &range]) {
        Ok(output) => Ok(parse_sides(&output)),
        Err(error @ Error::MissingTool { .. }) => Err(error),
        Err(_) => Ok(None),
    }
}

fn main_checkout_branch(dir: &Path) -> Result<Option<String>, Error> {
    let entries = worktrees::entries(dir)?;
    Ok(entries
        .into_iter()
        .next()
        .and_then(|entry| entry.worktree.branch))
}

fn parse_sides(output: &str) -> Option<(u32, u32)> {
    let (behind, ahead) = output.trim().split_once('\t')?;
    Some((behind.parse().ok()?, ahead.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_branch_without_an_upstream_counts_against_the_main_checkout_branch() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-counts-{}", std::process::id()));
        let result = count_in(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn count_in(root: &Path) -> Result<(), Error> {
        let repo = root.join("app");
        std::fs::create_dir_all(&repo)?;
        git::run(&repo, &["init", "-q", "-b", "main"])?;
        let commit = [
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
            "c",
        ];
        git::run(&repo, &commit)?;
        let tree = root.join("tree");
        git::run(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat",
                tree.to_str().unwrap_or_default(),
            ],
        )?;
        git::run(&tree, &commit)?;

        let counts = read(&tree)?;

        assert_eq!((counts.ahead, counts.behind), (Some(1), Some(0)));
        Ok(())
    }

    #[test]
    fn parses_behind_then_ahead() {
        assert_eq!(parse_sides("3\t5\n"), Some((3, 5)));
        assert_eq!(parse_sides("oops"), None);
    }
}
