//! A worktree's changed files from `git status`, staged or not, with the lines each adds and removes, one file's diff: the index against HEAD, or the file on disk against the index, and the stage, unstage and discard of files and of a diff's hunks.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{DiffSide, Error, files, git};

#[derive(Serialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ChangeStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
}

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub path: String,
    /// The path a rename came from.
    pub old_path: Option<String>,
    /// Under Staged: the index against HEAD; else the file on disk against the index.
    pub staged: bool,
    pub status: ChangeStatus,
    /// None for a binary file.
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

#[derive(Serialize, Deserialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    pub kind: LineKind,
    pub text: String,
    /// The line's number in the old side, the new side, or both for context.
    pub old: Option<u32>,
    pub new: Option<u32>,
}

#[derive(Serialize, Deserialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    /// The line git names after the hunk's range, mostly the function it sits in; empty when git finds none.
    pub context: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Serialize, Type, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub binary: bool,
    pub hunks: Vec<Hunk>,
}

// Without optional locks git leaves the index alone, so an agent's own git command never meets our lock.
fn read(dir: &Path, args: &[&str]) -> Result<String, Error> {
    git::run(
        dir,
        &[&["--no-optional-locks", "--literal-pathspecs"], args].concat(),
    )
}

pub fn list(dir: &Path) -> Result<Vec<Change>, Error> {
    let (status, staged, unstaged) = std::thread::scope(|scope| {
        let staged = scope.spawn(|| read(dir, &["diff", "--cached", "-M", "--numstat", "-z"]));
        // -2 compares a conflicted file with our side; it changes nothing for the others.
        let unstaged = scope.spawn(|| read(dir, &["diff", "-2", "--numstat", "-z"]));
        let status = read(
            dir,
            &["status", "--porcelain=v2", "-z", "--untracked-files=all"],
        );
        let staged = staged
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        let unstaged = unstaged
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        (status, staged, unstaged)
    });
    let staged = parse_numstat(&staged?);
    let unstaged = parse_numstat(&unstaged?);
    let mut changes = parse_status(&status?);
    for change in &mut changes {
        let counts = match (change.staged, change.status) {
            (true, _) => staged.get(&change.path).copied(),
            (false, ChangeStatus::Untracked) => untracked_lines(dir, &change.path),
            (false, _) => unstaged.get(&change.path).copied(),
        };
        (change.added, change.removed) = counts.flatten().unzip();
    }
    Ok(changes)
}

/// git's diff of the file, as `git apply` takes it.
fn raw_diff(dir: &Path, path: &str, side: DiffSide) -> Result<String, Error> {
    let mut args = vec![
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--src-prefix=a/",
        "--dst-prefix=b/",
    ];
    if side == DiffSide::Unstaged {
        args.extend(["-2", "--", path]);
        return read(dir, &args);
    }
    // A rename shows against the file it came from, so git needs both paths.
    let status = read(
        dir,
        &["status", "--porcelain=v2", "-z", "--untracked-files=no"],
    )?;
    let from = parse_status(&status)
        .into_iter()
        .find(|change| change.staged && change.path == path)
        .and_then(|change| change.old_path);
    args.extend(["--cached", "-M", "--"]);
    args.extend(from.as_deref());
    args.push(path);
    read(dir, &args)
}

pub fn diff(dir: &Path, path: &str, side: DiffSide) -> Result<FileDiff, Error> {
    let output = raw_diff(dir, path, side)?;
    if side == DiffSide::Staged {
        return Ok(parse_diff(&output));
    }
    // An untracked file has no diff output, so only an empty one needs the tracked check.
    if !output.is_empty() || !read(dir, &["ls-files", "--", path])?.is_empty() {
        return Ok(parse_diff(&output));
    }
    let file = files::inside(dir, path)?;
    // An untracked file deleted since the list was read, or a nested repo's folder, has nothing to show.
    if !file.is_file() {
        return Ok(FileDiff::default());
    }
    let bytes = std::fs::read(file)?;
    if is_binary(&bytes) {
        return Ok(FileDiff {
            binary: true,
            hunks: Vec::new(),
        });
    }
    let lines: Vec<DiffLine> = String::from_utf8_lossy(&bytes)
        .lines()
        .zip(1..)
        .map(|(text, new)| DiffLine {
            kind: LineKind::Added,
            text: text.to_string(),
            old: None,
            new: Some(new),
        })
        .collect();
    Ok(FileDiff {
        binary: false,
        hunks: if lines.is_empty() {
            Vec::new()
        } else {
            vec![Hunk {
                context: String::new(),
                lines,
            }]
        },
    })
}

#[derive(Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum HunkAction {
    /// A hunk of the file on disk against the index, into the index.
    Stage,
    /// A hunk of the index against HEAD, out of the index.
    Unstage,
    /// A hunk of the file on disk against the index, off the disk.
    Discard,
    /// A hunk of the index against HEAD, out of the index and off the disk.
    DiscardStaged,
}

// Paths go on stdin, so a long list never meets the argument limit.
fn with_paths(dir: &Path, args: &[&str], paths: &[String]) -> Result<(), Error> {
    let args = [
        &["--literal-pathspecs"],
        args,
        &["--pathspec-from-file=-", "--pathspec-file-nul"],
    ]
    .concat();
    git::run_with_input(dir, &args, paths.join("\0").as_bytes()).map(drop)
}

/// Adds the files to the index, a deleted file's removal too; a conflicted file counts as resolved.
pub fn stage(dir: &Path, paths: &[String]) -> Result<(), Error> {
    with_paths(dir, &["add", "-A"], paths)
}

/// Puts the files' HEAD versions back in the index; a rename needs its old path too.
pub fn unstage(dir: &Path, paths: &[String]) -> Result<(), Error> {
    with_paths(dir, &["reset", "-q"], paths)
}

/// Restores the files on disk from the index, and moves the untracked ones to the Trash.
pub fn discard(dir: &Path, paths: &[String]) -> Result<(), Error> {
    let status = read(
        dir,
        &["status", "--porcelain=v2", "-z", "--untracked-files=all"],
    )?;
    let untracked: Vec<String> = parse_status(&status)
        .into_iter()
        .filter(|change| change.status == ChangeStatus::Untracked)
        .map(|change| change.path)
        .collect();
    let (untracked, tracked): (Vec<String>, Vec<String>) = paths
        .iter()
        .cloned()
        .partition(|path| untracked.contains(path));
    if !tracked.is_empty() {
        with_paths(dir, &["checkout"], &tracked)?;
    }
    untracked
        .iter()
        .try_for_each(|path| files::trash(dir, path))
}

/// Stages, unstages or discards the one hunk of the file's diff that reads as `hunk` now; a diff changed since it showed fails.
pub fn apply_hunk(dir: &Path, path: &str, hunk: &Hunk, action: HunkAction) -> Result<(), Error> {
    let side = match action {
        HunkAction::Unstage | HunkAction::DiscardStaged => DiffSide::Staged,
        HunkAction::Stage | HunkAction::Discard => DiffSide::Unstaged,
    };
    let changed = || {
        Error::Message(
            "The diff changed since it showed. Try again on the diff as it reads now.".into(),
        )
    };
    let output = raw_diff(dir, path, side)?;
    let Some(start) = output.find("\n@@ ").map(|at| at + 1) else {
        // An untracked file's one hunk is the whole file.
        if side == DiffSide::Unstaged
            && diff(dir, path, side)?.hunks.as_slice() == std::slice::from_ref(hunk)
        {
            let paths = [path.to_string()];
            return if action == HunkAction::Stage {
                stage(dir, &paths)
            } else {
                discard(dir, &paths)
            };
        }
        return Err(changed());
    };
    // A hunk of a rename applies to the new path alone, so the rename itself stays.
    let header = if output[..start].contains("\nrename from ") {
        format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n")
    } else {
        output[..start].to_string()
    };
    let mut hunks: Vec<String> = Vec::new();
    for line in output[start..].split_inclusive('\n') {
        match hunks.last_mut() {
            Some(text) if !line.starts_with("@@ ") => text.push_str(line),
            _ => hunks.push(line.to_string()),
        }
    }
    let patch = hunks
        .into_iter()
        .map(|text| header.clone() + &text)
        .find(|patch| parse_diff(patch).hunks.first() == Some(hunk))
        .ok_or_else(changed)?;
    let apply = |args: &[&str]| git::run_with_input(dir, args, patch.as_bytes()).map(drop);
    match action {
        HunkAction::Stage => apply(&["apply", "--cached", "-"]),
        HunkAction::Unstage => apply(&["apply", "--cached", "-R", "-"]),
        HunkAction::Discard => apply(&["apply", "-R", "-"]),
        HunkAction::DiscardStaged => {
            apply(&["apply", "--cached", "-R", "-"])?;
            // Edits on disk over the hunk's lines stop it there; the hunk goes back in the index.
            apply(&["apply", "-R", "-"]).inspect_err(|_| {
                let _ = apply(&["apply", "--cached", "-"]);
            })
        }
    }
}

// git's own test: a NUL in the first 8000 bytes.
fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8000).any(|&byte| byte == 0)
}

/// An untracked file adds all its lines; None for a binary or unreadable file.
fn untracked_lines(dir: &Path, path: &str) -> Option<Option<(u32, u32)>> {
    let mut reader = BufReader::new(File::open(files::inside(dir, path).ok()?).ok()?);
    if is_binary(reader.fill_buf().ok()?) {
        return Some(None);
    }
    // The count `str::lines` gives, read in chunks so a large file never sits in memory.
    let (mut lines, mut last) = (0usize, b'\n');
    loop {
        let chunk = reader.fill_buf().ok()?;
        let Some(&end) = chunk.last() else { break };
        lines += chunk.iter().filter(|&&byte| byte == b'\n').count();
        last = end;
        let read = chunk.len();
        reader.consume(read);
    }
    lines += usize::from(last != b'\n');
    Some(Some((u32::try_from(lines).unwrap_or(u32::MAX), 0)))
}

fn status_of(code: u8) -> Option<ChangeStatus> {
    match code {
        b'M' | b'T' => Some(ChangeStatus::Modified),
        b'A' | b'C' => Some(ChangeStatus::Added),
        b'D' => Some(ChangeStatus::Deleted),
        b'R' => Some(ChangeStatus::Renamed),
        _ => None,
    }
}

/// Each record of `git status --porcelain=v2 -z`: a file staged and changed again on disk gives one change in each group.
fn parse_status(output: &str) -> Vec<Change> {
    let change = |path: &str, old_path: Option<&str>, staged, status| Change {
        path: path.to_string(),
        old_path: old_path.map(str::to_string),
        staged,
        status,
        added: None,
        removed: None,
    };
    let mut changes = Vec::new();
    let mut records = output.split('\0');
    while let Some(record) = records.next() {
        let fields = |count| record.splitn(count, ' ').collect::<Vec<_>>();
        let (codes, path, old_path) = match record.as_bytes().first() {
            Some(b'?') => {
                let path = record.get(2..).unwrap_or_default();
                changes.push(change(path, None, false, ChangeStatus::Untracked));
                continue;
            }
            Some(b'u') => {
                if let Some(path) = fields(11).get(10) {
                    changes.push(change(path, None, false, ChangeStatus::Conflicted));
                }
                continue;
            }
            Some(b'1') => {
                let fields = fields(9);
                (fields.get(1).copied(), fields.get(8).copied(), None)
            }
            // A rename's record is followed by the path it came from.
            Some(b'2') => {
                let fields = fields(10);
                (
                    fields.get(1).copied(),
                    fields.get(9).copied(),
                    records.next(),
                )
            }
            _ => continue,
        };
        let (Some(codes), Some(path)) = (codes.map(str::as_bytes), path) else {
            continue;
        };
        if let Some(status) = codes.first().copied().and_then(status_of) {
            changes.push(change(path, old_path, true, status));
        }
        // On disk git sees no rename: the file at the new path changed.
        if let Some(status) = codes.get(1).copied().and_then(status_of) {
            changes.push(change(path, None, false, status));
        }
    }
    changes
}

/// The lines added and removed per path of `git diff --numstat -z`, None for a binary file; a rename counts under its new path.
fn parse_numstat(output: &str) -> HashMap<String, Option<(u32, u32)>> {
    let mut counts = HashMap::new();
    let mut records = output.split('\0');
    while let Some(record) = records.next() {
        let mut fields = record.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        // A rename leaves the path empty, then gives the old and the new path.
        let path = if path.is_empty() {
            records.next();
            records.next().unwrap_or_default()
        } else {
            path
        };
        let lines = added.parse().ok().zip(removed.parse().ok());
        counts.insert(path.to_string(), lines);
    }
    counts
}

fn parse_diff(output: &str) -> FileDiff {
    let mut diff = FileDiff::default();
    let (mut old, mut new) = (0, 0);
    for line in output.lines() {
        if let Some(header) = line.strip_prefix("@@ -") {
            let (ranges, context) = header.split_once(" @@").unwrap_or((header, ""));
            let mut sides = ranges.split(' ');
            let start = |side: Option<&str>| {
                side.and_then(|side| side.split(',').next()?.trim_start_matches('+').parse().ok())
                    .unwrap_or(0)
            };
            old = start(sides.next());
            new = start(sides.next());
            diff.hunks.push(Hunk {
                context: context.trim().to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        let Some(hunk) = diff.hunks.last_mut() else {
            diff.binary |= line.starts_with("Binary files ") || line == "GIT binary patch";
            continue;
        };
        let (kind, old_at, new_at) = match line.as_bytes().first() {
            Some(b'+') => (LineKind::Added, None, Some(new)),
            Some(b'-') => (LineKind::Removed, Some(old), None),
            Some(b' ') | None => (LineKind::Context, Some(old), Some(new)),
            // "\ No newline at end of file"
            _ => continue,
        };
        old += u32::from(old_at.is_some());
        new += u32::from(new_at.is_some());
        hunk.lines.push(DiffLine {
            kind,
            text: line.get(1..).unwrap_or_default().to_string(),
            old: old_at,
            new: new_at,
        });
    }
    diff
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_splits_into_staged_and_unstaged_changes() {
        let output = "1 MM N... 100644 100644 100644 1111 2222 src/a b.rs\x002 R. N... 100644 100644 100644 1111 1111 R100 new.rs\x00old.rs\x00u UU N... 100644 100644 100644 100644 1111 2222 3333 c.rs\x00? d.rs\x00";
        let changes: Vec<_> = parse_status(output)
            .into_iter()
            .map(|c| (c.path, c.old_path, c.staged, c.status))
            .collect();
        assert_eq!(
            changes,
            vec![
                ("src/a b.rs".into(), None, true, ChangeStatus::Modified),
                ("src/a b.rs".into(), None, false, ChangeStatus::Modified),
                (
                    "new.rs".into(),
                    Some("old.rs".into()),
                    true,
                    ChangeStatus::Renamed
                ),
                ("c.rs".into(), None, false, ChangeStatus::Conflicted),
                ("d.rs".into(), None, false, ChangeStatus::Untracked),
            ]
        );
    }

    #[test]
    fn a_numstat_counts_renames_under_the_new_path_and_binaries_as_none() {
        let counts = parse_numstat("3\t1\ta.rs\x001\t1\t\x00old.rs\x00new.rs\x00-\t-\timg.png\x00");
        assert_eq!(counts.get("a.rs"), Some(&Some((3, 1))));
        assert_eq!(counts.get("new.rs"), Some(&Some((1, 1))));
        assert_eq!(counts.get("img.png"), Some(&None));
        assert_eq!(counts.len(), 3);
    }

    #[test]
    fn a_diff_numbers_each_side() {
        let output = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -4,3 +4,3 @@ fn main\n keep\n-old\n+new\n\\ No newline at end of file\n";
        let hunks = parse_diff(output).hunks;
        assert_eq!(hunks[0].context, "fn main");
        let lines: Vec<_> = hunks
            .into_iter()
            .flat_map(|hunk| hunk.lines)
            .map(|l| (l.kind, l.text, l.old, l.new))
            .collect();
        assert_eq!(
            lines,
            vec![
                (LineKind::Context, "keep".into(), Some(4), Some(4)),
                (LineKind::Removed, "old".into(), Some(5), None),
                (LineKind::Added, "new".into(), None, Some(5)),
            ]
        );
        assert!(parse_diff("diff --git a/i b/i\nBinary files a/i and b/i differ\n").binary);
    }

    #[test]
    fn a_hunk_stages_and_discards_alone() -> Result<(), Error> {
        let repo = std::env::temp_dir().join(format!("chord-hunks-{}", std::process::id()));
        let result = hunks_in(&repo);
        let _ = std::fs::remove_dir_all(&repo);
        result
    }

    fn hunks_in(repo: &Path) -> Result<(), Error> {
        std::fs::create_dir_all(repo)?;
        let file = repo.join("f");
        let numbers = (1..=40).map(|n| format!("{n}\n")).collect::<String>();
        std::fs::write(&file, &numbers)?;
        git::run(repo, &["init", "-q"])?;
        git::run(repo, &["add", "f"])?;
        git::run(
            repo,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-qm",
                "i",
            ],
        )?;
        std::fs::write(
            &file,
            numbers
                .replace("\n3\n", "\nthree\n")
                .replace("\n30\n", "\nthirty\n"),
        )?;
        let hunks = diff(repo, "f", DiffSide::Unstaged)?.hunks;
        apply_hunk(repo, "f", &hunks[1], HunkAction::Stage)?;
        assert_eq!(read(repo, &["diff", "--cached", "--numstat"])?, "1\t1\tf\n");
        apply_hunk(repo, "f", &hunks[0], HunkAction::Discard)?;
        assert_eq!(
            std::fs::read_to_string(&file)?,
            numbers.replace("\n30\n", "\nthirty\n")
        );
        assert!(apply_hunk(repo, "f", &hunks[0], HunkAction::Discard).is_err());
        let staged = diff(repo, "f", DiffSide::Staged)?.hunks;
        apply_hunk(repo, "f", &staged[0], HunkAction::DiscardStaged)?;
        assert_eq!(std::fs::read_to_string(&file)?, numbers);
        assert_eq!(read(repo, &["diff", "--cached", "--numstat"])?, "");
        Ok(())
    }
}
