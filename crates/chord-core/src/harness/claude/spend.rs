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

#[derive(Deserialize)]
struct Usage {
    input_tokens: Option<f64>,
    cache_read_input_tokens: Option<f64>,
    cache_creation_input_tokens: Option<f64>,
    output_tokens: Option<f64>,
}

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

pub(crate) fn folder() -> Option<PathBuf> {
    transcript::projects()
}
