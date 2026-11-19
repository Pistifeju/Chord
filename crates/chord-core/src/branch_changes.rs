//! A worktree's branch against a base: the base it compares with by default, the branches it can compare with instead, its commits since the merge base, the files the branch or one commit changed, and one file's diff in either.

use std::path::Path;

use serde::Serialize;
use specta::Type;

use crate::changes::{self, ChangeStatus, FileDiff};
use crate::{DiffSide, Error, git, worktrees};

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommittedFile {
    pub path: String,
    /// The path a rename came from.
    pub old_path: Option<String>,
    pub status: ChangeStatus,
    /// None for a binary file.
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
    /// When it was committed, in milliseconds since 1970.
    pub time: f64,
}

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchChanges {
    /// The base the commits and files compare with: the one asked for while it exists, else the default.
    pub base: String,
    /// The main checkout's branch, or the upstream on the main checkout itself.
    pub default_base: String,
    /// Newest first.
    pub commits: Vec<Commit>,
    pub files: Vec<CommittedFile>,
}

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseBranches {
    /// Without the worktree's own branch.
    pub local: Vec<String>,
    /// As `origin/main`, without the remotes' HEAD.
    pub remote: Vec<String>,
}

/// Other git failures read as None, so a detached HEAD or a branch without an upstream only hides the Branch scope.
fn optional(result: Result<String, Error>) -> Result<Option<String>, Error> {
    match result {
        Ok(output) => Ok(Some(output.trim().to_string())),
        Err(error @ Error::MissingTool { .. }) => Err(error),
        Err(_) => Ok(None),
    }
}

fn current_branch(dir: &Path) -> Result<Option<String>, Error> {
    optional(git::run(dir, &["symbolic-ref", "-q", "--short", "HEAD"]))
}

/// The main checkout's branch; on the main checkout itself its upstream; None on a detached HEAD.
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
    optional(git::run(
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
    optional(git::run(dir, &["merge-base", base, "HEAD"]))
}

/// The branch's commits and files against `base`, or the default base when `base` is None or gone; None without a default base or a shared history.
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
    let log = changes::read(
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
        files: files_between(dir, &from, "HEAD")?,
        commits: parse_log(&log),
        base,
        default_base,
    }))
}

/// The branches a worktree can compare with, local and remote.
pub fn bases(dir: &Path) -> Result<BaseBranches, Error> {
    let own = current_branch(dir)?;
    let refs = changes::read(
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

/// The commit's first parent, or git's empty tree for a root commit.
fn parent(dir: &Path, commit: &str) -> Result<String, Error> {
    match optional(git::run(
        dir,
        &["rev-parse", "--verify", "-q", &format!("{commit}^")],
    ))? {
        Some(parent) => Ok(parent),
        None => Ok(
            git::run_with_input(dir, &["hash-object", "-t", "tree", "--stdin"], b"")?
                .trim()
                .to_string(),
        ),
    }
}

/// The files the commit changed against its first parent.
pub fn commit_files(dir: &Path, commit: &str) -> Result<Vec<CommittedFile>, Error> {
    files_between(dir, &parent(dir, commit)?, commit)
}

fn files_between(dir: &Path, from: &str, to: &str) -> Result<Vec<CommittedFile>, Error> {
    let names = changes::read(dir, &["diff", "-M", "--name-status", "-z", from, to, "--"])?;
    let counts = changes::parse_numstat(&changes::read(
        dir,
        &["diff", "-M", "--numstat", "-z", from, to, "--"],
    )?);
    let mut files = parse_name_status(&names);
    for file in &mut files {
        (file.added, file.removed) = counts.get(&file.path).copied().flatten().unzip();
    }
    Ok(files)
}

/// The file's diff on the branch since its merge base with the base, or in the one commit.
pub fn diff(dir: &Path, path: &str, side: &DiffSide) -> Result<FileDiff, Error> {
    let (from, to) = match side {
        DiffSide::Branch(base) => (
            merge_base(dir, base)?.ok_or_else(|| {
                Error::Message(format!("This branch shares no history with {base}."))
            })?,
            "HEAD".to_string(),
        ),
        DiffSide::Commit(commit) => (parent(dir, commit)?, commit.clone()),
        DiffSide::Staged | DiffSide::Unstaged => return changes::diff(dir, path, side),
    };
    // A rename shows against the file it came from, so git needs both paths.
    let names = changes::read(
        dir,
        &["diff", "-M", "--name-status", "-z", &from, &to, "--"],
    )?;
    let old_path = parse_name_status(&names)
        .into_iter()
        .find(|file| file.path == path)
        .and_then(|file| file.old_path);
    let mut args = vec![
        "diff",
        "--no-color",
        "--no-ext-diff",
        "-M",
        &from,
        &to,
        "--",
    ];
    args.extend(old_path.as_deref());
    args.push(path);
    Ok(changes::parse_diff(&changes::read(dir, &args)?))
}

/// Each record of `git log -z` with the fields 1F apart: hash, short hash, commit time in seconds, subject.
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

/// Each record of `git diff --name-status -z`: the status, then the path, or a rename's or copy's old and new path.
fn parse_name_status(output: &str) -> Vec<CommittedFile> {
    let mut files = Vec::new();
    let mut records = output.split('\0');
    while let Some(code) = records.next() {
        let Some(status) = code.bytes().next().and_then(changes::status_of) else {
            continue;
        };
        let first = records.next().unwrap_or_default();
        let (old_path, path) = if matches!(code.as_bytes().first(), Some(b'R' | b'C')) {
            (Some(first), records.next().unwrap_or_default())
        } else {
            (None, first)
        };
        files.push(CommittedFile {
            path: path.to_string(),
            // A copy leaves its source in place, so only a rename keeps the old path.
            old_path: old_path
                .filter(|_| status == ChangeStatus::Renamed)
                .map(str::to_string),
            status,
            added: None,
            removed: None,
        });
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // A commit on main after the branch split off changes nothing on the branch.
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
        let last = commit_files(&tree, &branch.commits[0].hash)?;
        assert_eq!(last.len(), 1);
        assert_eq!(last[0].path, "b.rs");
        // The main checkout has no upstream, so it has no base.
        assert_eq!(read(&repo, None)?, None);
        Ok(())
    }

    #[test]
    fn a_name_status_keeps_a_rename_old_path() {
        let files = parse_name_status("M\0a.rs\0R087\0old.rs\0new.rs\0D\0gone.rs\0");
        let read: Vec<_> = files
            .iter()
            .map(|f| (f.path.as_str(), f.old_path.as_deref(), f.status))
            .collect();
        assert_eq!(
            read,
            [
                ("a.rs", None, ChangeStatus::Modified),
                ("new.rs", Some("old.rs"), ChangeStatus::Renamed),
                ("gone.rs", None, ChangeStatus::Deleted),
            ]
        );
    }
}
