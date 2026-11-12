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
    // shortcut: one thread per folder, cap them once a window shows hundreds of worktrees.
    let results: Vec<_> = std::thread::scope(|scope| {
        let threads: Vec<_> = paths
            .iter()
            .map(|path| scope.spawn(move || read(path)))
            .collect();
        threads.into_iter().map(|thread| thread.join()).collect()
    });
    let mut all = Vec::new();
    for result in results {
        match result {
            Ok(Ok(counts)) => all.push(counts),
            Ok(Err(error @ Error::MissingTool { .. })) => return Err(error),
            _ => {}
        }
    }
    Ok(all)
}

fn read(dir: &Path) -> Result<GitCounts, Error> {
    let status = git::run(
        dir,
        &["status", "--porcelain=v2", "--branch", "--ahead-behind"],
    )?;
    let (against_upstream, changed) = parse_status(&status);
    let (behind, ahead) = match against_upstream {
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

/// The (behind, ahead) counts against the upstream when the branch has one, and the number of changed files.
fn parse_status(output: &str) -> (Option<(u32, u32)>, u32) {
    let mut sides = None;
    let mut changed = 0;
    for line in output.lines() {
        match line.strip_prefix("# ") {
            Some(header) => {
                if let Some(counts) = header.strip_prefix("branch.ab +") {
                    sides = counts.split_once(" -").and_then(|(ahead, behind)| {
                        Some((behind.parse().ok()?, ahead.parse().ok()?))
                    });
                }
            }
            None => changed += 1,
        }
    }
    (sides, changed)
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
    fn a_status_gives_the_upstream_sides_and_the_changed_files() {
        let with_upstream = "# branch.oid 1111\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +3 -5\n1 .M N... 100644 100644 100644 1111 1111 a.rs\n? b.rs\n";
        assert_eq!(parse_status(with_upstream), (Some((5, 3)), 2));
        assert_eq!(
            parse_status("# branch.oid 1111\n# branch.head feat\n"),
            (None, 0)
        );
    }

    #[test]
    fn parses_behind_then_ahead() {
        assert_eq!(parse_sides("3\t5\n"), Some((3, 5)));
        assert_eq!(parse_sides("oops"), None);
    }
}
