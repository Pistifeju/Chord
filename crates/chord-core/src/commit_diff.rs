use std::path::Path;

use serde::Serialize;
use specta::Type;

use crate::changes::{self, ChangeStatus};
use crate::diff::{self, FileDiff};
use crate::{Error, git};

#[derive(Serialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CommittedFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: ChangeStatus,
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

pub(crate) fn parent(dir: &Path, commit: &str) -> Result<String, Error> {
    match git::optional(git::run(
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

pub fn commit_files(dir: &Path, commit: &str) -> Result<Vec<CommittedFile>, Error> {
    files_between(dir, &parent(dir, commit)?, commit)
}

pub(crate) fn files_between(dir: &Path, from: &str, to: &str) -> Result<Vec<CommittedFile>, Error> {
    let names = git::read(dir, &["diff", "-M", "--name-status", "-z", from, to, "--"])?;
    let counts = diff::parse_numstat(&git::read(
        dir,
        &["diff", "-M", "--numstat", "-z", from, to, "--"],
    )?);
    let mut files = parse_name_status(&names);
    for file in &mut files {
        (file.added, file.removed) = counts.get(&file.path).copied().flatten().unzip();
    }
    Ok(files)
}

pub(crate) fn commit_diff(dir: &Path, commit: &str, path: &str) -> Result<FileDiff, Error> {
    diff_between(dir, &parent(dir, commit)?, commit, path)
}

pub(crate) fn diff_between(
    dir: &Path,
    from: &str,
    to: &str,
    path: &str,
) -> Result<FileDiff, Error> {
    let names = git::read(dir, &["diff", "-M", "--name-status", "-z", from, to, "--"])?;
    let old_path = parse_name_status(&names)
        .into_iter()
        .find(|file| file.path == path)
        .and_then(|file| file.old_path);
    let mut args = vec!["diff", "--no-color", "--no-ext-diff", "-M", from, to, "--"];
    args.extend(old_path.as_deref());
    args.push(path);
    Ok(diff::parse_diff(&git::read(dir, &args)?))
}

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
