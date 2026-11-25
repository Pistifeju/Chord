use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::branches::{self, Deletion};
use crate::{Error, git, projects::Project, state::place_in};

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub xcode: bool,
    pub git: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Entry {
    pub worktree: Worktree,
    pub bare: bool,
    pub prunable: bool,
}

pub(crate) fn folder_at(path: &Path) -> Result<(), Error> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "{} is not a folder",
            path.display()
        )))
    }
}

pub fn list(project: &Path) -> Result<Vec<Worktree>, Error> {
    folder_at(project)?;
    if !git::in_repository(project) {
        return Ok(vec![Worktree {
            path: project.to_path_buf(),
            branch: None,
            xcode: !xcode_bundles(project).is_empty(),
            git: false,
        }]);
    }
    Ok(entries(project)?
        .into_iter()
        .filter(|entry| !entry.bare && !entry.prunable)
        .map(|mut entry| {
            entry.worktree.xcode = !xcode_bundles(&entry.worktree.path).is_empty();
            entry.worktree
        })
        .collect())
}

pub fn in_order(mut list: Vec<Worktree>, order: &[PathBuf]) -> Vec<Worktree> {
    list.sort_by_key(|w| place_in(order, &w.path));
    list
}

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

pub fn check_new_branch(project: &Project, name: &str) -> Result<Option<String>, Error> {
    branch_problem(project, name, None)
}

pub fn check_branch(project: &Project, name: &str) -> Result<Option<String>, Error> {
    if let Some(reason) = branch_problem(project, name, None)? {
        return Ok(Some(reason));
    }
    Ok(folder(project, name)?
        .exists()
        .then(|| "A folder with this name exists".into()))
}

pub fn check_branch_rename(
    project: &Project,
    current: &str,
    name: &str,
) -> Result<Option<String>, Error> {
    if current == name {
        return Ok(None);
    }
    branch_problem(project, name, Some(current))
}

pub fn rename_branch(project: &Project, current: &str, name: &str) -> Result<(), Error> {
    if let Some(reason) = check_branch_rename(project, current, name)? {
        return Err(Error::Message(reason));
    }
    if current == name {
        return Ok(());
    }
    let flag = if current.eq_ignore_ascii_case(name) {
        "-M"
    } else {
        "-m"
    };
    git::run(&project.path, &["branch", flag, "--", current, name]).map(|_| ())
}

fn branch_problem(
    project: &Project,
    name: &str,
    renaming: Option<&str>,
) -> Result<Option<String>, Error> {
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
    if renaming.is_some_and(|current| current.eq_ignore_ascii_case(name)) {
        return Ok(None);
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
    Ok(taken.then(|| "A branch with this name exists".into()))
}

pub fn create(project: &Project, branch: &str) -> Result<Worktree, Error> {
    if let Some(reason) = check_branch(project, branch)? {
        return Err(Error::Message(reason));
    }
    let path = folder(project, branch)?;
    let path_arg = path.to_string_lossy();
    git::run(&project.path, &["worktree", "add", "-b", branch, &path_arg])?;
    copy_ignored_files(&project.path, &path);
    Ok(Worktree {
        xcode: !xcode_bundles(&path).is_empty(),
        path,
        branch: Some(branch.to_string()),
        git: true,
    })
}

const MAX_COPIED_FILE_BYTES: u64 = 5 * 1024 * 1024;

const NEVER_CLONED: &str = "DerivedData";

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

fn is_clone_unsupported(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(libc::ENOTSUP | libc::EXDEV))
}

#[derive(Serialize, Type, Debug, PartialEq)]
pub enum Removal {
    Removed { kept_branch: Option<KeptBranch> },
    NeedsForce { reason: String },
}

#[derive(Serialize, Type, Debug, PartialEq)]
pub struct KeptBranch {
    pub name: String,
    pub reason: String,
}

pub fn refusal(path: &Path, force: bool) -> Result<Option<Removal>, Error> {
    if force {
        return Ok(None);
    }
    let status = git::run(
        path,
        &[
            "--no-optional-locks",
            "status",
            "--porcelain",
            "--ignore-submodules=none",
        ],
    )?;
    Ok((!status.trim().is_empty()).then(|| Removal::NeedsForce {
        reason: format!("{} contains modified or untracked files", path.display()),
    }))
}

pub fn remove_folder(
    project: &Project,
    path: &Path,
    force: bool,
) -> Result<Result<Option<String>, Removal>, Error> {
    let real = path.canonicalize()?;
    let branch = entries(&project.path)?
        .into_iter()
        .find(|entry| entry.worktree.path == real)
        .and_then(|entry| entry.worktree.branch);
    let path_arg = path.to_string_lossy();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.extend(["--force", "--force"]);
    }
    args.push(&path_arg);
    match git::run(&project.path, &args) {
        Ok(_) => {
            if let Some(parent) = real.parent()
                && worktrees_folders(project)
                    .iter()
                    .any(|folder| folder.canonicalize().is_ok_and(|f| f == parent))
            {
                let _ = std::fs::remove_dir(parent);
            }
            Ok(Ok(branch))
        }
        Err(Error::Message(reason)) if !force && force_helps(&reason) => {
            Ok(Err(Removal::NeedsForce { reason }))
        }
        Err(other) => Err(other),
    }
}

pub fn delete_merged_branch(project: &Project, name: String) -> Option<KeptBranch> {
    let reason = match branches::delete(&project.path, &name, false) {
        Ok(Deletion::Unmerged) => "it is not merged".to_string(),
        Err(Error::Message(message)) => message,
        Ok(Deletion::Deleted) | Err(_) => return None,
    };
    Some(KeptBranch { name, reason })
}

fn force_helps(refusal: &str) -> bool {
    refusal.contains("modified or untracked files") || refusal.contains("locked")
}

fn worktrees_folder(project: &Project) -> Result<PathBuf, Error> {
    let home = home(project).ok_or_else(|| {
        Error::Message("HOME is not set, so Chord cannot place the worktree.".into())
    })?;
    Ok(home.join(".chord/worktrees").join(&project.name))
}

pub fn worktrees_folders(project: &Project) -> Vec<PathBuf> {
    let legacy = project
        .path
        .with_file_name(format!("{}-worktrees", project.name));
    worktrees_folder(project)
        .into_iter()
        .chain([legacy])
        .collect()
}

#[cfg(not(test))]
fn home(_: &Project) -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
fn home(project: &Project) -> Option<PathBuf> {
    project.path.parent().map(Path::to_path_buf)
}

fn folder(project: &Project, branch: &str) -> Result<PathBuf, Error> {
    Ok(worktrees_folder(project)?.join(branch.replace('/', "-")))
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
                        git: true,
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
    fn in_order_puts_saved_worktrees_first_and_new_ones_last() {
        let worktree = |path: &str| Worktree {
            path: path.into(),
            branch: None,
            xcode: false,
            git: true,
        };
        let git = vec![
            worktree("/a"),
            worktree("/b"),
            worktree("/c"),
            worktree("/d"),
        ];
        let saved: Vec<PathBuf> = vec!["/c".into(), "/gone".into(), "/a".into()];
        let paths: Vec<PathBuf> = in_order(git, &saved).into_iter().map(|w| w.path).collect();
        let expected: Vec<PathBuf> = vec!["/c".into(), "/a".into(), "/b".into(), "/d".into()];
        assert_eq!(paths, expected);
    }

    #[test]
    fn create_adds_a_worktree_on_a_new_branch_and_check_rejects_unusable_names() -> Result<(), Error>
    {
        let root = std::env::temp_dir().join(format!("chord-create-{}", std::process::id()));
        let result = create_and_check(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    #[test]
    fn rename_moves_the_branch_and_check_rejects_taken_names_but_not_a_case_change()
    -> Result<(), Error> {
        let root = std::env::temp_dir().join(format!("chord-rename-{}", std::process::id()));
        let result = rename_and_check(&root);
        let _ = std::fs::remove_dir_all(&root);
        result
    }

    fn rename_and_check(root: &Path) -> Result<(), Error> {
        let project = new_repo(root)?;
        create(&project, "foo")?;
        let path = |branch: &str| -> Result<PathBuf, Error> {
            Ok(list(&project.path)?
                .into_iter()
                .find(|w| w.branch.as_deref() == Some(branch))
                .map(|w| w.path)
                .unwrap_or_default())
        };
        let worktree = path("foo")?;

        let taken = check_branch_rename(&project, "foo", "main")?;
        let own = check_branch_rename(&project, "foo", "foo")?;
        let bad = check_branch_rename(&project, "foo", "a b")?;
        rename_branch(&project, "foo", "Foo")?;
        let after_case = path("Foo")?;
        rename_branch(&project, "Foo", "bar/baz")?;
        let after_rename = path("bar/baz")?;

        assert_eq!(taken.as_deref(), Some("A branch with this name exists"));
        assert_eq!(own, None);
        assert_eq!(bad.as_deref(), Some("Not a valid branch name"));
        assert_eq!(after_case, worktree);
        assert_eq!(after_rename, worktree);
        Ok(())
    }

    fn new_repo(root: &Path) -> Result<Project, Error> {
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
        Ok(Project {
            path: repo,
            name: "app".into(),
        })
    }

    fn create_and_check(root: &Path) -> Result<(), Error> {
        let project = new_repo(root)?;
        let repo = project.path.clone();

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
        let merged_removal = remove(&project, &made.path)?;
        let unmerged_removal = remove(&project, &unmerged.path)?;

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
        assert!(!root.join(".chord/worktrees/app").exists());
        assert!(matches!(remove(&project, &repo), Err(Error::Message(_))));
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
        assert_eq!(made.path, root.join(".chord/worktrees/app/feature-x"));
        assert!(
            listed
                .iter()
                .any(|w| w.branch.as_deref() == Some("feature/x"))
        );
        Ok(())
    }

    fn remove(project: &Project, path: &Path) -> Result<Removal, Error> {
        if let Some(refusal) = refusal(path, false)? {
            return Ok(refusal);
        }
        Ok(match remove_folder(project, path, false)? {
            Ok(branch) => Removal::Removed {
                kept_branch: branch.and_then(|name| delete_merged_branch(project, name)),
            },
            Err(refusal) => refusal,
        })
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
                git: true,
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
