use std::path::Path;

use chord_core::{Error, PreviewPick, files};

use crate::view_graph::{find, past_literal};

pub fn list(worktree: &Path) -> Result<Vec<PreviewPick>, Error> {
    let mut paths: Vec<String> = files::all(worktree)?
        .into_iter()
        .filter(|path| path.ends_with(".swift"))
        .collect();
    paths.sort();
    Ok(paths
        .into_iter()
        .flat_map(|file| {
            let source = std::fs::read_to_string(worktree.join(&file)).unwrap_or_default();
            names(&source)
                .into_iter()
                .zip(0..)
                .map(move |(name, index)| PreviewPick {
                    file: file.clone(),
                    index,
                    name,
                })
        })
        .collect())
}

pub(crate) fn holds(worktree: &Path, pick: &PreviewPick) -> bool {
    std::fs::read_to_string(worktree.join(&pick.file))
        .is_ok_and(|source| names(&source).len() > pick.index as usize)
}

fn names(source: &str) -> Vec<Option<String>> {
    let b = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if let Some(end) = past_literal(b, i) {
            i = end;
        } else if b[i..].starts_with(b"#Preview")
            && !b
                .get(i + 8)
                .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            out.push(name_at(b, i + 8));
            i += 8;
        } else {
            i += 1;
        }
    }
    out
}

fn name_at(b: &[u8], i: usize) -> Option<String> {
    let rest = &b[i..];
    let open = rest.iter().position(|c| !c.is_ascii_whitespace())?;
    if rest[open] != b'(' {
        return None;
    }
    let quote = open
        + 1
        + rest[open + 1..]
            .iter()
            .position(|c| !c.is_ascii_whitespace())?;
    if rest[quote] != b'"' {
        return None;
    }
    let close = find(rest, quote + 1, b"\"")?;
    Some(String::from_utf8_lossy(&rest[quote + 1..close]).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_skip_comments_strings_and_other_macros() {
        let source = "\n// #Preview(\"Commented\")\n#Preview { Text(\"a\") }\nlet s = \"#Preview(\\\"In a string\\\")\"\n#Preview( \"Dark\", traits: .landscapeLeft) { Text(\"b\") }\n#PreviewMacro(\"Other\")\n/* #Preview */\n#Preview(traits: .sizeThatFitsLayout) { Text(\"c\") }\n";
        assert_eq!(names(source), [None, Some("Dark".into()), None]);
    }
}
