use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use specta::Type;

use super::{claude, codex, pi};
use crate::lock;

const HOUR: f64 = 3_600_000.0;

const DEPTH: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reply {
    pub(crate) id: String,
    pub(crate) at: f64,
    pub(crate) model: String,
    pub(crate) input: f64,
    pub(crate) cache_read: f64,
    pub(crate) cache_write: f64,
    pub(crate) output: f64,
    pub(crate) cost: Option<f64>,
    pub(crate) saved: Option<f64>,
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpendRow {
    pub hour: f64,
    pub harness: String,
    pub model: String,
    pub input: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub output: f64,
    pub cost: Option<f64>,
    pub saved: Option<f64>,
}

#[derive(Serialize, Type, Debug, Clone, Default, PartialEq)]
pub struct Spend {
    pub rows: Vec<SpendRow>,
    pub sessions: BTreeMap<String, u32>,
}

struct Source {
    harness: &'static str,
    folder: fn() -> Option<PathBuf>,
    replies: fn(&str) -> Vec<Reply>,
}

const SOURCES: [Source; 3] = [
    Source {
        harness: pi::NAME,
        folder: pi::spend::folder,
        replies: pi::spend::replies,
    },
    Source {
        harness: codex::NAME,
        folder: codex::spend::folder,
        replies: codex::spend::replies,
    },
    Source {
        harness: claude::NAME,
        folder: claude::spend::folder,
        replies: claude::spend::replies,
    },
];

pub fn read(since: f64) -> Spend {
    static READING: Mutex<()> = Mutex::new(());
    let _reading = lock(&READING);
    let mut tally = Tally::new(since);
    for source in &SOURCES {
        let Some(folder) = (source.folder)() else {
            continue;
        };
        let mut files = vec![];
        session_files(&folder, DEPTH, &mut files);
        for file in files {
            if let Some(replies) = file_replies(&file, since, source.replies) {
                tally.add(source.harness, &replies);
            }
        }
    }
    tally.spend()
}

fn session_files(dir: &Path, depth: usize, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() && depth > 0 {
            session_files(&path, depth - 1, files);
        } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "jsonl") {
            files.push(path);
        }
    }
}

struct Cached {
    modified: SystemTime,
    size: u64,
    replies: Arc<Vec<Reply>>,
}

static CACHE: LazyLock<Mutex<HashMap<PathBuf, Cached>>> = LazyLock::new(Mutex::default);

fn file_replies(path: &Path, since: f64, parse: fn(&str) -> Vec<Reply>) -> Option<Arc<Vec<Reply>>> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let changed = modified
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |age| age.as_millis() as f64);
    if changed < since {
        return None;
    }
    if let Some(cached) = lock(&CACHE).get(path)
        && cached.modified == modified
        && cached.size == meta.len()
    {
        return Some(cached.replies.clone());
    }
    let text = std::fs::read_to_string(path).ok()?;
    let replies = Arc::new(parse(&text));
    lock(&CACHE).insert(
        path.to_path_buf(),
        Cached {
            modified,
            size: meta.len(),
            replies: replies.clone(),
        },
    );
    Some(replies)
}

struct Tally {
    since: f64,
    seen: HashMap<&'static str, HashSet<String>>,
    rows: BTreeMap<(i64, &'static str, String), SpendRow>,
    sessions: BTreeMap<String, u32>,
}

impl Tally {
    fn new(since: f64) -> Tally {
        Tally {
            since,
            seen: HashMap::new(),
            rows: BTreeMap::new(),
            sessions: BTreeMap::new(),
        }
    }

    fn add(&mut self, harness: &'static str, replies: &[Reply]) {
        let seen = self.seen.entry(harness).or_default();
        let mut session = false;
        for reply in replies.iter().filter(|reply| reply.at >= self.since) {
            if reply.input + reply.cache_read + reply.cache_write + reply.output == 0.0 {
                continue;
            }
            session = true;
            if !seen.insert(reply.id.clone()) {
                continue;
            }
            let hour = (reply.at / HOUR).floor() * HOUR;
            let row = self
                .rows
                .entry((hour as i64, harness, reply.model.clone()))
                .or_insert_with(|| SpendRow {
                    hour,
                    harness: harness.into(),
                    model: reply.model.clone(),
                    input: 0.0,
                    cache_read: 0.0,
                    cache_write: 0.0,
                    output: 0.0,
                    cost: None,
                    saved: None,
                });
            row.input += reply.input;
            row.cache_read += reply.cache_read;
            row.cache_write += reply.cache_write;
            row.output += reply.output;
            row.cost = plus(row.cost, reply.cost);
            row.saved = plus(row.saved, reply.saved);
        }
        if session {
            *self.sessions.entry(harness.into()).or_default() += 1;
        }
    }

    fn spend(self) -> Spend {
        Spend {
            rows: self.rows.into_values().collect(),
            sessions: self.sessions,
        }
    }
}

fn plus(sum: Option<f64>, part: Option<f64>) -> Option<f64> {
    match (sum, part) {
        (None, None) => None,
        _ => Some(sum.unwrap_or(0.0) + part.unwrap_or(0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tally(harness: &'static str, files: &[&str], parse: fn(&str) -> Vec<Reply>) -> Spend {
        let mut tally = Tally::new(0.0);
        for file in files {
            tally.add(harness, &parse(file));
        }
        tally.spend()
    }

    #[test]
    fn pi_reply_in_two_files_counts_once() {
        let line = r#"{"type":"message","id":"e1","message":{"role":"assistant","provider":"provider-a","model":"model-a","responseId":"r1","timestamp":7200000,"usage":{"input":10,"output":5,"cacheRead":20,"cacheWrite":0,"cost":{"input":0.1,"output":0.1,"cacheRead":0.02,"cacheWrite":0,"total":0.22}}}}"#;
        let spend = tally(pi::NAME, &[line, line], pi::spend::replies);
        assert_eq!(spend.rows.len(), 1);
        let row = &spend.rows[0];
        assert_eq!(
            (row.hour, row.model.as_str()),
            (7_200_000.0, "provider-a/model-a")
        );
        assert_eq!((row.input, row.cache_read, row.output), (10.0, 20.0, 5.0));
        assert_eq!(row.cost, Some(0.22));
        assert_eq!(spend.sessions.get(pi::NAME), Some(&2));
    }

    #[test]
    fn pi_saving_prices_cache_reads_at_the_input_rate() {
        let line = r#"{"type":"message","id":"e1","message":{"role":"assistant","provider":"p","model":"m","responseId":"r1","timestamp":0,"usage":{"input":100,"output":0,"cacheRead":1000,"cacheWrite":0,"cost":{"input":0.0005,"output":0,"cacheRead":0.0005,"cacheWrite":0,"total":0.001}}}}"#;
        let replies = pi::spend::replies(line);
        let saved = replies[0].saved.unwrap_or_default();
        assert!((saved - 0.0045).abs() < 1e-12, "saved {saved}");
    }

    #[test]
    fn codex_uncached_input_leaves_out_the_cached_parts() {
        let file = [
            r#"{"timestamp":"2026-01-02T03:04:05.000Z","type":"turn_context","payload":{"model":"model-a"}}"#,
            r#"{"timestamp":"2026-01-02T03:04:06.000Z","type":"token_usage_record","payload":{"response_id":"r1","usage":{"input_tokens":1000,"cached_input_tokens":600,"cache_write_input_tokens":300,"output_tokens":50}}}"#,
        ]
        .join("\n");
        let replies = codex::spend::replies(&file);
        assert_eq!(replies.len(), 1);
        let reply = &replies[0];
        assert_eq!(reply.model, "model-a");
        assert_eq!(
            (
                reply.input,
                reply.cache_read,
                reply.cache_write,
                reply.output
            ),
            (100.0, 600.0, 300.0, 50.0)
        );
    }

    #[test]
    fn claude_message_on_two_lines_counts_once_with_the_last_counts() {
        let file = [
            r#"{"type":"assistant","timestamp":"2026-01-02T03:04:05.000Z","message":{"id":"m1","model":"model-a","usage":{"input_tokens":3,"cache_read_input_tokens":10,"cache_creation_input_tokens":0,"output_tokens":1}}}"#,
            r#"{"type":"assistant","timestamp":"2026-01-02T03:04:07.000Z","message":{"id":"m1","model":"model-a","usage":{"input_tokens":3,"cache_read_input_tokens":10,"cache_creation_input_tokens":0,"output_tokens":40}}}"#,
        ]
        .join("\n");
        let spend = tally(claude::NAME, &[&file], claude::spend::replies);
        assert_eq!(spend.rows.len(), 1);
        assert_eq!((spend.rows[0].input, spend.rows[0].output), (3.0, 40.0));
    }
}
