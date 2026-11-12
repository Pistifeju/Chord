//! A project's worktrees, read from `git worktree list`.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::{Error, git, harness::sessions::Sessions, projects::Project};

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Entry {
    pub worktree: Worktree,
    pub bare: bool,
    pub prunable: bool,
}

pub fn list(project: &Path) -> Result<Vec<Worktree>, Error> {
    Ok(entries(project)?
        .into_iter()
        .filter(|entry| !entry.bare && !entry.prunable)
        .map(|entry| entry.worktree)
        .collect())
}

/// None when `name` can become a new branch with a free worktree folder, else the reason it cannot.
pub fn check_branch(project: &Project, name: &str) -> Result<Option<String>, Error> {
    // `git check-ref-format` exits 1 with no output for a bad ref, which `git::run` reports as an empty message.
    // `--branch` would print a fatal message instead, so check the full ref and reject a leading dash by hand.
    if name.starts_with('-') {
        return Ok(Some("Not a valid branch name".into()));
    }
    match git::run(
        &project.path,
        &["check-ref-format", &format!("refs/heads/{name}")],
    ) {
        Ok(_) => {}
        Err(Error::Message(m)) if m.is_empty() => {
            return Ok(Some("Not a valid branch name".into()));
        }
        Err(other) => return Err(other),
    }
    let taken = git::run(
        &project.path,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ],
    )
    .is_ok();
    if taken {
        return Ok(Some("A branch with this name exists".into()));
    }
    Ok(folder(project, name)
        .exists()
        .then(|| "A folder with this name exists".into()))
}

/// Adds a worktree on a new branch in `<project>-worktrees/<branch>` beside the project.
pub fn create(project: &Project, branch: &str) -> Result<Worktree, Error> {
    if let Some(reason) = check_branch(project, branch)? {
        return Err(Error::Message(reason));
    }
    let path = folder(project, branch);
    let path_arg = path.to_string_lossy();
    git::run(&project.path, &["worktree", "add", "-b", branch, &path_arg])?;
    copy_ignored_files(&project.path, &path);
    Ok(Worktree {
        path,
        branch: Some(branch.to_string()),
    })
}

const MAX_COPIED_FILE_BYTES: u64 = 5 * 1024 * 1024;

// A failed copy leaves the worktree usable, so it never fails the creation.
fn copy_ignored_files(from: &Path, to: &Path) {
    let listing = match git::run(
        from,
        &[
            "ls-files",
            "-z",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
        ],
    ) {
        Ok(listing) => listing,
        Err(error) => {
            eprintln!("chord: listing gitignored files failed: {error}");
            return;
        }
    };
    for relative in listing
        .split('\0')
        .filter(|item| !item.is_empty() && !item.ends_with('/'))
    {
        let source = from.join(relative);
        let Ok(metadata) = std::fs::symlink_metadata(&source) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > MAX_COPIED_FILE_BYTES {
            continue;
        }
        let target = to.join(relative);
        if target.exists() {
            continue;
        }
        let copied = match target.parent() {
            Some(parent) => std::fs::create_dir_all(parent),
            None => Ok(()),
        }
        .and_then(|()| std::fs::copy(&source, &target));
        if let Err(error) = copied {
            eprintln!("chord: copying gitignored {relative} failed: {error}");
        }
    }
}

#[derive(Serialize, Type, Debug, PartialEq)]
pub enum Removal {
    /// The worktree is gone; `kept_branch` is its branch when it stays.
    Removed { kept_branch: Option<KeptBranch> },
    /// Git refused over modified or untracked files or a lock, which `--force` overrides.
    NeedsForce { reason: String },
}

#[derive(Serialize, Type, Debug, PartialEq)]
pub struct KeptBranch {
    pub name: String,
    pub reason: String,
}

/// Stops the worktree's agent, removes the worktree and deletes its branch when it is merged.
pub fn remove(
    sessions: &Sessions,
    project: &Project,
    path: &Path,
    force: bool,
) -> Result<Removal, Error> {
    sessions.stop(path);
    // git lists real paths, so a path through a symlink must resolve before it is compared.
    let real = path.canonicalize()?;
    let branch = entries(&project.path)?
        .into_iter()
        .find(|entry| entry.worktree.path == real)
        .and_then(|entry| entry.worktree.branch);
    let path_arg = path.to_string_lossy();
    let mut args = vec!["worktree", "remove"];
    if force {
        // A locked worktree needs the flag twice.
        args.extend(["--force", "--force"]);
    }
    args.push(&path_arg);
    match git::run(&project.path, &args) {
        Ok(_) => {}
        Err(Error::Message(reason)) if !force && force_helps(&reason) => {
            return Ok(Removal::NeedsForce { reason });
        }
        Err(other) => return Err(other),
    }
    let kept_branch = branch.and_then(|name| {
        let Err(Error::Message(message)) = git::run(&project.path, &["branch", "-d", &name]) else {
            return None;
        };
        let reason = if message.contains("not fully merged") {
            "it is not merged".to_string()
        } else {
            message
        };
        Some(KeptBranch { name, reason })
    });
    Ok(Removal::Removed { kept_branch })
}

fn force_helps(refusal: &str) -> bool {
    refusal.contains("modified or untracked files") || refusal.contains("locked")
}

fn folder(project: &Project, branch: &str) -> PathBuf {
    project
        .path
        .with_file_name(format!("{}-worktrees", project.name))
        .join(branch.replace('/', "-"))
}

pub(crate) fn entries(dir: &Path) -> Result<Vec<Entry>, Error> {
    Ok(parse(&git::run(
        dir,
        &["worktree", "list", "--porcelain", "-z"],
    )?))
}

fn parse(porcelain: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;
    for field in porcelain.split('\0') {
        if field.is_empty() {
            entries.extend(current.take());
            continue;
        }
        let (key, value) = field.split_once(' ').unwrap_or((field, ""));
        match (key, current.as_mut()) {
            ("worktree", _) => {
                entries.extend(current.take());
                current = Some(Entry {
                    worktree: Worktree {
                        path: PathBuf::from(value),
                        branch: None,
                    },
                    bare: false,
                    prunable: false,
                });
            }
            ("branch", Some(entry)) => {
                let name = value.strip_prefix("refs/heads/").unwrap_or(value);
                entry.worktree.branch = Some(name.to_string());
            }
            ("bare", Some(entry)) => entry.bare = true,
            ("prunable", Some(entry)) => entry.prunable = true,
            _ => {}
        }
    }
    entries.extend(current);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_adds_a_worktree_on_a_new_branch_and_check_rejects_unusable_names() -> Result<(), Error>
    {
        let root = std::env::temp_dir().join(format!("chord-create-{}", std::process::id()));
        let result = create_and_check(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn create_and_check(root: &Path) -> Result<(), Error> {
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
        let project = Project {
            path: repo.clone(),
            name: "app".into(),
        };

        let bad = check_branch(&project, "a b")?;
        let taken = check_branch(&project, "main")?;
        let made = create(&project, "feature/x")?;
        let clash = check_branch(&project, "feature-x")?;
        let listed = list(&repo)?;
        let unmerged = create(&project, "wip")?;
        git::run(
            &unmerged.path,
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
                "more",
            ],
        )?;
        let sessions = Sessions::default();
        let merged_removal = remove(&sessions, &project, &made.path, false)?;
        let unmerged_removal = remove(&sessions, &project, &unmerged.path, false)?;

        assert_eq!(merged_removal, Removal::Removed { kept_branch: None });
        assert_eq!(
            unmerged_removal,
            Removal::Removed {
                kept_branch: Some(KeptBranch {
                    name: "wip".into(),
                    reason: "it is not merged".into()
                })
            }
        );
        assert!(!made.path.exists());
        assert!(matches!(
            remove(&sessions, &project, &repo, false),
            Err(Error::Message(_))
        ));
        assert!(
            git::run(
                &repo,
                &["rev-parse", "--verify", "--quiet", "refs/heads/feature/x"]
            )
            .is_err()
        );
        assert_eq!(bad.as_deref(), Some("Not a valid branch name"));
        assert_eq!(taken.as_deref(), Some("A branch with this name exists"));
        assert_eq!(clash.as_deref(), Some("A folder with this name exists"));
        assert_eq!(made.path, root.join("app-worktrees").join("feature-x"));
        assert!(
            listed
                .iter()
                .any(|w| w.branch.as_deref() == Some("feature/x"))
        );
        Ok(())
    }

    #[test]
    fn parses_branches_detached_and_prunable_worktrees() {
        let porcelain = "worktree /repo\0HEAD 1111\0branch refs/heads/main\0\0\
            worktree /repo-wt/fix\nline\0HEAD 2222\0detached\0\0\
            worktree /gone\0HEAD 3333\0branch refs/heads/feature/old\0prunable gitdir file points to non-existent location\0\0";
        let entry = |path: &str, branch: Option<&str>, prunable| Entry {
            worktree: Worktree {
                path: PathBuf::from(path),
                branch: branch.map(String::from),
            },
            bare: false,
            prunable,
        };
        assert_eq!(
            parse(porcelain),
            vec![
                entry("/repo", Some("main"), false),
                entry("/repo-wt/fix\nline", None, false),
                entry("/gone", Some("feature/old"), true),
            ]
        );
    }
}
