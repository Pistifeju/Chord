//! The pi adapter: runs `pi --mode rpc` as a process and hands its stdout to the reader. The only module that knows pi's records.

mod commands;
mod reader;
mod records;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

use super::process::{Output, Process, login_shell};
use super::{Agent, Capability, DialogAnswer, HarnessEvent, Image};
use crate::Error;
use commands::{
    command_name, dialog_response, fork_command, model_command, prompt_command, session_command,
    session_requests, thinking_command,
};
use reader::{Read, Reader};

/// The harness name a pane saves.
pub const NAME: &str = "pi";
/// The binary to run when the user chose none.
pub const DEFAULT_BINARY: &str = "pi";
const INSTALL: &str = "npm install -g @earendil-works/pi-coding-agent";
const OLDEST_TESTED: (u32, u32, u32) = (1, 5, 0);

/// The extension Chord ships: the session commands RPC lacks, and the subscription usage windows.
const EXTENSION: &str = include_str!("extension.ts");
const EXTENSION_FILE: &str = "chord-extension.ts";
/// The commands the extension adds start with this; the slash menu lists Chord's own names for them.
const EXTENSION_PREFIX: &str = "chord-";
/// The status key the extension reports the usage windows under.
const USAGE_STATUS: &str = "chord-usage";
/// A subagent extension's widget carries its background runs as one line with this prefix, then a JSON snapshot.
const SUBAGENTS_PREFIX: &str = "PI_SUBAGENT_ASYNC_JSON:";
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
const SESSION_COMMANDS: [(&str, &str); 10] = [
    ("model", "Pick the model"),
    ("thinking", "Pick the thinking level"),
    ("new", "Start a new session"),
    ("resume", "Open another session of this worktree"),
    ("fork", "Start a new session from an earlier message"),
    ("clone", "Copy this session into a new one"),
    ("tree", "Go back to an earlier point of this session"),
    (
        "reload",
        "Reload extensions, skills, prompts and context files",
    ),
    (
        "compact",
        "Summarize the session to free the context, optionally with what to keep",
    ),
    (
        "export",
        "Write this session to an HTML file, optionally at a path",
    ),
];

/// One running `pi --mode rpc` process.
pub struct Session {
    process: Process,
    running: Arc<AtomicBool>,
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
    /// `extension_dir` is where the extension Chord ships is written for pi to load. `session` is a pi session file; without an existing one pi starts a new session in its own session folder.
    pub fn start(
        binary: &str,
        dir: &Path,
        session: Option<&str>,
        extension_dir: &Path,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        check_version(binary)?;
        let extension = installed_extension(extension_dir)?;
        let extension = extension.to_string_lossy();
        let mut args = vec!["--mode", "rpc", "--extension", &extension];
        // The same file on every start puts the pane back in its exact session.
        if let Some(file) = session.filter(|file| Path::new(file).is_file()) {
            args.extend(["--session", file]);
        }
        let running = Arc::new(AtomicBool::new(false));
        let reader_running = Arc::clone(&running);
        let mut records = Reader::default();
        let process = Process::start(binary, &args, dir, move |output, stdin| match output {
            Output::Line(line) => {
                let Read { events, requests } = records.feed(line);
                for event in events {
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
                        let _ = stdin.write(&request);
                    }
                }
            }
            Output::Closed { exited } => {
                records.end().into_iter().for_each(&on_event);
                if let Some(code) = exited {
                    on_event(HarnessEvent::Exited { code });
                }
            }
        })
        .map_err(|e| spawn_error(binary, e))?;
        let session = Session { process, running };
        // Without the list the slash menu stays hidden; the agent works without it.
        let _ = session.write(&json!({ "type": "get_commands" }));
        // The session file may hold earlier messages; they come back as history.
        for request in session_requests() {
            let _ = session.write(&request);
        }
        Ok(session)
    }

    fn write_if_picked(&self, command: Option<Value>) -> Result<(), Error> {
        command.map_or(Ok(()), |command| self.write(&command))
    }

    fn write(&self, command: &Value) -> Result<(), Error> {
        self.process.write(command)
    }
}

impl Agent for Session {
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Chat,
            Capability::Attachments,
            Capability::Steer,
            Capability::Models,
            Capability::Context,
            Capability::Subagents,
        ]
    }

    /// Sends a message; while the agent runs, pi queues it as a steer. A session command runs instead of being sent; `/resume` leaves out `held_elsewhere`, the session files other panes hold, so no two agents write one file.
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<(), Error> {
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

    fn abort(&self) -> Result<(), Error> {
        self.write(&json!({ "type": "abort" }))
    }

    /// Answers the dialog pi asked with `id`; a fork dialog is the adapter's own, so its answer becomes the fork.
    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
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

    fn end_reported(self: Box<Self>) {
        let Session { process, .. } = *self;
        process.end_reported();
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
    let output = login_shell(binary, &["--version"])
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
