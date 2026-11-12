//! The pi adapter: runs `pi --mode rpc` and turns its records into harness events. The only file that knows pi's records.

use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};
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
    UsageWindow,
};
use crate::Error;

/// The binary to run when the user chose none.
pub const DEFAULT_BINARY: &str = "pi";
const INSTALL: &str = "npm install -g @earendil-works/pi-coding-agent";
const OLDEST_TESTED: (u32, u32, u32) = (1, 5, 0);

/// The extension Chord ships: the session commands RPC lacks, and the subscription usage windows.
const EXTENSION: &str = include_str!("pi_extension.ts");
const EXTENSION_FILE: &str = "chord-extension.ts";
/// The commands the extension adds start with this; the slash menu lists Chord's own names for them.
const EXTENSION_PREFIX: &str = "chord-";
/// The status key the extension reports the usage windows under.
const USAGE_STATUS: &str = "chord-usage";
/// The request id of a prompt that runs one of the extension's commands; its response means the session may have changed.
const SESSION_PROMPT: &str = "chord-session";
/// A fork dialog's id starts with this; the ids of the entries it lists follow, comma-separated, in the order of its options.
const FORK_DIALOG: &str = "chord-fork:";
/// The id of the model dialog; its options are `provider/model`.
const MODEL_DIALOG: &str = "chord-model";
/// The id of the thinking level dialog; its options are the levels, capitalized.
const THINKING_DIALOG: &str = "chord-thinking";

/// A dialog the adapter opens to change the session's model or thinking level.
#[derive(Clone, Copy)]
enum Picker {
    Model,
    Thinking,
}

impl Picker {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            MODEL_DIALOG => Some(Self::Model),
            THINKING_DIALOG => Some(Self::Thinking),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Model => MODEL_DIALOG,
            Self::Thinking => THINKING_DIALOG,
        }
    }

    fn command(self, answer: &DialogAnswer) -> Option<Value> {
        match self {
            Self::Model => model_command(answer),
            Self::Thinking => thinking_command(answer),
        }
    }
}

/// What pi offers after a slash in its own terminal and RPC does not list: each name and its description.
const SESSION_COMMANDS: [(&str, &str); 8] = [
    ("model", "Pick the model"),
    ("thinking", "Pick the thinking level"),
    ("new", "Start a new session"),
    ("resume", "Open another session of this worktree"),
    ("fork", "Start a new session from an earlier message"),
    ("clone", "Copy this session into a new one"),
    ("tree", "Go back to an earlier point of this session"),
    (
        "export",
        "Write this session to an HTML file, optionally at a path",
    ),
];

/// One running `pi --mode rpc` process.
pub struct Session {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    stopped: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
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

static EXTENSION_WRITE: Mutex<()> = Mutex::new(());

// Written only when it differs, and one start at a time, so no pi reads a half-written file.
fn installed_extension(dir: &Path) -> Result<PathBuf, Error> {
    let _writing = EXTENSION_WRITE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    std::fs::create_dir_all(dir)?;
    let file = dir.join(EXTENSION_FILE);
    if std::fs::read_to_string(&file).ok().as_deref() != Some(EXTENSION) {
        std::fs::write(&file, EXTENSION)?;
    }
    Ok(file)
}

impl Session {
    /// `extension_dir` is where the extension Chord ships is written for pi to load. Without an existing `session_file` pi starts a new session in its own session folder.
    pub fn start(
        binary: &str,
        dir: &Path,
        session_file: &Path,
        extension_dir: &Path,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        check_version(binary)?;
        let extension = installed_extension(extension_dir)?;
        let extension = extension.to_string_lossy();
        let held = session_file.is_file();
        let session_file = session_file.to_string_lossy();
        let mut args = vec!["--mode", "rpc", "--extension", &extension];
        // The same file on every start puts the pane back in its exact session.
        if held {
            args.extend(["--session", &session_file]);
        }
        let mut child = through_login_shell(binary, &args)
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
        let stdin = Arc::new(Mutex::new(stdin));
        let reader_stdin = Arc::clone(&stdin);
        let stopped = Arc::new(AtomicBool::new(false));
        let reader_stopped = Arc::clone(&stopped);
        let running = Arc::new(AtomicBool::new(false));
        let reader_running = Arc::clone(&running);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            let mut open_calls = OpenCalls::default();
            let mut shown = ShownSession::default();
            // Split on LF only: a JSON string may hold U+2028.
            while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
                let Read { events, requests } = read_line(&line, &mut shown);
                for event in open_calls.track(events) {
                    if let HarnessEvent::State { running } = event {
                        reader_running.store(running, Ordering::SeqCst);
                    }
                    on_event(event);
                }
                // A `/tree` pi refused mid-run left the session as it was, so reading it again would restart the live transcript; only the state and the stats, which are read-only, go out mid-run.
                let running = reader_running.load(Ordering::SeqCst);
                for request in requests {
                    if !running
                        || matches!(
                            request["type"].as_str(),
                            Some("get_state" | "get_session_stats")
                        )
                    {
                        let _ = write_to(&reader_stdin, &request);
                    }
                }
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
            stdin,
            stopped,
            running,
        };
        // Without the list the slash menu stays hidden; the agent works without it.
        let _ = session.write(&json!({ "type": "get_commands" }));
        // The session file may hold earlier messages; they come back as history.
        for request in session_requests() {
            let _ = session.write(&request);
        }
        Ok(session)
    }

    pub fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Chat,
            Capability::Attachments,
            Capability::Steer,
            Capability::Models,
            Capability::Context,
        ]
    }

    /// Sends a message; while the agent runs, pi queues it as a steer. A session command runs instead of being sent; `/resume` leaves out `held_elsewhere`, the session files other panes hold, so no two agents write one file.
    pub fn send(
        &self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[PathBuf],
    ) -> Result<(), Error> {
        let Some(command) = session_command(text.trim(), held_elsewhere) else {
            return self.write(&prompt_command(text, images));
        };
        // pi aborts the run to open another session; `/tree` mid-run it refuses itself and `/export` leaves the run alone; `/model` and `/thinking` only open a picker, and pi's refusal of the pick shows as a notice.
        let keeps_run = matches!(
            command_name(text.trim()),
            "/tree" | "/export" | "/model" | "/thinking"
        );
        if self.running.load(Ordering::SeqCst) && !keeps_run {
            return Err(Error::Message(
                "Wait for the agent to finish, then change the session".into(),
            ));
        }
        if !images.is_empty() {
            return Err(Error::Message(
                "Remove the images, then change the session".into(),
            ));
        }
        self.write(&command)
    }

    pub fn abort(&self) -> Result<(), Error> {
        self.write(&json!({ "type": "abort" }))
    }

    /// Answers the dialog pi asked with `id`; a fork dialog is the adapter's own, so its answer becomes the fork.
    pub fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        if let Some(picker) = Picker::from_id(id) {
            return self.write_if_picked(picker.command(answer));
        }
        match id.strip_prefix(FORK_DIALOG) {
            Some(entries) => match fork_command(entries, answer) {
                Some(command) => self.write(&command),
                None => Ok(()),
            },
            None => self.write(&dialog_response(id, answer)),
        }
    }

    fn write_if_picked(&self, command: Option<Value>) -> Result<(), Error> {
        command.map_or(Ok(()), |command| self.write(&command))
    }

    fn write(&self, command: &Value) -> Result<(), Error> {
        write_to(&self.stdin, command)
    }
}

fn write_to(stdin: &Mutex<ChildStdin>, command: &Value) -> Result<(), Error> {
    let mut stdin = stdin.lock().unwrap_or_else(PoisonError::into_inner);
    writeln!(stdin, "{command}")?;
    stdin.flush()?;
    Ok(())
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

// The RPC command a session command runs; resume and tree need a pick pi's RPC has no list for, so the extension runs them.
fn session_command(text: &str, held_elsewhere: &[PathBuf]) -> Option<Value> {
    let extension = |name: &str| json!({ "id": SESSION_PROMPT, "type": "prompt", "message": format!("/{EXTENSION_PREFIX}{name}") });
    // `/export` takes an optional output path; pi picks the file without one.
    if let Some(path) = text
        .strip_prefix("/export")
        .filter(|_| command_name(text) == "/export")
    {
        return Some(match path.trim() {
            "" => json!({ "type": "export_html" }),
            path => json!({ "type": "export_html", "outputPath": path }),
        });
    }
    Some(match text {
        "/new" => json!({ "type": "new_session" }),
        "/resume" => {
            let held: Vec<_> = held_elsewhere
                .iter()
                .map(|file| file.to_string_lossy())
                .collect();
            extension(&format!("resume {}", json!(held)))
        }
        "/model" => json!({ "type": "get_available_models" }),
        "/thinking" => json!({ "type": "get_available_thinking_levels" }),
        "/fork" => json!({ "type": "get_fork_messages" }),
        "/clone" => json!({ "type": "clone" }),
        "/tree" => extension("tree"),
        _ => return None,
    })
}

fn command_name(text: &str) -> &str {
    text.split(char::is_whitespace).next().unwrap_or_default()
}

// What tells the chat which session pi holds now and what it says.
fn session_requests() -> [Value; 3] {
    [
        json!({ "type": "get_state" }),
        json!({ "type": "get_messages" }),
        json!({ "type": "get_session_stats" }),
    ]
}

// The picked option starts with its number in the list; `entries` holds the entry ids in that order.
fn fork_command(entries: &str, answer: &DialogAnswer) -> Option<Value> {
    let number: usize = picked_value(answer)?.split_once('.')?.0.parse().ok()?;
    let entry = entries.split(',').nth(number.checked_sub(1)?)?;
    Some(json!({ "type": "fork", "entryId": entry }))
}

// The option is `provider/model`; a model id may hold slashes, a provider name does not.
fn model_command(answer: &DialogAnswer) -> Option<Value> {
    let (provider, model) = picked_value(answer)?.split_once('/')?;
    Some(json!({ "type": "set_model", "provider": provider, "modelId": model }))
}

// The dialog lists the levels capitalized; pi takes them lowercase.
fn thinking_command(answer: &DialogAnswer) -> Option<Value> {
    Some(json!({ "type": "set_thinking_level", "level": picked_value(answer)?.to_lowercase() }))
}

fn picked_value(answer: &DialogAnswer) -> Option<&str> {
    match answer {
        DialogAnswer::Value { value } => Some(value),
        _ => None,
    }
}

// The model label shows pi's name for the model; the thinking level shows only for a model that reasons.
fn count(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|number| u32::try_from(number).ok())
}

fn chosen_model(state: &Value) -> Option<HarnessEvent> {
    let model = &state["model"];
    let name = model["name"].as_str().or_else(|| model["id"].as_str())?;
    let thinking = if model["reasoning"] == true {
        state["thinkingLevel"].as_str().map(String::from)
    } else {
        None
    };
    Some(HarnessEvent::Model {
        name: name.into(),
        thinking,
    })
}

fn model_dialog(models: &Value) -> HarnessEvent {
    let options = models
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| {
            Some(format!(
                "{}/{}",
                model["provider"].as_str()?,
                model["id"].as_str()?
            ))
        })
        .collect();
    picker_dialog(
        Picker::Model,
        options,
        "No model is available. Log in to a provider in pi",
    )
}

// pi lists only the levels the current model supports, and just "off" for one that does not think.
fn thinking_dialog(levels: &Value) -> HarnessEvent {
    let options = levels
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|level| level.as_str().map(capitalized))
        .collect();
    picker_dialog(Picker::Thinking, options, "The model does not think")
}

// A list with nothing to pick (no option, or only "Off") is a notice instead.
fn picker_dialog(picker: Picker, options: Vec<String>, nothing_to_pick: &str) -> HarnessEvent {
    if options.iter().all(|option| option == "Off") {
        return HarnessEvent::Notice {
            text: nothing_to_pick.into(),
        };
    }
    HarnessEvent::Dialog {
        id: picker.id().into(),
        title: match picker {
            Picker::Model => "Model",
            Picker::Thinking => "Thinking Level",
        }
        .into(),
        ask: DialogAsk::Menu {
            options,
            current: None,
            filter: matches!(picker, Picker::Model),
        },
    }
}

fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
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

const READ_KINDS: [&str; 10] = [
    "extension_ui_request",
    "extension_error",
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

/// What one stdout record leads to: events for the chat and commands to send pi next.
#[derive(Default)]
struct Read {
    events: Vec<HarnessEvent>,
    requests: Vec<Value>,
}

/// The session the chat shows: its file, a hash of its messages, its model and thinking level.
#[derive(Default)]
struct ShownSession {
    file: Option<String>,
    messages: Option<u64>,
    /// The current model as `provider/id` and the current thinking level, so a picker marks them.
    model: Option<String>,
    thinking: Option<String>,
}

impl ShownSession {
    // A cancelled or failed `/resume` or `/tree` still reads the session again; the same file and messages leave the transcript and status alone.
    fn repeats(&mut self, record: &Value) -> bool {
        if record["type"] != "response" || record["success"] != true {
            return false;
        }
        match record["command"].as_str() {
            Some("get_state") => {
                let data = &record["data"];
                self.file = data["sessionFile"].as_str().map(String::from);
                false
            }
            Some("get_messages") => {
                let mut hasher = DefaultHasher::new();
                (&self.file, record["data"]["messages"].to_string()).hash(&mut hasher);
                let messages = Some(hasher.finish());
                std::mem::replace(&mut self.messages, messages) == messages
            }
            _ => false,
        }
    }

    // The state names the model and level a picker marks.
    fn remember_choice(&mut self, record: &Value) {
        if record["type"] != "response"
            || record["success"] != true
            || record["command"] != "get_state"
        {
            return;
        }
        let data = &record["data"];
        self.model = match (
            data["model"]["provider"].as_str(),
            data["model"]["id"].as_str(),
        ) {
            (Some(provider), Some(id)) => Some(format!("{provider}/{id}")),
            _ => None,
        };
        self.thinking = data["thinkingLevel"].as_str().map(capitalized);
    }

    // A picker's list is built from a response that does not know the current choice.
    fn with_current(&self, mut event: HarnessEvent) -> HarnessEvent {
        if let HarnessEvent::Dialog {
            id,
            ask: DialogAsk::Menu { current, .. },
            ..
        } = &mut event
        {
            match Picker::from_id(id) {
                Some(Picker::Model) => *current = self.model.clone(),
                Some(Picker::Thinking) => *current = self.thinking.clone(),
                None => {}
            }
        }
        event
    }
}

// Tool output and turn summaries are large and unused, so only the records `events_from` reads get a full parse.
fn read_line(line: &[u8], shown: &mut ShownSession) -> Read {
    match serde_json::from_slice::<Kind>(line) {
        Ok(Kind { kind, update }) if kind == "message_update" => Read {
            events: message_update(update.as_ref().unwrap_or(&Value::Null)),
            requests: vec![],
        },
        Ok(Kind { kind, .. }) if READ_KINDS.contains(&kind.as_str()) => {
            serde_json::from_slice::<Value>(line)
                .ok()
                .inspect(|record| shown.remember_choice(record))
                .filter(|record| !shown.repeats(record))
                .map(|record| Read {
                    events: events_from(&record)
                        .into_iter()
                        .map(|event| shown.with_current(event))
                        .collect(),
                    requests: requests_after(&record),
                })
                .unwrap_or_default()
        }
        _ => Read::default(),
    }
}

// pi reports a session change only as the response to what caused it.
fn requests_after(record: &Value) -> Vec<Value> {
    let changed = record["success"] == true && record["data"]["cancelled"] != true;
    match (record["type"].as_str(), record["command"].as_str()) {
        (Some("response"), Some("new_session" | "fork" | "clone")) if changed => {
            session_requests().into()
        }
        (Some("response"), Some("prompt")) if changed && record["id"] == SESSION_PROMPT => {
            session_requests().into()
        }
        // A new model can change the thinking level and the context window too.
        (Some("response"), Some("set_model" | "set_thinking_level")) if changed => {
            vec![
                json!({ "type": "get_state" }),
                json!({ "type": "get_session_stats" }),
            ]
        }
        // The settled run is what grew the context.
        (Some("agent_settled"), _) => vec![json!({ "type": "get_session_stats" })],
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
        Some("extension_ui_request") => ui_request(record).into_iter().collect(),
        // A failure in one of Chord's own commands is the answer to what the user asked for.
        Some("extension_error")
            if record["extensionPath"]
                .as_str()
                .is_some_and(|path| path.starts_with(&format!("command:{EXTENSION_PREFIX}"))) =>
        {
            vec![HarnessEvent::Notice {
                text: record["error"]
                    .as_str()
                    .unwrap_or("The session command failed")
                    .into(),
            }]
        }
        Some("response") if record["success"] == false => vec![HarnessEvent::Notice {
            text: record["error"]
                .as_str()
                .unwrap_or("pi refused the command")
                .into(),
        }],
        Some("response") => response(record["command"].as_str(), &record["data"]),
        _ => vec![],
    }
}

fn response(command: Option<&str>, data: &Value) -> Vec<HarnessEvent> {
    match command {
        Some("get_commands") => {
            vec![HarnessEvent::Commands {
                commands: data["commands"]
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
                    .filter(|command| !command.name.starts_with(EXTENSION_PREFIX))
                    .chain(SESSION_COMMANDS.map(|(name, description)| SlashCommand {
                        name: name.into(),
                        description: description.into(),
                        kind: SlashKind::Session,
                        origin: None,
                    }))
                    .collect(),
            }]
        }
        Some("get_state") => data["sessionFile"]
            .as_str()
            .map(|file| HarnessEvent::SessionInfo { file: file.into() })
            .into_iter()
            .chain(chosen_model(data))
            .collect(),
        Some("get_messages") => history(&data["messages"]),
        Some("get_session_stats") => vec![HarnessEvent::Context {
            tokens: count(&data["contextUsage"]["tokens"]),
            window: count(&data["contextUsage"]["contextWindow"]),
        }],
        Some("get_available_models") => vec![model_dialog(&data["models"])],
        Some("get_available_thinking_levels") => vec![thinking_dialog(&data["levels"])],
        Some("get_fork_messages") => vec![fork_dialog(&data["messages"])],
        Some("fork") => data["text"]
            .as_str()
            .map(|text| HarnessEvent::Draft {
                text: collapse_skill(text),
            })
            .into_iter()
            .collect(),
        Some("export_html") => data["path"]
            .as_str()
            .map(|path| HarnessEvent::Exported { path: path.into() })
            .into_iter()
            .collect(),
        _ => vec![],
    }
}

// The session's saved messages as the events their live run produced, between the marks that start the transcript over and end the replay.
fn history(messages: &Value) -> Vec<HarnessEvent> {
    let mut events = vec![HarnessEvent::HistoryStart];
    let mut open_calls = OpenCalls::default();
    for message in messages.as_array().into_iter().flatten() {
        events.extend(open_calls.track(saved_message(message)));
    }
    events.extend(open_calls.stop());
    events.push(HarnessEvent::HistoryEnd);
    events
}

fn saved_message(message: &Value) -> Vec<HarnessEvent> {
    match message["role"].as_str() {
        Some("user") => message_end(message),
        Some("assistant") => {
            let blocks = message["content"].as_array().into_iter().flatten();
            let mut events: Vec<HarnessEvent> = blocks.flat_map(saved_block).collect();
            if message["stopReason"] == "aborted" {
                events.push(HarnessEvent::Aborted);
            }
            events
        }
        Some("toolResult") => vec![HarnessEvent::ToolEnd {
            id: message["toolCallId"].as_str().unwrap_or_default().into(),
            outcome: if message["isError"] == true {
                ToolOutcome::Failed
            } else {
                ToolOutcome::Done
            },
            images: images_of(&message["content"]),
        }],
        _ => vec![],
    }
}

fn saved_block(block: &Value) -> Vec<HarnessEvent> {
    match block["type"].as_str() {
        Some("text") => match block["text"].as_str() {
            Some(text) if !text.is_empty() => vec![
                HarnessEvent::MessageChunk { text: text.into() },
                HarnessEvent::MessageEnd { failed: false },
            ],
            _ => vec![],
        },
        Some("thinking") => vec![
            HarnessEvent::ThoughtStart,
            HarnessEvent::ThoughtChunk {
                text: block["thinking"].as_str().unwrap_or_default().into(),
            },
        ],
        Some("toolCall") => {
            let (title, detail) = describe_tool(
                block["name"].as_str().unwrap_or("tool"),
                &block["arguments"],
            );
            vec![HarnessEvent::ToolStart {
                id: block["id"].as_str().unwrap_or_default().into(),
                title,
                detail,
            }]
        }
        _ => vec![],
    }
}

// One numbered option per user message of the session, oldest first; the number finds the entry again in the answer.
fn fork_dialog(messages: &Value) -> HarnessEvent {
    let messages: Vec<(&str, String)> = messages
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|message| {
            let text = collapse_skill(message["text"].as_str()?);
            Some((message["entryId"].as_str()?, text))
        })
        .collect();
    if messages.is_empty() {
        return HarnessEvent::Notice {
            text: "The session has no message to fork from".into(),
        };
    }
    let entries: Vec<&str> = messages.iter().map(|(entry, _)| *entry).collect();
    HarnessEvent::Dialog {
        id: format!("{FORK_DIALOG}{}", entries.join(",")),
        title: "Fork From".into(),
        ask: DialogAsk::Select {
            options: messages
                .iter()
                .enumerate()
                .map(|(i, (_, text))| format!("{}. {}", i + 1, first_line(text)))
                .collect(),
            current: None,
        },
    }
}

fn first_line(text: &str) -> String {
    let line = text.lines().find(|line| !line.trim().is_empty());
    line.unwrap_or_default().trim().chars().take(80).collect()
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

// Select, confirm and input wait for an answer; the editor text goes in the composer; the usage status is read; notify and the other status methods are not.
fn ui_request(record: &Value) -> Option<HarnessEvent> {
    let text = |key: &str| record[key].as_str().unwrap_or_default().to_string();
    let ask = match record["method"].as_str()? {
        "setStatus" if record["statusKey"] == USAGE_STATUS => {
            return Some(HarnessEvent::Usage {
                windows: usage_windows(record["statusText"].as_str().unwrap_or("[]")),
            });
        }
        "set_editor_text" => return Some(HarnessEvent::Draft { text: text("text") }),
        "select" => DialogAsk::Select {
            options: record["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|option| option.as_str().map(String::from))
                .collect(),
            current: None,
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

// The extension writes the windows as JSON; text that does not parse shows no usage.
fn usage_windows(text: &str) -> Vec<UsageWindow> {
    let windows: Vec<Value> = serde_json::from_str(text).unwrap_or_default();
    windows
        .iter()
        .filter_map(|window| {
            Some(UsageWindow {
                label: window["label"].as_str()?.into(),
                resets_at: window["resetsAt"].as_f64(),
            })
        })
        .collect()
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
        read_line(record.as_bytes(), &mut ShownSession::default()).events
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
        let record = r#"{"type":"response","command":"get_commands","success":true,"data":{"commands":[{"name":"skill:deploy","description":"Ship it","source":"skill"},{"name":"compact","source":"extension"},{"name":"chord-resume","source":"extension"},{"name":"review","source":"prompt","sourceInfo":{"source":"npm:@acme/pi-tools@1.2.0","origin":"package"}}]}}"#;
        let HarnessEvent::Commands { commands } = &events(record)[0] else {
            panic!("no command list");
        };
        let from_pi: Vec<&SlashCommand> = commands
            .iter()
            .filter(|command| command.kind != SlashKind::Session)
            .collect();
        assert_eq!(
            from_pi,
            [
                &SlashCommand {
                    name: "skill:deploy".into(),
                    description: "Ship it".into(),
                    kind: SlashKind::Skill,
                    origin: None
                },
                &SlashCommand {
                    name: "compact".into(),
                    description: "".into(),
                    kind: SlashKind::Extension,
                    origin: None
                },
                &SlashCommand {
                    name: "review".into(),
                    description: "".into(),
                    kind: SlashKind::Prompt,
                    origin: Some("pi-tools".into())
                }
            ]
        );
    }

    #[test]
    fn saved_messages_come_back_as_the_events_of_their_run() {
        let record = r#"{"type":"response","command":"get_messages","success":true,"data":{"messages":[{"role":"user","content":"List it"},{"role":"assistant","content":[{"type":"thinking","thinking":"hm"},{"type":"text","text":"Sure."},{"type":"toolCall","id":"c1","name":"ls","arguments":{"path":"src"}},{"type":"toolCall","id":"c2","name":"read","arguments":{"path":"a.rs"}}],"stopReason":"toolUse"},{"role":"toolResult","toolCallId":"c1","content":[],"isError":true},{"role":"assistant","content":[],"stopReason":"aborted"}]}}"#;
        assert_eq!(
            events(record),
            vec![
                HarnessEvent::HistoryStart,
                HarnessEvent::UserMessageChunk {
                    text: "List it".into(),
                    images: vec![]
                },
                HarnessEvent::ThoughtStart,
                HarnessEvent::ThoughtChunk { text: "hm".into() },
                HarnessEvent::MessageChunk {
                    text: "Sure.".into()
                },
                HarnessEvent::MessageEnd { failed: false },
                HarnessEvent::ToolStart {
                    id: "c1".into(),
                    title: "List".into(),
                    detail: "src".into()
                },
                HarnessEvent::ToolStart {
                    id: "c2".into(),
                    title: "Read".into(),
                    detail: "a.rs".into()
                },
                HarnessEvent::ToolEnd {
                    id: "c1".into(),
                    outcome: ToolOutcome::Failed,
                    images: vec![]
                },
                HarnessEvent::Aborted,
                HarnessEvent::ToolEnd {
                    id: "c2".into(),
                    outcome: ToolOutcome::Stopped,
                    images: vec![]
                },
                HarnessEvent::HistoryEnd,
            ]
        );
    }

    #[test]
    fn the_state_tells_the_session_file() {
        let record = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"sessions/a.jsonl","sessionName":"Login fix"}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::SessionInfo {
                file: "sessions/a.jsonl".into()
            }]
        );
    }

    #[test]
    fn a_session_command_runs_instead_of_being_sent() {
        assert_eq!(
            session_command("/new", &[]),
            Some(json!({ "type": "new_session" }))
        );
        assert_eq!(
            session_command("/tree", &[]),
            Some(
                json!({ "id": "chord-session", "type": "prompt", "message": "/chord-tree" })
            )
        );
        assert_eq!(session_command("/new feature", &[]), None);
    }

    #[test]
    fn export_passes_the_path_it_was_given() {
        assert_eq!(
            session_command("/export", &[]),
            Some(json!({ "type": "export_html" }))
        );
        assert_eq!(
            session_command("/export  out/login.html", &[]),
            Some(json!({ "type": "export_html", "outputPath": "out/login.html" }))
        );
        assert_eq!(session_command("/exports", &[]), None);
    }

    #[test]
    fn resume_leaves_out_the_sessions_other_panes_hold() {
        assert_eq!(
            session_command("/resume", &[PathBuf::from("sessions/b.jsonl")]),
            Some(
                json!({ "id": "chord-session", "type": "prompt", "message": r#"/chord-resume ["sessions/b.jsonl"]"# })
            )
        );
    }

    #[test]
    fn a_reload_that_finds_the_shown_session_changes_nothing() {
        let mut shown = ShownSession::default();
        let mut read = |record: &str| read_line(record.as_bytes(), &mut shown).events;
        let state_a = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"a.jsonl"}}"#;
        let state_b = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"b.jsonl"}}"#;
        let messages =
            r#"{"type":"response","command":"get_messages","success":true,"data":{"messages":[]}}"#;
        let history = vec![HarnessEvent::HistoryStart, HarnessEvent::HistoryEnd];
        read(state_a);
        assert_eq!(read(messages), history);
        read(state_a);
        assert_eq!(read(messages), vec![]);
        read(state_b);
        assert_eq!(read(messages), history);
    }

    #[test]
    fn a_changed_session_is_read_again_and_a_cancelled_change_is_not() {
        let read =
            |record: &str| read_line(record.as_bytes(), &mut ShownSession::default()).requests;
        let again = vec![
            json!({ "type": "get_state" }),
            json!({ "type": "get_messages" }),
            json!({ "type": "get_session_stats" }),
        ];
        assert_eq!(
            read(
                r#"{"type":"response","command":"clone","success":true,"data":{"cancelled":false}}"#
            ),
            again
        );
        assert_eq!(
            read(
                r#"{"id":"chord-session","type":"response","command":"prompt","success":true,"data":{"disposition":"handled"}}"#
            ),
            again
        );
        assert_eq!(
            read(
                r#"{"type":"response","command":"new_session","success":true,"data":{"cancelled":true}}"#
            ),
            Vec::<Value>::new()
        );
        assert_eq!(
            read(
                r#"{"type":"response","command":"prompt","success":true,"data":{"disposition":"started"}}"#
            ),
            Vec::<Value>::new()
        );
    }

    #[test]
    fn the_model_list_becomes_a_dialog_and_the_pick_sets_the_model() {
        let record = r#"{"type":"response","command":"get_available_models","success":true,"data":{"models":[{"id":"sonnet-4","provider":"anthropic"},{"id":"openai/gpt-5","provider":"openrouter"}]}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::Dialog {
                id: "chord-model".into(),
                title: "Model".into(),
                ask: DialogAsk::Menu {
                    options: vec![
                        "anthropic/sonnet-4".into(),
                        "openrouter/openai/gpt-5".into()
                    ],
                    current: None,
                    filter: true
                },
            }]
        );
        let picked = DialogAnswer::Value {
            value: "openrouter/openai/gpt-5".into(),
        };
        assert_eq!(
            model_command(&picked),
            Some(
                json!({ "type": "set_model", "provider": "openrouter", "modelId": "openai/gpt-5" })
            )
        );
        assert_eq!(model_command(&DialogAnswer::Cancelled), None);
    }

    #[test]
    fn the_thinking_levels_open_for_a_model_that_thinks_and_the_pick_sets_the_level() {
        let thinks = r#"{"type":"response","command":"get_available_thinking_levels","success":true,"data":{"levels":["off","low","high"]}}"#;
        assert_eq!(
            events(thinks),
            vec![HarnessEvent::Dialog {
                id: "chord-thinking".into(),
                title: "Thinking Level".into(),
                ask: DialogAsk::Menu {
                    options: ["Off", "Low", "High"].map(String::from).into(),
                    current: None,
                    filter: false
                },
            }]
        );
        let plain = r#"{"type":"response","command":"get_available_thinking_levels","success":true,"data":{"levels":["off"]}}"#;
        assert_eq!(
            events(plain),
            vec![HarnessEvent::Notice {
                text: "The model does not think".into()
            }]
        );
        let picked = DialogAnswer::Value {
            value: "High".into(),
        };
        assert_eq!(
            thinking_command(&picked),
            Some(json!({ "type": "set_thinking_level", "level": "high" }))
        );
    }

    #[test]
    fn the_state_tells_the_model_and_a_picker_marks_it() {
        let mut shown = ShownSession::default();
        let state = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"a.jsonl","thinkingLevel":"medium","model":{"id":"sonnet-4","provider":"anthropic","name":"Sonnet 4","reasoning":true}}}"#;
        assert_eq!(
            read_line(state.as_bytes(), &mut shown).events,
            vec![
                HarnessEvent::SessionInfo {
                    file: "a.jsonl".into()
                },
                HarnessEvent::Model {
                    name: "Sonnet 4".into(),
                    thinking: Some("medium".into())
                },
            ]
        );
        let plain = r#"{"type":"response","command":"get_state","success":true,"data":{"thinkingLevel":"off","model":{"id":"small","provider":"local","reasoning":false}}}"#;
        assert_eq!(
            events(plain),
            vec![HarnessEvent::Model {
                name: "small".into(),
                thinking: None
            }]
        );
        read_line(state.as_bytes(), &mut shown);
        let models = r#"{"type":"response","command":"get_available_models","success":true,"data":{"models":[{"id":"sonnet-4","provider":"anthropic"}]}}"#;
        let levels = r#"{"type":"response","command":"get_available_thinking_levels","success":true,"data":{"levels":["off","medium"]}}"#;
        let mut current = |record: &str| match read_line(record.as_bytes(), &mut shown).events.pop()
        {
            Some(HarnessEvent::Dialog {
                ask: DialogAsk::Menu { current, .. },
                ..
            }) => current,
            other => panic!("not a menu: {other:?}"),
        };
        assert_eq!(current(models), Some("anthropic/sonnet-4".into()));
        assert_eq!(current(levels), Some("Medium".into()));
    }

    #[test]
    fn a_usage_status_becomes_the_usage_windows() {
        let status = r#"{"type":"extension_ui_request","id":"u1","method":"setStatus","statusKey":"chord-usage","statusText":"[{\"label\":\"5h 42%\",\"resetsAt\":1700000000000},{\"label\":\"week 10%\",\"resetsAt\":null}]"}"#;
        assert_eq!(
            events(status),
            vec![HarnessEvent::Usage {
                windows: vec![
                    UsageWindow {
                        label: "5h 42%".into(),
                        resets_at: Some(1700000000000.0)
                    },
                    UsageWindow {
                        label: "week 10%".into(),
                        resets_at: None
                    }
                ]
            }]
        );
        let cleared = r#"{"type":"extension_ui_request","id":"u2","method":"setStatus","statusKey":"chord-usage"}"#;
        assert_eq!(
            events(cleared),
            vec![HarnessEvent::Usage { windows: vec![] }]
        );
        let other = r#"{"type":"extension_ui_request","id":"u3","method":"setStatus","statusKey":"other","statusText":"x"}"#;
        assert_eq!(events(other), vec![]);
    }

    #[test]
    fn the_session_stats_tell_the_context_fill() {
        let stats = r#"{"type":"response","command":"get_session_stats","success":true,"data":{"contextUsage":{"tokens":60000,"contextWindow":200000,"percent":30}}}"#;
        assert_eq!(
            events(stats),
            vec![HarnessEvent::Context {
                tokens: Some(60000),
                window: Some(200000)
            }]
        );
        let no_window = r#"{"type":"response","command":"get_session_stats","success":true,"data":{"contextUsage":{"tokens":null,"contextWindow":null}}}"#;
        assert_eq!(
            events(no_window),
            vec![HarnessEvent::Context {
                tokens: None,
                window: None
            }]
        );
        let compacted = r#"{"type":"response","command":"get_session_stats","success":true,"data":{"contextUsage":{"tokens":null,"contextWindow":200000,"percent":null}}}"#;
        assert_eq!(
            events(compacted),
            vec![HarnessEvent::Context {
                tokens: None,
                window: Some(200000)
            }]
        );
    }

    #[test]
    fn a_settled_run_reads_the_stats_again() {
        let settled = r#"{"type":"agent_settled","aborted":false}"#;
        assert_eq!(
            read_line(settled.as_bytes(), &mut ShownSession::default()).requests,
            vec![json!({ "type": "get_session_stats" })]
        );
    }

    #[test]
    fn a_pick_reads_the_state_again() {
        let set = r#"{"type":"response","command":"set_model","success":true,"data":{}}"#;
        let refused = r#"{"type":"response","command":"set_thinking_level","success":false}"#;
        assert_eq!(
            read_line(set.as_bytes(), &mut ShownSession::default()).requests,
            vec![
                json!({ "type": "get_state" }),
                json!({ "type": "get_session_stats" })
            ]
        );
        assert!(
            read_line(refused.as_bytes(), &mut ShownSession::default())
                .requests
                .is_empty()
        );
    }

    #[test]
    fn a_fork_asks_which_message_and_forks_from_the_picked_one() {
        let record = r#"{"type":"response","command":"get_fork_messages","success":true,"data":{"messages":[{"entryId":"a1","text":"Fix the login\nand the logout"},{"entryId":"b2","text":"Fix the login"}]}}"#;
        assert_eq!(
            events(record),
            vec![HarnessEvent::Dialog {
                id: "chord-fork:a1,b2".into(),
                title: "Fork From".into(),
                ask: DialogAsk::Select {
                    options: vec!["1. Fix the login".into(), "2. Fix the login".into()],
                    current: None
                },
            }]
        );
        let picked = DialogAnswer::Value {
            value: "2. Fix the login".into(),
        };
        assert_eq!(
            fork_command("a1,b2", &picked),
            Some(json!({ "type": "fork", "entryId": "b2" }))
        );
        assert_eq!(fork_command("a1,b2", &DialogAnswer::Cancelled), None);
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
                    options: vec!["a".into()],
                    current: None
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
