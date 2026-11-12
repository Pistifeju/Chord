//! The pi adapter: runs `pi --mode rpc` and turns its records into harness events. The only file that knows pi's records.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};

use super::shell::read_only_title;
use super::{
    Capability, DialogAnswer, DialogAsk, HarnessEvent, Image, SlashCommand, SlashKind, ToolOutcome,
};
use crate::Error;

/// The binary to run when the user chose none.
pub const DEFAULT_BINARY: &str = "pi";
const INSTALL: &str = "npm install -g @earendil-works/pi-coding-agent";
const OLDEST_TESTED: (u32, u32, u32) = (1, 5, 0);

/// One running `pi --mode rpc` process.
pub struct Session {
    child: Arc<Mutex<Child>>,
    stdin: Mutex<ChildStdin>,
    stopped: Arc<AtomicBool>,
}

/// The file that holds a pane's pi session, so the pane's agent can open the same session again.
pub fn session_file(sessions_dir: &Path, pane: &str) -> Result<PathBuf, Error> {
    let plain = !pane.is_empty()
        && pane
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !plain {
        return Err(Error::Message(format!("\"{pane}\" is not a pane id")));
    }
    Ok(sessions_dir.join(format!("{pane}.jsonl")))
}

impl Session {
    pub fn start(
        binary: &str,
        dir: &Path,
        session_file: &Path,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        check_version(binary)?;
        if let Some(parent) = session_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // The same file on every start puts the pane back in its exact session.
        let session_file = session_file.to_string_lossy();
        let mut child = through_login_shell(binary, &["--mode", "rpc", "--session", &session_file])
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| spawn_error(binary, e))?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(Error::Message("pi started without its pipes".into()));
        };
        let child = Arc::new(Mutex::new(child));
        let reader_child = Arc::clone(&child);
        let stopped = Arc::new(AtomicBool::new(false));
        let reader_stopped = Arc::clone(&stopped);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            let mut open_calls = OpenCalls::default();
            // Split on LF only: a JSON string may hold U+2028.
            while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
                open_calls
                    .track(events_from_line(&line))
                    .into_iter()
                    .for_each(&on_event);
                line.clear();
            }
            open_calls.stop().into_iter().for_each(&on_event);
            // Only an agent that ended on its own is reported; a stopped one is already forgotten.
            if !reader_stopped.load(Ordering::SeqCst) {
                on_event(HarnessEvent::Exited {
                    code: exit_code(&reader_child),
                });
            }
        });
        let session = Session {
            child,
            stdin: Mutex::new(stdin),
            stopped,
        };
        // Without the list the slash menu stays hidden; the agent works without it.
        let _ = session.write(&json!({ "type": "get_commands" }));
        Ok(session)
    }

    pub fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat, Capability::Attachments, Capability::Steer]
    }

    /// Sends a message; while the agent runs, pi queues it as a steer.
    pub fn send(&self, text: &str, images: &[Image]) -> Result<(), Error> {
        self.write(&prompt_command(text, images))
    }

    pub fn abort(&self) -> Result<(), Error> {
        self.write(&json!({ "type": "abort" }))
    }

    /// Answers the dialog pi asked with `id`.
    pub fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        self.write(&dialog_response(id, answer))
    }

    fn write(&self, command: &Value) -> Result<(), Error> {
        let mut stdin = self.stdin.lock().unwrap_or_else(|e| e.into_inner());
        writeln!(stdin, "{command}")?;
        stdin.flush()?;
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let mut child = self.child.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn prompt_command(text: &str, images: &[Image]) -> Value {
    let images: Vec<Value> = images
        .iter()
        .map(|image| json!({ "type": "image", "data": image.data, "mimeType": image.mime_type }))
        .collect();
    json!({ "type": "prompt", "message": text, "images": images, "streamingBehavior": "steer" })
}

fn dialog_response(id: &str, answer: &DialogAnswer) -> Value {
    let mut response = json!({ "type": "extension_ui_response", "id": id });
    match answer {
        DialogAnswer::Value { value } => response["value"] = json!(value),
        DialogAnswer::Confirmed { confirmed } => response["confirmed"] = json!(confirmed),
        DialogAnswer::Cancelled => response["cancelled"] = json!(true),
    }
    response
}

// Stdout closed, so the process is ending; it gets a moment to report its code.
fn exit_code(child: &Mutex<Child>) -> Option<i32> {
    for _ in 0..50 {
        let waited = child
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .try_wait();
        match waited {
            Ok(Some(status)) => return status.code(),
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    None
}

// The user's login shell finds pi and its API keys the way their terminal does.
// Without a SHELL, pi runs directly and finds itself on the app's own PATH.
fn through_login_shell(binary: &str, args: &[&str]) -> Command {
    match std::env::var("SHELL") {
        Ok(shell) if !shell.is_empty() => {
            let mut command = Command::new(shell);
            command
                .args(["-l", "-c", r#"exec "$0" "$@""#, binary])
                .args(args);
            command
        }
        _ => {
            let mut command = Command::new(binary);
            command.args(args);
            command
        }
    }
}

fn spawn_error(binary: &str, error: std::io::Error) -> Error {
    match error.kind() {
        ErrorKind::NotFound => missing_pi(binary),
        _ => Error::Message(format!("pi failed to start: {error}")),
    }
}

fn missing_pi(binary: &str) -> Error {
    Error::MissingTool {
        tool: binary.into(),
        install: INSTALL.into(),
    }
}

static CHECKED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

// A binary that passed is not asked again while the app runs: each ask costs a login shell and a pi start.
fn check_version(binary: &str) -> Result<(), Error> {
    let mut checked = CHECKED.lock().unwrap_or_else(PoisonError::into_inner);
    if checked.contains(binary) {
        return Ok(());
    }
    let output = through_login_shell(binary, &["--version"])
        .output()
        .map_err(|e| spawn_error(binary, e))?;
    if output.status.code() == Some(127) {
        return Err(missing_pi(binary));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text.lines().rev().find_map(parse_version);
    match version {
        Some(found) if found < OLDEST_TESTED => Err(Error::Message(format!(
            "pi {}.{}.{} is older than the tested {}.{}.{}. Upgrade it with: {INSTALL}",
            found.0, found.1, found.2, OLDEST_TESTED.0, OLDEST_TESTED.1, OLDEST_TESTED.2
        ))),
        _ => {
            checked.insert(binary.to_string());
            Ok(())
        }
    }
}

fn parse_version(line: &str) -> Option<(u32, u32, u32)> {
    let mut parts = line.trim().split('.').map(|part| part.parse::<u32>());
    Some((
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ))
}

#[derive(Deserialize)]
struct Kind {
    #[serde(rename = "type")]
    kind: String,
    // A reply chunk needs only this field, so its record is not parsed a second time.
    #[serde(rename = "assistantMessageEvent")]
    update: Option<Value>,
}

const READ_KINDS: [&str; 9] = [
    "extension_ui_request",
    "agent_start",
    "agent_settled",
    "message_update",
    "message_end",
    "tool_execution_start",
    "tool_execution_end",
    "queue_update",
    "response",
];

// A run that settles or dies leaves calls pi never closed; they end as stopped so none shows as running.
#[derive(Default)]
struct OpenCalls(Vec<String>);

impl OpenCalls {
    fn track(&mut self, events: Vec<HarnessEvent>) -> Vec<HarnessEvent> {
        let mut out = Vec::new();
        for event in events {
            match &event {
                HarnessEvent::ToolStart { id, .. } => self.0.push(id.clone()),
                HarnessEvent::ToolEnd { id, .. } => self.0.retain(|open| open != id),
                HarnessEvent::State { running: false } => out.extend(self.stop()),
                _ => {}
            }
            out.push(event);
        }
        out
    }

    fn stop(&mut self) -> Vec<HarnessEvent> {
        self.0
            .drain(..)
            .map(|id| HarnessEvent::ToolEnd {
                id,
                outcome: ToolOutcome::Stopped,
                images: vec![],
            })
            .collect()
    }
}

// Tool output and turn summaries are large and unused, so only the records `events_from` reads get a full parse.
fn events_from_line(line: &[u8]) -> Vec<HarnessEvent> {
    match serde_json::from_slice::<Kind>(line) {
        Ok(Kind { kind, update }) if kind == "message_update" => {
            message_update(update.as_ref().unwrap_or(&Value::Null))
        }
        Ok(Kind { kind, .. }) if READ_KINDS.contains(&kind.as_str()) => {
            serde_json::from_slice::<Value>(line)
                .map(|record| events_from(&record))
                .unwrap_or_default()
        }
        _ => vec![],
    }
}

fn events_from(record: &Value) -> Vec<HarnessEvent> {
    match record["type"].as_str() {
        Some("agent_start") => vec![HarnessEvent::State { running: true }],
        Some("agent_settled") if record["aborted"] == true => {
            vec![
                HarnessEvent::State { running: false },
                HarnessEvent::Aborted,
            ]
        }
        Some("agent_settled") => vec![HarnessEvent::State { running: false }],
        Some("message_update") => message_update(&record["assistantMessageEvent"]),
        Some("tool_execution_start") => {
            let (title, detail) = describe_tool(
                record["toolName"].as_str().unwrap_or("tool"),
                &record["args"],
            );
            vec![HarnessEvent::ToolStart {
                id: record["toolCallId"].as_str().unwrap_or_default().into(),
                title,
                detail,
            }]
        }
        Some("tool_execution_end") => vec![HarnessEvent::ToolEnd {
            id: record["toolCallId"].as_str().unwrap_or_default().into(),
            outcome: if record["isError"] == true {
                ToolOutcome::Failed
            } else {
                ToolOutcome::Done
            },
            images: images_of(&record["result"]["content"]),
        }],
        Some("message_end") => message_end(&record["message"]),
        Some("queue_update") => vec![HarnessEvent::Queue {
            steering: record["steering"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|text| text.as_str().map(collapse_skill))
                .collect(),
        }],
        Some("extension_ui_request") => dialog_from(record).into_iter().collect(),
        Some("response") if record["command"] == "get_commands" => {
            vec![HarnessEvent::Commands {
                commands: record["data"]["commands"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|command| {
                        Some(SlashCommand {
                            name: command["name"].as_str()?.into(),
                            description: command["description"].as_str().unwrap_or_default().into(),
                            kind: match command["source"].as_str() {
                                Some("skill") => SlashKind::Skill,
                                Some("prompt") => SlashKind::Prompt,
                                _ => SlashKind::Extension,
                            },
                            origin: package_name(&command["sourceInfo"]),
                        })
                    })
                    .collect(),
            }]
        }
        Some("response") if record["success"] == false => vec![HarnessEvent::Notice {
            text: record["error"]
                .as_str()
                .unwrap_or("pi refused the command")
                .into(),
        }],
        _ => vec![],
    }
}

// The package a command comes from, from a source like `npm:@scope/name@1.2.0` or `https://host/user/name.git`; none for a local file.
fn package_name(source_info: &Value) -> Option<String> {
    if source_info["origin"] != "package" {
        return None;
    }
    let source = source_info["source"].as_str()?;
    let last = source.trim_end_matches('/').rsplit('/').next()?;
    let name = last.strip_suffix(".git").unwrap_or(last);
    let name = name.rsplit(':').next()?;
    let name = match name.rfind('@') {
        Some(at) if at > 0 => &name[..at],
        _ => name,
    };
    (!name.is_empty()).then(|| name.to_string())
}

// Select, confirm and input wait for an answer; notify and the status methods do not.
fn dialog_from(record: &Value) -> Option<HarnessEvent> {
    let text = |key: &str| record[key].as_str().unwrap_or_default().to_string();
    let ask = match record["method"].as_str()? {
        "select" => DialogAsk::Select {
            options: record["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|option| option.as_str().map(String::from))
                .collect(),
        },
        "confirm" => DialogAsk::Confirm {
            message: text("message"),
        },
        "input" => DialogAsk::Input {
            placeholder: text("placeholder"),
        },
        _ => return None,
    };
    Some(HarnessEvent::Dialog {
        id: text("id"),
        title: text("title"),
        ask,
    })
}

fn message_update(update: &Value) -> Vec<HarnessEvent> {
    match (update["type"].as_str(), update["delta"].as_str()) {
        (Some("text_delta"), Some(text)) => {
            vec![HarnessEvent::MessageChunk { text: text.into() }]
        }
        (Some("thinking_start"), _) => vec![HarnessEvent::ThoughtStart],
        (Some("thinking_delta"), Some(text)) => {
            vec![HarnessEvent::ThoughtChunk { text: text.into() }]
        }
        _ => vec![],
    }
}

// A shell command that only reads shows as Read, Search or List; an unknown tool shows its name and argument values.
fn describe_tool(name: &str, args: &Value) -> (String, String) {
    let text = |key: &str| args[key].as_str().unwrap_or_default().to_string();
    match name {
        "read" => ("Read".into(), text("path")),
        "grep" | "find" => ("Search".into(), text("pattern")),
        "ls" => ("List".into(), text("path")),
        "edit" => ("Edit".into(), text("path")),
        "write" => ("Write".into(), text("path")),
        "bash" => {
            let command = text("command");
            let title = read_only_title(&command).unwrap_or("Run");
            (title.into(), command)
        }
        _ => (readable_name(name), argument_values(args)),
    }
}

// "mcp__github__list_issues" reads as "Github list issues".
fn readable_name(name: &str) -> String {
    let words = name
        .trim_start_matches("mcp__")
        .split(['_', '-'])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Tool".into(),
    }
}

fn argument_values(args: &Value) -> String {
    let Some(map) = args.as_object() else {
        return String::new();
    };
    map.values()
        .map(|value| match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn message_end(message: &Value) -> Vec<HarnessEvent> {
    match message["role"].as_str() {
        Some("user") => vec![HarnessEvent::UserMessageChunk {
            text: collapse_skill(&text_of(&message["content"])),
            images: images_of(&message["content"]),
        }],
        Some("assistant") => {
            let failed = message["stopReason"] == "error";
            let mut events = vec![HarnessEvent::MessageEnd { failed }];
            if failed {
                events.push(HarnessEvent::Notice {
                    text: message["errorMessage"]
                        .as_str()
                        .unwrap_or("The reply failed")
                        .into(),
                });
            }
            events
        }
        _ => vec![],
    }
}

// pi expands `/skill:name args` into `<skill name="name" location="...">\n...\n</skill>\n\nargs`; the chat shows what the user typed.
fn collapse_skill(text: &str) -> String {
    let collapsed = text
        .strip_prefix("<skill name=\"")
        .and_then(|rest| rest.split_once("\" location=\""))
        .and_then(|(name, rest)| {
            let (_, after) = rest.split_once("\">\n")?;
            let (_, args) = after.split_once("\n</skill>")?;
            Some((name, args.trim_start_matches("\n\n")))
        });
    match collapsed {
        Some((name, "")) => format!("/skill:{name}"),
        Some((name, args)) => format!("/skill:{name} {args}"),
        None => text.into(),
    }
}

fn text_of(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn images_of(content: &Value) -> Vec<Image> {
    let Some(blocks) = content.as_array() else {
        return vec![];
    };
    blocks
        .iter()
        .filter(|block| block["type"] == "image")
        .filter_map(|block| {
            Some(Image {
                data: block["data"].as_str()?.into(),
                mime_type: block["mimeType"].as_str()?.into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(record: &str) -> Vec<HarnessEvent> {
        events_from_line(record.as_bytes())
    }

    #[test]
    fn a_text_delta_becomes_a_chunk() {
        let record = r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"Hi"}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::MessageChunk { text: "Hi".into() }]
        );
    }

    #[test]
    fn a_thinking_delta_becomes_a_thought() {
        let record = r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","contentIndex":0,"delta":"hm"}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::ThoughtChunk { text: "hm".into() }]
        );
    }

    #[test]
    fn a_read_only_shell_command_is_titled_by_what_it_does() {
        let record = r#"{"type":"tool_execution_start","toolCallId":"c1","toolName":"bash","args":{"command":"grep -rn foo src"}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::ToolStart {
                id: "c1".into(),
                title: "Search".into(),
                detail: "grep -rn foo src".into()
            }]
        );
    }

    #[test]
    fn an_unknown_tool_shows_its_name_and_argument_values() {
        let record = r#"{"type":"tool_execution_start","toolCallId":"c2","toolName":"mcp__github__list_issues","args":{"repo":"a/b"}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::ToolStart {
                id: "c2".into(),
                title: "Github list issues".into(),
                detail: "a/b".into()
            }]
        );
    }

    #[test]
    fn a_settled_run_stops_the_calls_it_left_open() {
        let mut open = OpenCalls::default();
        let started = |id: &str| HarnessEvent::ToolStart {
            id: id.into(),
            title: "Run".into(),
            detail: "sleep 9".into(),
        };
        assert_eq!(open.track(vec![started("a"), started("b")]).len(), 2);
        let ended = HarnessEvent::ToolEnd {
            id: "a".into(),
            outcome: ToolOutcome::Done,
            images: vec![],
        };
        open.track(vec![ended]);
        assert_eq!(
            open.track(vec![HarnessEvent::State { running: false }]),
            vec![
                HarnessEvent::ToolEnd {
                    id: "b".into(),
                    outcome: ToolOutcome::Stopped,
                    images: vec![]
                },
                HarnessEvent::State { running: false }
            ]
        );
    }

    #[test]
    fn a_thinking_start_begins_a_thought() {
        let record = r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_start","contentIndex":0}}"#;
        assert_eq!(events(record), vec![HarnessEvent::ThoughtStart]);
    }

    #[test]
    fn a_failed_tool_call_ends_failed() {
        let record = r#"{"type":"tool_execution_end","toolCallId":"c1","toolName":"bash","result":{},"isError":true}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::ToolEnd {
                id: "c1".into(),
                outcome: ToolOutcome::Failed,
                images: vec![]
            }]
        );
    }

    #[test]
    fn a_tool_result_keeps_its_images() {
        let record = r#"{"type":"tool_execution_end","toolCallId":"c1","toolName":"read","result":{"content":[{"type":"text","text":"Read image"},{"type":"image","data":"x","mimeType":"image/png"}]},"isError":false}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::ToolEnd {
                id: "c1".into(),
                outcome: ToolOutcome::Done,
                images: vec![Image {
                    data: "x".into(),
                    mime_type: "image/png".into()
                }]
            }]
        );
    }

    #[test]
    fn a_user_message_joins_its_text_blocks_and_keeps_its_images() {
        let record = r#"{"type":"message_end","message":{"role":"user","content":[{"type":"text","text":"a"},{"type":"image","data":"x","mimeType":"image/png"},{"type":"text","text":"b"}]}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::UserMessageChunk {
                text: "a\nb".into(),
                images: vec![Image {
                    data: "x".into(),
                    mime_type: "image/png".into()
                }]
            }]
        );
    }

    #[test]
    fn an_expanded_skill_shows_as_the_command_the_user_typed() {
        let record = r#"{"type":"message_end","message":{"role":"user","content":[{"type":"text","text":"<skill name=\"deploy\" location=\"deploy/SKILL.md\">\nReferences are relative to deploy.\n\nSteps\n</skill>\n\nto staging"}]}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::UserMessageChunk {
                text: "/skill:deploy to staging".into(),
                images: vec![]
            }]
        );
        assert_eq!(
            collapse_skill("<skill name=\"deploy\" location=\"x\">\nbody\n</skill>"),
            "/skill:deploy"
        );
        assert_eq!(collapse_skill("<skill is a tag"), "<skill is a tag");
    }

    #[test]
    fn the_command_list_becomes_slash_commands() {
        let record = r#"{"type":"response","command":"get_commands","success":true,"data":{"commands":[{"name":"skill:deploy","description":"Ship it","source":"skill"},{"name":"compact","source":"extension"},{"name":"review","source":"prompt","sourceInfo":{"source":"npm:@acme/pi-tools@1.2.0","origin":"package"}}]}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::Commands {
                commands: vec![
                    SlashCommand {
                        name: "skill:deploy".into(),
                        description: "Ship it".into(),
                        kind: SlashKind::Skill,
                        origin: None
                    },
                    SlashCommand {
                        name: "compact".into(),
                        description: "".into(),
                        kind: SlashKind::Extension,
                        origin: None
                    },
                    SlashCommand {
                        name: "review".into(),
                        description: "".into(),
                        kind: SlashKind::Prompt,
                        origin: Some("pi-tools".into())
                    }
                ]
            }]
        );
    }

    #[test]
    fn a_queue_update_lists_the_waiting_steers() {
        let record = r#"{"type":"queue_update","steering":["a","b"],"followUp":[]}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::Queue {
                steering: vec!["a".into(), "b".into()]
            }]
        );
    }

    #[test]
    fn a_prompt_carries_its_images() {
        let image = Image {
            data: "x".into(),
            mime_type: "image/png".into(),
        };
        assert_eq!(
            prompt_command("look", &[image]),
            json!({ "type": "prompt", "message": "look", "images": [{ "type": "image", "data": "x", "mimeType": "image/png" }], "streamingBehavior": "steer" })
        );
    }

    #[test]
    fn a_failed_reply_ends_and_says_why() {
        let record = r#"{"type":"message_end","message":{"role":"assistant","stopReason":"error","errorMessage":"No API key"}}"#;
        assert_eq!(
            events(record),
            vec![
                HarnessEvent::MessageEnd { failed: true },
                HarnessEvent::Notice {
                    text: "No API key".into()
                }
            ]
        );
    }

    #[test]
    fn a_select_request_opens_a_dialog_and_a_notify_does_not() {
        let dialog = r#"{"type":"extension_ui_request","id":"u1","method":"select","title":"Pick","options":["a"]}"#;
        let notify =
            r#"{"type":"extension_ui_request","id":"u2","method":"notify","message":"hi"}"#;
        assert_eq!(
            events(dialog),
            vec![HarnessEvent::Dialog {
                id: "u1".into(),
                title: "Pick".into(),
                ask: DialogAsk::Select {
                    options: vec!["a".into()]
                },
            }]
        );
        assert_eq!(events(notify), vec![]);
    }

    #[test]
    fn an_answer_goes_back_under_the_request_id() {
        assert_eq!(
            dialog_response(
                "u1",
                &DialogAnswer::Value {
                    value: "Allow".into()
                }
            ),
            json!({"type": "extension_ui_response", "id": "u1", "value": "Allow"})
        );
        assert_eq!(
            dialog_response("u2", &DialogAnswer::Confirmed { confirmed: false }),
            json!({"type": "extension_ui_response", "id": "u2", "confirmed": false})
        );
        assert_eq!(
            dialog_response("u3", &DialogAnswer::Cancelled),
            json!({"type": "extension_ui_response", "id": "u3", "cancelled": true})
        );
    }

    #[test]
    fn a_refused_command_becomes_a_notice() {
        let record = r#"{"type":"response","command":"prompt","success":false,"error":"busy"}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::Notice {
                text: "busy".into()
            }]
        );
    }

    #[test]
    fn the_run_starts_and_settles() {
        assert_eq!(
            events(r#"{"type":"agent_start"}"#),
            vec![HarnessEvent::State { running: true }]
        );
        assert_eq!(
            events(r#"{"type":"agent_settled","aborted":false}"#),
            vec![HarnessEvent::State { running: false }]
        );
        assert_eq!(
            events(r#"{"type":"agent_settled","aborted":true}"#),
            vec![
                HarnessEvent::State { running: false },
                HarnessEvent::Aborted
            ]
        );
    }

    #[test]
    fn a_version_line_parses() {
        assert_eq!(parse_version("1.5.0"), Some((1, 5, 0)));
        assert_eq!(parse_version("welcome back"), None);
    }
}
