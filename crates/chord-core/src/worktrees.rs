//! A project's worktrees, read from `git worktree list`.

use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::{Error, Terminals, git, harness::sessions::Sessions, projects::Project};

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    /// The folder holds an Xcode project or workspace.
    pub xcode: bool,
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
        .map(|mut entry| {
            entry.worktree.xcode = !xcode_bundles(&entry.worktree.path).is_empty();
            entry.worktree
        })
        .collect())
}

/// The Xcode projects and workspaces directly in the folder, sorted by path.
pub fn xcode_bundles(folder: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("xcodeproj" | "xcworkspace")
            )
        })
        .collect();
    found.sort();
    found
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
        xcode: !xcode_bundles(&path).is_empty(),
        path,
        branch: Some(branch.to_string()),
    })
}

const MAX_COPIED_FILE_BYTES: u64 = 5 * 1024 * 1024;

// Xcode rebuilds DerivedData per checkout path, so a clone of it is never used.
const NEVER_CLONED: &str = "DerivedData";

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
    for relative in listing.split('\0').filter(|item| !item.is_empty()) {
        // Components drop the listing's trailing `/` without touching non-UTF-8 bytes.
        let relative_path: PathBuf = Path::new(relative).components().collect();
        if relative_path
            .components()
            .any(|part| part.as_os_str() == NEVER_CLONED)
        {
            continue;
        }
        let source = from.join(&relative_path);
        let target = to.join(&relative_path);
        let Ok(metadata) = std::fs::symlink_metadata(&source) else {
            continue;
        };
        if (!metadata.is_dir() && !metadata.is_file()) || target.exists() {
            continue;
        }
        if let Err(error) = clone_or_copy(&source, &target, &metadata) {
            eprintln!("chord: placing gitignored {relative} failed: {error}");
        }
    }
}

// Without clone support a folder is skipped, so ignored folders never cost a full copy.
fn clone_or_copy(
    source: &Path,
    target: &Path,
    metadata: &std::fs::Metadata,
) -> std::io::Result<()> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match clone(source, target) {
        Ok(()) => {
            if metadata.is_dir() {
                remove_nested_never_cloned(target);
            }
            Ok(())
        }
        Err(error) if is_clone_unsupported(&error) => {
            if metadata.is_dir() || metadata.len() > MAX_COPIED_FILE_BYTES {
                return Ok(());
            }
            std::fs::copy(source, target).map(|_| ())
        }
        Err(error) => Err(error),
    }
}

// A cloned ignored folder such as `build/` can hold a DerivedData folder.
fn remove_nested_never_cloned(folder: &Path) {
    let Ok(children) = std::fs::read_dir(folder) else {
        return;
    };
    for child in children.flatten() {
        let Ok(kind) = child.file_type() else {
            continue;
        };
        if !kind.is_dir() {
            continue;
        }
        if child.file_name() == NEVER_CLONED {
            if let Err(error) = std::fs::remove_dir_all(child.path()) {
                eprintln!("chord: removing cloned {NEVER_CLONED} failed: {error}");
            }
        } else {
            remove_nested_never_cloned(&child.path());
        }
    }
}

#[cfg(target_os = "macos")]
fn clone(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let cstring = |path: &Path| {
        std::ffi::CString::new(path.as_os_str().as_bytes())
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))
    };
    let (source, target) = (cstring(source)?, cstring(target)?);
    // SAFETY: both pointers are NUL-terminated strings that outlive the call.
    if unsafe { libc::clonefile(source.as_ptr(), target.as_ptr(), 0) } == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "macos"))]
fn clone(_source: &Path, _target: &Path) -> std::io::Result<()> {
    Err(std::io::Error::from_raw_os_error(libc::ENOTSUP))
}

// ENOTSUP: the volume has no clones. EXDEV: the two sides are on different volumes.
fn is_clone_unsupported(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::ENOTSUP | libc::EXDEV))
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

/// Stops the worktree's agents, removes the worktree, forgets its panes and deletes its branch when it is merged.
pub fn remove(
    sessions: &Sessions,
    terminals: &Terminals,
    project: &Project,
    path: &Path,
    force: bool,
) -> Result<Removal, Error> {
    // Git's own check for a dirty worktree, asked before the agents stop, so a refusal the user cancels leaves them running.
    if !force {
        let status = git::run(
            path,
            &[
                "--no-optional-locks",
                "status",
                "--porcelain",
                "--ignore-submodules=none",
            ],
        )?;
        if !status.trim().is_empty() {
            return Ok(Removal::NeedsForce {
                reason: format!("{} contains modified or untracked files", path.display()),
            });
        }
    }
    terminals.stop(path);
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
    sessions.forget(path)?;
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
                        xcode: false,
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
        let sessions = Sessions::new(
            std::sync::Arc::new(crate::Store::open(root.join("state.json"))?),
            root.join("sessions"),
        );
        let merged_removal = remove(
            &sessions,
            &Terminals::default(),
            &project,
            &made.path,
            false,
        )?;
        let unmerged_removal = remove(
            &sessions,
            &Terminals::default(),
            &project,
            &unmerged.path,
            false,
        )?;

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
            remove(&sessions, &Terminals::default(), &project, &repo, false),
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
                xcode: false,
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
