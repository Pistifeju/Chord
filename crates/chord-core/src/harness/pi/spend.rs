use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;

use crate::harness::spend::Reply;
use crate::harness::{login_shell, timestamp};

#[derive(Deserialize)]
struct Entry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    timestamp: Value,
    message: Option<Message>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    #[serde(default)]
    role: String,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    model: String,
    response_id: Option<String>,
    timestamp: Option<f64>,
    usage: Option<Usage>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Usage {
    input: f64,
    output: f64,
    cache_read: f64,
    cache_write: f64,
    cost: Option<Cost>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Cost {
    input: f64,
    cache_read: f64,
    total: f64,
}

pub(crate) fn replies(text: &str) -> Vec<Reply> {
    text.lines()
        .filter(|line| line.contains("\"usage\""))
        .filter_map(|line| serde_json::from_str::<Entry>(line).ok())
        .filter_map(reply)
        .collect()
}

fn reply(entry: Entry) -> Option<Reply> {
    let message = entry.message?;
    if message.role != "assistant" {
        return None;
    }
    let usage = message.usage?;
    let id = message.response_id.unwrap_or(entry.id);
    if id.is_empty() {
        return None;
    }
    let at = message.timestamp.or_else(|| timestamp(&entry.timestamp))?;
    let saved = usage.cost.as_ref().map(|cost| {
        if usage.input > 0.0 {
            usage.cache_read * cost.input / usage.input - cost.cache_read
        } else {
            0.0
        }
    });
    Some(Reply {
        id,
        at,
        model: format!("{}/{}", message.provider, message.model),
        input: usage.input,
        cache_read: usage.cache_read,
        cache_write: usage.cache_write,
        output: usage.output,
        cost: usage.cost.map(|cost| cost.total),
        saved,
    })
}

pub(crate) fn folder() -> Option<PathBuf> {
    static FOLDER: OnceLock<Option<PathBuf>> = OnceLock::new();
    FOLDER
        .get_or_init(|| {
            let home = PathBuf::from(std::env::var_os("HOME")?);
            let script = r#"printf 'session=%s\nagent=%s\n' "$PI_CODING_AGENT_SESSION_DIR" "$PI_CODING_AGENT_DIR""#;
            let output = login_shell("sh", &["-c", script]).output().ok();
            let text = output.map_or_else(String::new, |output| {
                String::from_utf8_lossy(&output.stdout).into_owned()
            });
            let variable = |name: &str| {
                text.lines()
                    .rev()
                    .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            };
            Some(match (variable("session"), variable("agent")) {
                (Some(session), _) => expand(session, &home),
                (None, Some(agent)) => expand(agent, &home).join("sessions"),
                (None, None) => home.join(".pi/agent/sessions"),
            })
        })
        .clone()
}

fn expand(path: &str, home: &Path) -> PathBuf {
    path.strip_prefix("~/")
        .map_or_else(|| PathBuf::from(path), |rest| home.join(rest))
}
