//! The replies a Claude Code session file records, with their tokens, and the folder Claude Code keeps its session files in.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use super::transcript;
use crate::harness::spend::Reply;
use crate::harness::timestamp;

#[derive(Deserialize)]
struct Line {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    timestamp: Value,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    #[serde(default)]
    model: String,
    usage: Option<Usage>,
}

/// Claude Code writes null for a count it lacks.
#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<f64>,
    cache_read_input_tokens: Option<f64>,
    cache_creation_input_tokens: Option<f64>,
    output_tokens: Option<f64>,
}

/// The `assistant` messages of a session file, keyed by `message.id`. A message spans several lines; the last one holds its final counts.
pub(crate) fn replies(text: &str) -> Vec<Reply> {
    let mut replies: Vec<Reply> = vec![];
    let mut index: HashMap<String, usize> = HashMap::new();
    for line in text.lines().filter(|line| line.contains("\"assistant\"")) {
        let Ok(line) = serde_json::from_str::<Line>(line) else {
            continue;
        };
        let Some(message) = line.message.filter(|_| line.kind == "assistant") else {
            continue;
        };
        let (Some(id), Some(usage), Some(at)) =
            (message.id, message.usage, timestamp(&line.timestamp))
        else {
            continue;
        };
        let reply = Reply {
            id,
            at,
            model: message.model,
            input: usage.input_tokens.unwrap_or_default(),
            cache_read: usage.cache_read_input_tokens.unwrap_or_default(),
            cache_write: usage.cache_creation_input_tokens.unwrap_or_default(),
            output: usage.output_tokens.unwrap_or_default(),
            cost: None,
            saved: None,
        };
        match index.get(&reply.id) {
            Some(&at) => replies[at] = reply,
            None => {
                index.insert(reply.id.clone(), replies.len());
                replies.push(reply);
            }
        }
    }
    replies
}

/// Claude Code's projects folder, which holds a folder of session files per project.
pub(crate) fn folder() -> Option<PathBuf> {
    transcript::projects()
}
