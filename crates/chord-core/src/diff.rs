use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::Type;

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
    pub old: Option<u32>,
    pub new: Option<u32>,
}

#[derive(Serialize, Deserialize, Type, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hunk {
    pub context: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Serialize, Type, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    pub binary: bool,
    pub hunks: Vec<Hunk>,
}

pub(crate) fn parse_numstat(output: &str) -> HashMap<String, Option<(u32, u32)>> {
    let mut counts = HashMap::new();
    let mut records = output.split('\0');
    while let Some(record) = records.next() {
        let mut fields = record.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
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

pub(crate) fn parse_diff(output: &str) -> FileDiff {
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
}
