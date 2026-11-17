//! A worktree's files on disk: one folder's entries for the Files view, without `.git`, every file git does not ignore for Open File by Name, a text file's read and write for the editor, a file's change time for the image view, and the tree's create, rename, trash and reveal.

use std::cmp::Ordering;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use specta::Type;

use crate::Error;

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// What the row shows: a chain of single-child folders reads as one name, `src/app/views`.
    pub name: String,
    /// Relative to the worktree, `/`-separated; for a chain, the deepest folder's.
    pub path: String,
    pub is_dir: bool,
}

/// The entries of `folder` (relative to `root`, empty for the root): folders first, in natural order.
pub fn list(root: &Path, folder: &str) -> Result<Vec<FileEntry>, Error> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(inside(root, folder)?)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let file_type = entry.file_type()?;
        let is_dir = file_type.is_dir() || (file_type.is_symlink() && entry.path().is_dir());
        let mut path = join(folder, &name);
        let mut shown = name;
        if file_type.is_dir() {
            while let Some(only) = only_folder(&root.join(&path)) {
                path = join(&path, &only);
                shown = join(&shown, &only);
            }
        }
        entries.push(FileEntry {
            name: shown,
            path,
            is_dir,
        });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| natural_cmp(&a.name, &b.name))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(entries)
}

/// Every file under `root` that git tracks or does not ignore, relative and `/`-separated, in git's order; a tracked file gone from disk is left out.
pub fn all(root: &Path) -> Result<Vec<String>, Error> {
    let listed = crate::git::run(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--deduplicate",
        ],
    )?;
    Ok(listed
        .split('\0')
        .filter(|path| !path.is_empty() && root.join(path).is_file())
        .map(str::to_owned)
        .collect())
}

/// The text of `file` (relative to `root`); a file that is not UTF-8 text is an error.
pub fn read(root: &Path, file: &str) -> Result<String, Error> {
    fs::read_to_string(inside(root, file)?).map_err(|error| match error.kind() {
        std::io::ErrorKind::InvalidData => Error::Message(format!("{file} is not a text file")),
        _ => error.into(),
    })
}

/// A key of when `file` (relative to `root`) last changed and its length, so the image view reloads only on a change.
pub fn modified(root: &Path, file: &str) -> Result<String, Error> {
    let metadata = fs::metadata(inside(root, file)?)?;
    let nanos = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // The length tells a truncate from the write that follows it within the clock's resolution.
    Ok(format!("{nanos}-{}", metadata.len()))
}

/// Replaces the text of `file` (relative to `root`) when the disk still holds `expected`, the text the editor last read or wrote; an agent's change stops the save. The new text lands whole: an agent reading meanwhile sees the old text or the new, never half of it.
pub fn write(root: &Path, file: &str, expected: &str, text: &str) -> Result<(), Error> {
    let path = inside(root, file)?;
    // shortcut: an agent write between this read and the write below is still overwritten; agents ignore advisory locks, so upgrade only if such a loss is reported.
    if fs::read(&path)? != expected.as_bytes() {
        return Err(Error::Message(format!(
            "{file} changed on disk, so the save stopped. Reopen the file to see the change."
        )));
    }
    // The real path, so a symlinked file stays a symlink.
    let real = path.canonicalize()?;
    let name = real
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let temp = real.with_file_name(format!(".{name}.chord-save"));
    let saved = fs::write(&temp, text)
        .and_then(|()| fs::set_permissions(&temp, fs::metadata(&real)?.permissions()))
        .and_then(|()| fs::rename(&temp, &real));
    if saved.is_err() {
        let _ = fs::remove_file(&temp);
    }
    Ok(saved?)
}

/// Makes an empty file, or a folder, at `path` (relative to `root`); an existing entry is never replaced.
pub fn create(root: &Path, path: &str, folder: bool) -> Result<(), Error> {
    let target = inside(root, path)?;
    let made = if folder {
        fs::create_dir(&target)
    } else {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map(drop)
    };
    made.map_err(|error| match error.kind() {
        std::io::ErrorKind::AlreadyExists => Error::Message(format!("{path} already exists")),
        _ => error.into(),
    })
}

/// Moves `from` to `to` (both relative to `root`); an existing `to` stops it, unless it is `from` itself, as in a case-only rename on a case-insensitive volume.
pub fn rename(root: &Path, from: &str, to: &str) -> Result<(), Error> {
    refuse_root(from)?;
    let (source, target) = (inside(root, from)?, inside(root, to)?);
    // shortcut: a file created at `to` between this check and the rename is replaced; upgrade to renamex_np with RENAME_EXCL if that is reported.
    if let Ok(existing) = target.symlink_metadata() {
        use std::os::unix::fs::MetadataExt;
        let moved = source.symlink_metadata()?;
        if (existing.dev(), existing.ino()) != (moved.dev(), moved.ino()) {
            return Err(Error::Message(format!("{to} already exists")));
        }
    }
    Ok(fs::rename(source, target)?)
}

/// Moves `path` (relative to `root`) to the user's Trash, where Finder can put it back.
pub fn trash(root: &Path, path: &str) -> Result<(), Error> {
    use objc2_foundation::{NSFileManager, NSString, NSURL};
    refuse_root(path)?;
    let target = inside(root, path)?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(&target.to_string_lossy()));
    NSFileManager::defaultManager()
        .trashItemAtURL_resultingItemURL_error(&url, None)
        .map_err(|error| Error::Message(error.localizedDescription().to_string()))
}

/// Shows `path` (relative to `root`) selected in a Finder window.
pub fn reveal(root: &Path, path: &str) -> Result<(), Error> {
    let status = std::process::Command::new("open")
        .arg("-R")
        .arg(inside(root, path)?)
        .status()?;
    if !status.success() {
        return Err(Error::Message(format!("Cannot show {path} in Finder")));
    }
    Ok(())
}

// An empty path names the worktree itself, which `inside` would accept.
fn refuse_root(path: &str) -> Result<(), Error> {
    if path.is_empty() {
        return Err(Error::Message(
            "The worktree itself cannot be trashed or renamed".into(),
        ));
    }
    Ok(())
}

// The path under `root`; a relative path that climbs out or is absolute is refused.
fn inside(root: &Path, relative: &str) -> Result<PathBuf, Error> {
    let relative = Path::new(relative);
    if relative
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(Error::Message(format!(
            "{} is not inside the worktree",
            relative.display()
        )));
    }
    Ok(root.join(relative))
}

fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_owned()
    } else {
        format!("{folder}/{name}")
    }
}

// The name of the folder inside `folder` when it is all that `folder` holds.
fn only_folder(folder: &Path) -> Option<String> {
    let mut visible = fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != ".git");
    let only = visible.next()?;
    if visible.next().is_some() || !only.file_type().ok()?.is_dir() {
        return None;
    }
    Some(only.file_name().to_string_lossy().into_owned())
}

// Digit runs compare as numbers and letters ignore case, so `file2` comes before `file10`.
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let (x, y) = (digit_run(&mut a), digit_run(&mut b));
                let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != Ordering::Equal {
                    return order;
                }
                a.next();
                b.next();
            }
        }
    }
}

fn digit_run(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut run = String::new();
    while let Some(digit) = chars.next_if(char::is_ascii_digit) {
        run.push(digit);
    }
    run
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_reads_numbers_whole() {
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("File2", "file1"), Ordering::Greater);
        assert_eq!(natural_cmp("a", "B"), Ordering::Less);
    }

    #[test]
    fn lists_folders_first_without_git_and_joins_single_child_chains() {
        let root = std::env::temp_dir().join(format!("chord-files-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("src/app/views")).unwrap();
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs/a.md"), "").unwrap();
        fs::write(root.join("docs/b.md"), "").unwrap();
        fs::write(root.join("src/app/views/x.tsx"), "").unwrap();
        fs::write(root.join("file10"), "").unwrap();
        fs::write(root.join("file2"), "").unwrap();

        let names: Vec<_> = list(&root, "")
            .unwrap()
            .into_iter()
            .map(|entry| (entry.name, entry.path, entry.is_dir))
            .collect();
        let expected = [
            ("docs", "docs", true),
            ("src/app/views", "src/app/views", true),
            ("file2", "file2", false),
            ("file10", "file10", false),
        ];
        assert_eq!(
            names,
            expected.map(|(name, path, is_dir)| (name.to_owned(), path.to_owned(), is_dir))
        );
        assert!(list(&root, "../x").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn writes_and_reads_text_and_refuses_paths_outside_the_worktree() {
        let root = std::env::temp_dir().join(format!("chord-edit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), "").unwrap();
        write(&root, "a.txt", "", "héllo\n").unwrap();
        assert_eq!(read(&root, "a.txt").unwrap(), "héllo\n");
        fs::write(root.join("b.bin"), [0xff, 0xfe]).unwrap();
        assert_eq!(
            read(&root, "b.bin").unwrap_err().to_string(),
            "b.bin is not a text file"
        );
        assert!(read(&root, "../a.txt").is_err());
        assert!(write(&root, "/tmp/x", "", "").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn create_and_rename_never_replace_an_entry() {
        let root = std::env::temp_dir().join(format!("chord-new-entry-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        create(&root, "a.txt", false).unwrap();
        create(&root, "docs", true).unwrap();
        fs::write(root.join("a.txt"), "kept").unwrap();
        assert_eq!(
            create(&root, "a.txt", false).unwrap_err().to_string(),
            "a.txt already exists"
        );
        assert!(create(&root, "docs", true).is_err());
        assert!(rename(&root, "docs", "a.txt").is_err());
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "kept");
        rename(&root, "a.txt", "A.txt").unwrap();
        rename(&root, "A.txt", "docs/b.txt").unwrap();
        assert_eq!(fs::read_to_string(root.join("docs/b.txt")).unwrap(), "kept");
        assert!(rename(&root, "docs/b.txt", "../b.txt").is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn all_lists_tracked_and_untracked_files_but_not_ignored_or_gone_ones() {
        let root = std::env::temp_dir().join(format!("chord-all-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        crate::git::run(&root, &["init", "-q"]).unwrap();
        fs::write(root.join(".gitignore"), "build/\n").unwrap();
        fs::write(root.join("src/a.ts"), "").unwrap();
        fs::write(root.join("gone.ts"), "").unwrap();
        crate::git::run(&root, &["add", "."]).unwrap();
        fs::remove_file(root.join("gone.ts")).unwrap();
        fs::write(root.join("new.ts"), "").unwrap();
        fs::create_dir_all(root.join("build")).unwrap();
        fs::write(root.join("build/out.js"), "").unwrap();

        let mut files = all(&root).unwrap();
        files.sort();
        assert_eq!(files, [".gitignore", "new.ts", "src/a.ts"]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_change_on_disk_stops_the_save() {
        let root = std::env::temp_dir().join(format!("chord-conflict-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), "agent\n").unwrap();
        assert!(write(&root, "a.txt", "mine\n", "edited\n").is_err());
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "agent\n");
        let _ = fs::remove_dir_all(&root);
    }
}
