//! The error lines of a build log: each one's file, line and message. Pure; the build feeds it every log line.

use std::collections::HashSet;

use serde::Serialize;
use specta::Type;

/// How many errors the Build view lists; the rest only add to a count.
pub(crate) const LISTED: usize = 30;

#[derive(Serialize, Type, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BuildIssue {
    /// The path as the compiler printed it.
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

/// The issue a log line holds, if it is an error line: `File.swift:42:13: error: message`, or `error: message` with no file.
pub(crate) fn parse_line(line: &str) -> Option<BuildIssue> {
    let line = line.trim_end();
    if line.starts_with(char::is_whitespace) || line.starts_with('|') {
        return None;
    }
    let (before, message) = [": fatal error: ", ": error: "]
        .iter()
        .filter_map(|marker| {
            line.find(marker)
                .map(|at| (&line[..at], &line[at + marker.len()..]))
        })
        .min_by_key(|(before, _)| before.len())
        .or_else(|| line.strip_prefix("error: ").map(|message| ("", message)))?;
    let (file, line) = position(before);
    // Without a position the text before the marker is at most a tool's name; "note" lines and the like are not errors.
    if file.is_none() && before.contains([':', ' ']) {
        return None;
    }
    Some(BuildIssue {
        file,
        line,
        message: message.to_string(),
    })
}

// `path:42:13` or `path:42`; anything else names no file.
fn position(before: &str) -> (Option<String>, Option<u32>) {
    let mut rest = before;
    let mut numbers = Vec::new();
    while numbers.len() < 2 {
        let Some((head, tail)) = rest.rsplit_once(':') else {
            break;
        };
        let Ok(number) = tail.parse::<u32>() else {
            break;
        };
        numbers.push(number);
        rest = head;
    }
    match numbers.last() {
        Some(&line) if !rest.is_empty() => (Some(rest.to_string()), Some(line)),
        _ => (None, None),
    }
}

/// The distinct errors of one build, in the order they printed.
#[derive(Default)]
pub(crate) struct Issues {
    listed: Vec<BuildIssue>,
    seen: HashSet<BuildIssue>,
}

impl Issues {
    pub fn push_line(&mut self, line: &str) {
        if let Some(issue) = parse_line(line) {
            self.push(issue);
        }
    }

    pub fn push(&mut self, issue: BuildIssue) {
        if self.seen.insert(issue.clone()) && self.listed.len() < LISTED {
            self.listed.push(issue);
        }
    }

    pub fn total(&self) -> usize {
        self.seen.len()
    }

    pub fn into_listed(self) -> (Vec<BuildIssue>, u32) {
        let more = self.seen.len() - self.listed.len();
        (self.listed, more as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(file: Option<&str>, line: Option<u32>, message: &str) -> BuildIssue {
        BuildIssue {
            file: file.map(String::from),
            line,
            message: message.to_string(),
        }
    }

    #[test]
    fn reads_a_compiler_error_with_its_file_and_line() {
        assert_eq!(
            parse_line("/w/App/LoginView.swift:42:13: error: cannot find 'x' in scope"),
            Some(issue(
                Some("/w/App/LoginView.swift"),
                Some(42),
                "cannot find 'x' in scope"
            ))
        );
        assert_eq!(
            parse_line("/w/a.m:7: error: oops\r"),
            Some(issue(Some("/w/a.m"), Some(7), "oops"))
        );
        assert_eq!(
            parse_line("/w/a.m:3:1: fatal error: 'x.h' file not found"),
            Some(issue(Some("/w/a.m"), Some(3), "'x.h' file not found"))
        );
    }

    #[test]
    fn reads_an_error_without_a_file() {
        assert_eq!(
            parse_line("error: Signing for \"App\" requires a development team."),
            Some(issue(
                None,
                None,
                "Signing for \"App\" requires a development team."
            ))
        );
        assert_eq!(
            parse_line("xcodebuild: error: Unable to find a destination"),
            Some(issue(None, None, "Unable to find a destination"))
        );
        assert_eq!(
            parse_line("clang: error: linker command failed with exit code 1"),
            Some(issue(None, None, "linker command failed with exit code 1"))
        );
    }

    #[test]
    fn skips_warnings_notes_and_quoted_source() {
        assert_eq!(parse_line("/w/a.swift:1:1: warning: unused"), None);
        assert_eq!(parse_line("/w/a.swift:1:1: note: error: shown"), None);
        assert_eq!(parse_line("    let s = \"x: error: y\""), None);
        assert_eq!(parse_line("** BUILD FAILED **"), None);
    }

    #[test]
    fn lists_thirty_distinct_errors_and_counts_the_rest() {
        let mut issues = Issues::default();
        for n in 0..45 {
            issues.push_line(&format!("/w/a.swift:{n}:1: error: bad"));
            issues.push_line(&format!("/w/a.swift:{n}:1: error: bad"));
        }
        assert_eq!(issues.total(), 45);
        let (listed, more) = issues.into_listed();
        assert_eq!(listed.len(), 30);
        assert_eq!(listed[29].line, Some(29));
        assert_eq!(more, 15);
    }
}
