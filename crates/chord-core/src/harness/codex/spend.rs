//! The replies a Codex session file records, with their tokens, and the folder Codex keeps its session files in.

use std::path::PathBuf;
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;

use super::{HARNESS, requests};
use crate::harness::models::{self, Step};
use crate::harness::spend::Reply;
use crate::harness::timestamp;

#[derive(Deserialize)]
struct Record {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    timestamp: Value,
    #[serde(default)]
    payload: Payload,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Payload {
    model: Option<String>,
    response_id: Option<String>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Usage {
    input_tokens: f64,
    cached_input_tokens: f64,
    cache_write_input_tokens: f64,
    output_tokens: f64,
}

/// The `token_usage_record`s of a session file, keyed by `response_id`. A record without a model takes the last `turn_context`'s.
pub(crate) fn replies(text: &str) -> Vec<Reply> {
    let mut model = String::new();
    let mut replies = vec![];
    let lines = text
        .lines()
        .filter(|line| line.contains("token_usage_record") || line.contains("turn_context"));
    for line in lines {
        let Ok(record) = serde_json::from_str::<Record>(line) else {
            continue;
        };
        let Payload {
            model: named,
            response_id,
            usage,
        } = record.payload;
        match record.kind.as_str() {
            "turn_context" => {
                if let Some(named) = named {
                    model = named;
                }
            }
            "token_usage_record" => {
                let (Some(id), Some(usage), Some(at)) =
                    (response_id, usage, timestamp(&record.timestamp))
                else {
                    continue;
                };
                // Codex's input holds the cached and the cache write parts.
                let cached = usage.cached_input_tokens + usage.cache_write_input_tokens;
                replies.push(Reply {
                    id,
                    at,
                    model: named.unwrap_or_else(|| model.clone()),
                    input: (usage.input_tokens - cached).max(0.0),
                    cache_read: usage.cached_input_tokens,
                    cache_write: usage.cache_write_input_tokens,
                    output: usage.output_tokens,
                    cost: None,
                    saved: None,
                });
            }
            _ => {}
        }
    }
    replies
}

/// Codex's sessions folder: the `codexHome` its app server names in the `initialize` answer, plus `sessions`. Asked until Codex answers once per app run.
pub(crate) fn folder() -> Option<PathBuf> {
    static FOLDER: OnceLock<PathBuf> = OnceLock::new();
    if let Some(folder) = FOLDER.get() {
        return Some(folder.clone());
    }
    let home = models::answer(
        &HARNESS,
        &["app-server"],
        &[requests::initialize(1)],
        |record| match record["id"].as_u64() {
            Some(1) => Step::Done(record["result"]["codexHome"].as_str().map(PathBuf::from)),
            _ => Step::Wait,
        },
    );
    let folder = home.ok().flatten().flatten()?.join("sessions");
    Some(FOLDER.get_or_init(|| folder).clone())
}
