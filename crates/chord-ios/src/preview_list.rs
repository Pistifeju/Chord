//! The `#Preview`s in a worktree's Swift files, read from the source so the picker lists them before any build: each one's file, place in its file and name.

use std::path::Path;

use chord_core::{Error, PreviewPick, files};

use crate::view_graph::{find, past_literal};

/// Every `#Preview` in the worktree's `.swift` files that git does not ignore, by file in path order.
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

/// Whether the worktree still holds the picked `#Preview`.
pub(crate) fn holds(worktree: &Path, pick: &PreviewPick) -> bool {
    std::fs::read_to_string(worktree.join(&pick.file))
        .is_ok_and(|source| names(&source).len() > pick.index as usize)
}

/// The name argument of each `#Preview` macro outside comments and strings, in source order.
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

/// `("Dark"` after the macro name: the string, without escapes or interpolation.
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
        let source = r##"
// #Preview("Commented")
#Preview { Text("a") }
let s = "#Preview(\"In a string\")"
#Preview( "Dark", traits: .landscapeLeft) { Text("b") }
#PreviewMacro("Other")
/* #Preview */
#Preview(traits: .sizeThatFitsLayout) { Text("c") }
"##;
        assert_eq!(names(source), [None, Some("Dark".into()), None]);
    }
}
