//! Which files a preview hot reload recompiles: the shown preview's file, the saved view file and every view file between them, found by the type names each file declares and uses. The new image then builds the saved view's new types all the way from the preview, and old code never meets them.
//! shortcut: names, not the compiler's dependency graph (`.swiftdeps` has no reader in the toolchain). A missed edge leaves an old file on the path, which keeps the old view: stale, not a crash.

use std::collections::{BTreeSet, HashMap, HashSet};

const DECLARES: &[&str] = &["struct", "class", "enum", "protocol", "actor", "typealias"];

/// A file's identifiers outside comments and string literals, and the type names it declares at file level or one level in.
#[derive(Default)]
struct Scan {
    uses: HashSet<String>,
    declares: Vec<String>,
}

fn starts(b: &[u8], i: usize, pat: &[u8]) -> bool {
    b.get(i..).is_some_and(|r| r.starts_with(pat))
}

/// Index of `pat` from `i`.
pub(crate) fn find(b: &[u8], i: usize, pat: &[u8]) -> Option<usize> {
    b.get(i..)?
        .windows(pat.len())
        .position(|w| w == pat)
        .map(|k| i + k)
}

/// Index just past `pat` from `i`, or the end.
fn past(b: &[u8], i: usize, pat: &[u8]) -> usize {
    find(b, i, pat).map_or(b.len(), |k| k + pat.len())
}

/// Index just past the comment or string literal that starts at `i`; None when none starts there.
/// shortcut: string interpolation with nested quotes can end a string early; write a full Swift lexer if that misleads a scan.
pub(crate) fn past_literal(b: &[u8], i: usize) -> Option<usize> {
    if starts(b, i, b"//") {
        return Some(past(b, i, b"\n"));
    }
    if starts(b, i, b"/*") {
        // Swift block comments nest.
        let (mut open, mut i) = (1, i + 2);
        while i < b.len() && open > 0 {
            if starts(b, i, b"/*") {
                (open, i) = (open + 1, i + 2);
            } else if starts(b, i, b"*/") {
                (open, i) = (open - 1, i + 2);
            } else {
                i += 1;
            }
        }
        return Some(i);
    }
    // Raw strings, `#"…"#` and `#"""…"""#`, close with as many `#` as they open with.
    let hashes = b.get(i..)?.iter().take_while(|&&h| h == b'#').count();
    let open = i + hashes;
    if b.get(open) != Some(&b'"') {
        return None;
    }
    let quotes: &[u8] = if starts(b, open, b"\"\"\"") {
        b"\"\"\""
    } else {
        b"\""
    };
    if hashes > 0 || quotes.len() == 3 {
        let close = [quotes, &vec![b'#'; hashes]].concat();
        return Some(past(b, open + quotes.len(), &close));
    }
    let mut i = open + 1;
    while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
        i += if b[i] == b'\\' { 2 } else { 1 };
    }
    Some((i + 1).min(b.len()))
}

fn scan(src: &str) -> Scan {
    let b = src.as_bytes();
    let (mut out, mut i, mut depth, mut declaring) = (Scan::default(), 0, 0i32, false);
    while i < b.len() {
        let c = b[i];
        if let Some(end) = past_literal(b, i) {
            i = end;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = &src[start..i];
            // `class func` and `class var` are members, not types.
            if declaring
                && !matches!(
                    word,
                    "func" | "var" | "let" | "subscript" | "init" | "static" | "final"
                )
            {
                out.declares.push(word.to_string());
            }
            declaring = depth <= 1 && DECLARES.contains(&word);
            out.uses.insert(word.to_string());
        } else {
            match c {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
    }
    out
}

pub(crate) struct Graph {
    files: Vec<String>,
    /// Uses `View` or `ViewModifier`: only view files embed another file's view types in their own body types.
    view: Vec<bool>,
    /// Per file, the view files it depends on, and the reverse.
    into_views: Vec<Vec<usize>>,
    from_views: Vec<Vec<usize>>,
}

impl Graph {
    /// `sources`: (path, contents) of one module's files.
    pub(crate) fn new(sources: &[(String, String)]) -> Graph {
        let scans: Vec<Scan> = sources.iter().map(|(_, s)| scan(s)).collect();
        let view: Vec<bool> = scans
            .iter()
            .map(|s| s.uses.contains("View") || s.uses.contains("ViewModifier"))
            .collect();
        let mut owners: HashMap<&str, BTreeSet<usize>> = HashMap::new();
        for (i, s) in scans.iter().enumerate() {
            for d in &s.declares {
                owners.entry(d).or_default().insert(i);
            }
        }
        let n = sources.len();
        let (mut into_views, mut from_views) = (vec![Vec::new(); n], vec![Vec::new(); n]);
        for (i, s) in scans.iter().enumerate() {
            // A name declared in more than two files (`Content`, `Kind`) says nothing about which one is meant.
            let deps: BTreeSet<usize> = s
                .uses
                .iter()
                .filter_map(|u| owners.get(u.as_str()))
                .filter(|o| o.len() <= 2)
                .flatten()
                .copied()
                .collect();
            for j in deps.into_iter().filter(|&j| j != i && view[j]) {
                into_views[i].push(j);
                from_views[j].push(i);
            }
        }
        Graph {
            files: sources.iter().map(|(p, _)| p.clone()).collect(),
            view,
            into_views,
            from_views,
        }
    }

    fn index(&self, file: &str) -> Option<usize> {
        self.files.iter().position(|f| f == file)
    }

    pub(crate) fn is_view(&self, file: &str) -> bool {
        self.index(file).is_some_and(|i| self.view[i])
    }

    fn reach(start: usize, edges: &[Vec<usize>]) -> HashSet<usize> {
        let (mut seen, mut todo) = (HashSet::from([start]), vec![start]);
        while let Some(x) = todo.pop() {
            todo.extend(edges[x].iter().copied().filter(|&y| seen.insert(y)));
        }
        seen
    }

    /// `from`, `to` and every view file on a path between them; None when `from` does not reach `to`.
    pub(crate) fn path(&self, from: &str, to: &str) -> Option<BTreeSet<String>> {
        let (f, t) = (self.index(from)?, self.index(to)?);
        let forward = Self::reach(f, &self.into_views);
        if !forward.contains(&t) {
            return None;
        }
        let back = Self::reach(t, &self.from_views);
        Some(
            forward
                .intersection(&back)
                .chain([&f, &t])
                .map(|&i| self.files[i].clone())
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(files: &[(&str, &str)]) -> Graph {
        Graph::new(
            &files
                .iter()
                .map(|(p, s)| (p.to_string(), s.to_string()))
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn scans_names_outside_comments_and_strings() {
        let s = scan(
            "struct A: View { // B\n  /* C /* D */ E */ let x = \"F \\\" G\" + #\"H\"# + \"\"\"\n I\n \"\"\"\n  enum Inner {}\n func f() { struct Local {} }\n}\nclass func g() {}\n#Preview { J() }",
        );
        for gone in ["B", "C", "D", "E", "F", "G", "H", "I"] {
            assert!(!s.uses.contains(gone), "{gone}");
        }
        assert!(s.uses.contains("View") && s.uses.contains("J") && s.uses.contains("Preview"));
        assert_eq!(s.declares, ["A", "Inner"]);
    }

    #[test]
    fn path_runs_through_view_files_only() {
        let g = graph(&[
            (
                "P",
                "struct P: View { var body: some View { T(); Q(model: M()) } }",
            ),
            ("T", "struct T: View { var body: some View { S() } }"),
            ("S", "struct S: View { var body: some View { Text(\"\") } }"),
            // A model that names S (a route) is no way from P to S.
            ("M", "final class M { var route: S? }"),
            (
                "Q",
                "struct Q: View { let model: M; var body: some View { EmptyView() } }",
            ),
            ("U", "struct U: View { var body: some View { S() } }"),
        ]);
        assert_eq!(
            g.path("P", "S"),
            Some(["P", "S", "T"].map(String::from).into())
        );
        assert_eq!(g.path("P", "P"), Some(["P".to_string()].into()));
        assert_eq!(g.path("P", "U"), None);
        assert_eq!(g.path("P", "nowhere"), None);
        assert!(g.is_view("S") && !g.is_view("M"));
    }

    #[test]
    fn names_declared_in_many_files_link_nothing() {
        let g = graph(&[
            ("P", "struct P: View { var body: some View { Row() } }"),
            (
                "A",
                "struct A: View { struct Row {} ; var body: some View { EmptyView() } }",
            ),
            (
                "B",
                "struct B: View { struct Row {} ; var body: some View { EmptyView() } }",
            ),
            (
                "C",
                "struct C: View { struct Row {} ; var body: some View { EmptyView() } }",
            ),
        ]);
        assert_eq!(g.path("P", "A"), None);
    }
}
