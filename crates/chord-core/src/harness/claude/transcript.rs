//! Claude Code's session files: the folder of a worktree, one session's turns as history, and the worktree's other sessions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

use serde_json::Value;

use super::HARNESS;
use super::blocks::{self, Streamed};
use crate::harness::resume::Session;
use crate::harness::{HarnessEvent, count, login_shell};

/// The longest folder name Claude Code writes whole; a longer one it cuts and ends with a hash.
const NAME_LIMIT: usize = 200;

/// The folder of `cwd`'s session files; none when Claude Code says no projects folder or the worktree has none yet.
pub(super) fn folder(cwd: &Path) -> Option<PathBuf> {
    let projects = projects()?;
    let cwd = std::fs::canonicalize(cwd).ok()?;
    let name = folder_name(&cwd.to_string_lossy());
    if name.len() <= NAME_LIMIT {
        return Some(projects.join(name));
    }
    std::fs::read_dir(projects)
        .ok()?
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(&name[..NAME_LIMIT])
        })
        .map(|entry| entry.path())
}

/// A path as Claude Code names its folder: each character outside `A-Z`, `a-z` and `0-9` as `-`.
pub(super) fn folder_name(path: &str) -> String {
    path.chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char
            } else {
                '-'
            }
        })
        .collect()
}

/// The file of session `id` in `folder`.
pub(super) fn file(folder: &Path, id: &str) -> PathBuf {
    folder.join(format!("{id}.jsonl"))
}

/// The turns of a session file as the events a live session sends: the chain from the last line back through each `parentUuid`, oldest first. A branch a rewind left behind is off the chain; a compaction's boundary goes on through its `logicalParentUuid`.
pub(super) fn history(file: &str, cwd: &Path) -> Vec<HarnessEvent> {
    let lines: Vec<Value> = file
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|line| line["uuid"].is_string())
        .collect();
    let by_id: HashMap<&str, &Value> = lines
        .iter()
        .filter_map(|line| Some((line["uuid"].as_str()?, line)))
        .collect();
    let mut chain = vec![];
    let mut next = lines.last();
    // The chain is no longer than the file, so a loop in it ends.
    while let Some(line) = next.filter(|_| chain.len() < lines.len()) {
        chain.push(line);
        let parent = line["parentUuid"]
            .as_str()
            .or(line["logicalParentUuid"].as_str());
        next = parent.and_then(|parent| by_id.get(parent).copied());
    }
    let mut tools = HashMap::new();
    chain
        .into_iter()
        .rev()
        .filter(|line| {
            line["isSidechain"] != true
                && line["isMeta"] != true
                && line["isCompactSummary"] != true
        })
        .flat_map(|line| line_events(line, &mut tools, cwd))
        .collect()
}

/// The worktree's sessions in `folder`, newest first, without `own` and those in `held`, each labelled by its last prompt. A file without a prompt is left out.
pub(super) fn sessions(folder: &Path, own: Option<&str>, held: &[String]) -> Vec<Session> {
    let now = SystemTime::now();
    let mut files: Vec<(String, SystemTime)> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let id = path
                .file_name()?
                .to_str()?
                .strip_suffix(".jsonl")?
                .to_string();
            let free = Some(id.as_str()) != own && !held.contains(&id);
            let changed = entry.metadata().ok()?.modified().ok()?;
            free.then_some((id, changed))
        })
        .collect();
    files.sort_by_key(|(_, changed)| std::cmp::Reverse(*changed));
    files
        .into_iter()
        .filter_map(|(id, changed)| {
            let label = last_prompt(&std::fs::read_to_string(file(folder, &id)).ok()?)?;
            let age = now.duration_since(changed).map_or(0, |age| age.as_secs());
            Some(Session { id, label, age })
        })
        .collect()
}

// `projectsDirectory` in the JSON `claude auth status` prints, read once per app run.
fn projects() -> Option<PathBuf> {
    static PROJECTS: OnceLock<Option<PathBuf>> = OnceLock::new();
    PROJECTS
        .get_or_init(|| {
            let output = login_shell(HARNESS.binary, &["auth", "status"])
                .output()
                .ok()?;
            let status: Value = serde_json::from_slice(&output.stdout).ok()?;
            status["projectsDirectory"].as_str().map(PathBuf::from)
        })
        .clone()
}

fn line_events(line: &Value, tools: &mut HashMap<String, String>, cwd: &Path) -> Vec<HarnessEvent> {
    let content = &line["message"]["content"];
    let at = blocks::timestamp(&line["timestamp"]);
    match (line["type"].as_str(), line["subtype"].as_str()) {
        (Some("user"), _) if line["promptSource"].is_string() => user(content, at),
        (Some("user"), _)
            if content
                .as_str()
                .is_some_and(|text| text.contains("<command-name>")) =>
        {
            user(content, at)
        }
        (Some("user"), _) => blocks::tool_results(content, &line["toolUseResult"], tools),
        (Some("assistant"), _) => blocks::assistant(line, Streamed::default(), tools, cwd),
        (Some("system"), Some("local_command")) => {
            let text = line["content"].as_str().unwrap_or_default();
            let text = text
                .trim()
                .trim_start_matches("<local-command-stdout>")
                .trim_end_matches("</local-command-stdout>")
                .trim();
            if text.is_empty() {
                return vec![];
            }
            vec![
                HarnessEvent::MessageChunk { text: text.into() },
                HarnessEvent::MessageEnd { failed: false, at },
            ]
        }
        (Some("system"), Some("compact_boundary")) => vec![HarnessEvent::Compaction {
            summary: String::new(),
            tokens_before: count(&line["compactMetadata"]["preTokens"]),
        }],
        _ => vec![],
    }
}

fn user(content: &Value, at: Option<f64>) -> Vec<HarnessEvent> {
    vec![HarnessEvent::UserMessageChunk {
        text: blocks::user_text(content),
        images: blocks::images(content),
        at,
    }]
}

fn last_prompt(file: &str) -> Option<String> {
    file.lines()
        .rev()
        .filter(|line| line.contains("\"last-prompt\""))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|line| line["type"] == "last-prompt")?["lastPrompt"]
        .as_str()?
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(String::from)
}
