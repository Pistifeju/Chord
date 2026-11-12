//! The pi adapter: runs `pi --mode rpc` and turns its records into harness events. The only file that knows pi's records.

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::thread;

use serde_json::{Value, json};

use super::{Capability, HarnessEvent};
use crate::Error;

/// The binary to run when the user chose none.
pub const DEFAULT_BINARY: &str = "pi";
const INSTALL: &str = "npm install -g @earendil-works/pi-coding-agent";
const OLDEST_TESTED: (u32, u32, u32) = (1, 5, 0);

/// One running `pi --mode rpc` process.
pub struct Session {
    child: Child,
    stdin: Mutex<ChildStdin>,
}

impl Session {
    pub fn start(
        binary: &str,
        dir: &Path,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        check_version(binary)?;
        let mut child = through_login_shell(binary, &["--mode", "rpc"])
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| spawn_error(binary, e))?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(Error::Message("pi started without its pipes".into()));
        };
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            // Split on LF only: a JSON string may hold U+2028.
            while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
                if let Ok(record) = serde_json::from_slice::<Value>(&line) {
                    events_from(&record).into_iter().for_each(&on_event);
                }
                line.clear();
            }
            on_event(HarnessEvent::Exited);
        });
        Ok(Session {
            child,
            stdin: Mutex::new(stdin),
        })
    }

    pub fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Chat]
    }

    /// Sends a message; while the agent runs, pi queues it as a steer.
    pub fn send(&self, text: &str) -> Result<(), Error> {
        self.write(&json!({ "type": "prompt", "message": text, "streamingBehavior": "steer" }))
    }

    pub fn abort(&self) -> Result<(), Error> {
        self.write(&json!({ "type": "abort" }))
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
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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

fn check_version(binary: &str) -> Result<(), Error> {
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
        _ => Ok(()),
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

fn events_from(record: &Value) -> Vec<HarnessEvent> {
    match record["type"].as_str() {
        Some("agent_start") => vec![HarnessEvent::State { running: true }],
        Some("agent_settled") => vec![HarnessEvent::State { running: false }],
        Some("message_update") => {
            let update = &record["assistantMessageEvent"];
            match (update["type"].as_str(), update["delta"].as_str()) {
                (Some("text_delta"), Some(text)) => {
                    vec![HarnessEvent::MessageChunk { text: text.into() }]
                }
                _ => vec![],
            }
        }
        Some("message_end") => message_end(&record["message"]),
        Some("response") if record["success"] == false => vec![HarnessEvent::Notice {
            text: record["error"]
                .as_str()
                .unwrap_or("pi refused the command")
                .into(),
        }],
        _ => vec![],
    }
}

fn message_end(message: &Value) -> Vec<HarnessEvent> {
    match message["role"].as_str() {
        Some("user") => vec![HarnessEvent::UserMessageChunk {
            text: text_of(&message["content"]),
        }],
        Some("assistant") => {
            let mut events = vec![HarnessEvent::MessageEnd];
            if message["stopReason"] == "error" {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn events(record: &str) -> Vec<HarnessEvent> {
        events_from(&serde_json::from_str(record).expect("valid json"))
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
    fn a_thinking_delta_is_not_a_chunk() {
        let record = r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","contentIndex":0,"delta":"hm"}}"#;
        assert_eq!(events(record), vec![]);
    }

    #[test]
    fn a_user_message_joins_its_text_blocks() {
        let record = r#"{"type":"message_end","message":{"role":"user","content":[{"type":"text","text":"a"},{"type":"image","data":"x"},{"type":"text","text":"b"}]}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::UserMessageChunk {
                text: "a\nb".into()
            }]
        );
    }

    #[test]
    fn a_failed_reply_ends_and_says_why() {
        let record = r#"{"type":"message_end","message":{"role":"assistant","stopReason":"error","errorMessage":"No API key"}}"#;
        assert_eq!(
            events(record),
            vec![
                HarnessEvent::MessageEnd,
                HarnessEvent::Notice {
                    text: "No API key".into()
                }
            ]
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
            events(r#"{"type":"agent_settled","aborted":true}"#),
            vec![HarnessEvent::State { running: false }]
        );
    }

    #[test]
    fn a_version_line_parses() {
        assert_eq!(parse_version("1.5.0"), Some((1, 5, 0)));
        assert_eq!(parse_version("welcome back"), None);
    }
}
