use std::path::Path;

use serde::Serialize;
use specta::Type;

use crate::commit_diff::{self, CommittedFile};
use crate::diff::FileDiff;
use crate::{Error, git, worktrees};

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    pub time: f64,
}

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchChanges {
    pub base: String,
    pub default_base: String,
    pub commits: Vec<Commit>,
    pub files: Vec<CommittedFile>,
}

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseBranches {
    pub local: Vec<String>,
    pub remote: Vec<String>,
}

fn current_branch(dir: &Path) -> Result<Option<String>, Error> {
    git::optional(git::run(dir, &["symbolic-ref", "-q", "--short", "HEAD"]))
}

fn default_base(dir: &Path) -> Result<Option<String>, Error> {
    if current_branch(dir)?.is_none() {
        return Ok(None);
    }
    let entries = worktrees::entries(dir)?;
    let Some(main) = entries.first() else {
        return Ok(None);
    };
    let here = dir.canonicalize()?;
    if main.worktree.path.canonicalize().ok().as_deref() != Some(here.as_path()) {
        return Ok(main.worktree.branch.clone());
    }
    git::optional(git::run(
        dir,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    ))
}

fn merge_base(dir: &Path, base: &str) -> Result<Option<String>, Error> {
    git::optional(git::run(dir, &["merge-base", base, "HEAD"]))
}

pub fn read(dir: &Path, base: Option<&str>) -> Result<Option<BranchChanges>, Error> {
    let Some(default_base) = default_base(dir)? else {
        return Ok(None);
    };
    let picked = match base {
        Some(base) => merge_base(dir, base)?.map(|from| (base.to_string(), from)),
        None => None,
    };
    let found = match picked {
        Some(picked) => Some(picked),
        None => merge_base(dir, &default_base)?.map(|from| (default_base.clone(), from)),
    };
    let Some((base, from)) = found else {
        return Ok(None);
    };
    let log = git::read(
        dir,
        &[
            "log",
            "-z",
            "--format=%H%x1f%h%x1f%ct%x1f%s",
            &format!("{base}..HEAD"),
            "--",
        ],
    )?;
    Ok(Some(BranchChanges {
        files: commit_diff::files_between(dir, &from, "HEAD")?,
        commits: parse_log(&log),
        base,
        default_base,
    }))
}

pub fn bases(dir: &Path) -> Result<BaseBranches, Error> {
    let own = current_branch(dir)?;
    let refs = git::read(
        dir,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;
    let mut branches = BaseBranches {
        local: Vec::new(),
        remote: Vec::new(),
    };
    for name in refs.lines() {
        if let Some(local) = name.strip_prefix("refs/heads/") {
            if own.as_deref() != Some(local) {
                branches.local.push(local.to_string());
            }
        } else if let Some(remote) = name.strip_prefix("refs/remotes/")
            && !remote.ends_with("/HEAD")
        {
            branches.remote.push(remote.to_string());
        }
    }
    Ok(branches)
}

pub fn diff(dir: &Path, path: &str, base: &str) -> Result<FileDiff, Error> {
    let from = merge_base(dir, base)?
        .ok_or_else(|| Error::Message(format!("This branch shares no history with {base}.")))?;
    commit_diff::diff_between(dir, &from, "HEAD", path)
}

fn parse_log(output: &str) -> Vec<Commit> {
    output
        .split('\0')
        .filter_map(|record| {
            let mut fields = record.trim_start_matches('\n').splitn(4, '\x1f');
            Some(Commit {
                hash: fields.next().filter(|hash| !hash.is_empty())?.to_string(),
                short_hash: fields.next()?.to_string(),
                time: fields.next()?.parse::<f64>().ok()? * 1000.0,
                subject: fields.next()?.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changes::ChangeStatus;

    #[test]
    fn a_branch_lists_its_commits_and_files_since_the_main_checkout_branch() -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-branch-{}", std::process::id()));
        let result = branch_in(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn commit(dir: &Path, message: &str) -> Result<(), Error> {
        git::run(dir, &["add", "-A"])?;
        git::run(
            dir,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                message,
            ],
        )
        .map(drop)
    }

    fn branch_in(root: &Path) -> Result<(), Error> {
        let repo = root.join("app");
        std::fs::create_dir_all(&repo)?;
        git::run(&repo, &["init", "-q", "-b", "main"])?;
        std::fs::write(repo.join("a.rs"), "one\n")?;
        commit(&repo, "first")?;
        let tree = root.join("tree");
        let tree_path = tree.to_str().unwrap_or_default();
        git::run(&repo, &["worktree", "add", "-q", "-b", "feat", tree_path])?;
        std::fs::write(tree.join("a.rs"), "one\ntwo\n")?;
        commit(&tree, "add two")?;
        std::fs::write(tree.join("b.rs"), "new\n")?;
        commit(&tree, "add b")?;
        std::fs::write(repo.join("c.rs"), "main\n")?;
        commit(&repo, "on main")?;

        let branch = read(&tree, None)?.expect("a base");

        assert_eq!(branch.base, "main");
        let subjects: Vec<_> = branch.commits.iter().map(|c| c.subject.as_str()).collect();
        assert_eq!(subjects, ["add b", "add two"]);
        let files: Vec<_> = branch
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.status, f.added, f.removed))
            .collect();
        assert_eq!(
            files,
            [
                ("a.rs", ChangeStatus::Modified, Some(1), Some(0)),
                ("b.rs", ChangeStatus::Added, Some(1), Some(0)),
            ]
        );
        let last = commit_diff::commit_files(&tree, &branch.commits[0].hash)?;
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].path, "b.rs");
        assert_eq!(read(&repo, None)?, None);
        Ok(())
    }
}
