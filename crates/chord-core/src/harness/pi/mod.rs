//! The pi adapter: runs `pi --mode rpc` as a process and hands its stdout to the reader. The only module that knows pi's records.

mod commands;
mod reader;
mod records;
#[cfg(test)]
mod tests;

use crate::lock;
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
/// The binary Chord runs, found on the login shell's PATH.
const BINARY: &str = "pi";
const INSTALL: &str = "npm install -g @earendil-works/pi-coding-agent";
const OLDEST_TESTED: (u32, u32, u32) = (1, 5, 0);

/// The extension Chord ships: the session commands RPC lacks, the subscription usage windows, and the build and run tools.
const EXTENSION: &str = include_str!("extension.ts");
const EXTENSION_FILE: &str = "chord-extension.ts";
/// The commands the extension adds start with this; the slash menu lists Chord's own names for them.
const EXTENSION_PREFIX: &str = "chord-";
/// The status key the extension reports the usage windows under.
const USAGE_STATUS: &str = "chord-usage";
/// The extension registers the build and run tools when pi starts with these: the socket Chord takes the calls on, and the worktree they build.
const TOOLS_SOCKET_ENV: &str = "CHORD_TOOLS";
const WORKTREE_ENV: &str = "CHORD_WORKTREE";
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
    process: Mutex<Process>,
    running: Arc<AtomicBool>,
    on_event: Arc<Mutex<dyn Fn(HarnessEvent) + Send>>,
    /// Starts another pi with the same setup on a new session.
    launch_new: Box<dyn Fn() -> Result<Process, Error> + Send>,
}

static EXTENSION_WRITE: Mutex<()> = Mutex::new(());

// Written only when it differs, and one start at a time, so no pi reads a half-written file.
fn installed_extension(dir: &Path) -> Result<PathBuf, Error> {
    let _writing = lock(&EXTENSION_WRITE);
    std::fs::create_dir_all(dir)?;
    let file = dir.join(EXTENSION_FILE);
    if std::fs::read_to_string(&file).ok().as_deref() != Some(EXTENSION) {
        std::fs::write(&file, EXTENSION)?;
    }
    Ok(file)
}

impl Session {
    /// `extension_dir` is where the extension Chord ships is written for pi to load. `session` is a pi session file; without an existing one pi starts a new session in its own session folder. With `tools`, the socket Chord's tools listen on, the agent gets the build and run tools.
    pub fn start(
        dir: &Path,
        session: Option<&str>,
        extension_dir: &Path,
        tools: Option<&Path>,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        check_version()?;
        let extension = installed_extension(extension_dir)?;
        let extension = extension.to_string_lossy().into_owned();
        let running = Arc::new(AtomicBool::new(false));
        let on_event = Arc::new(Mutex::new(on_event));
        let launch = {
            let (dir, tools) = (dir.to_path_buf(), tools.map(Path::to_path_buf));
            let (running, on_event) = (Arc::clone(&running), Arc::clone(&on_event));
            move |session: Option<&str>| {
                launch(
                    &dir,
                    &extension,
                    session,
                    tools.as_deref(),
                    &running,
                    &on_event,
                )
            }
        };
        let process = launch(session)?;
        let session = Session {
            process: Mutex::new(process),
            running,
            on_event,
            launch_new: Box::new(move || launch(None)),
        };
        session.read_session();
        Ok(session)
    }

    // pi's own `/new` first waits for the run to settle, which a hung tool never does; a new pi starts at once and the old one is killed.
    fn restart(&self) -> Result<(), Error> {
        let new = (self.launch_new)()?;
        let old = std::mem::replace(&mut *lock(&self.process), new);
        drop(old);
        self.running.store(false, Ordering::SeqCst);
        (lock(&self.on_event))(HarnessEvent::State { running: false });
        self.read_session();
        Ok(())
    }

    fn read_session(&self) {
        // Without the list the slash menu stays hidden; the agent works without it.
        let _ = self.write(&json!({ "type": "get_commands" }));
        // The session file may hold earlier messages; they come back as history.
        for request in session_requests() {
            let _ = self.write(&request);
        }
    }

    fn write_if_picked(&self, command: Option<Value>) -> Result<(), Error> {
        command.map_or(Ok(()), |command| self.write(&command))
    }

    fn write(&self, command: &Value) -> Result<(), Error> {
        lock(&self.process).write(command)
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
        // A new session mid-run replaces pi; pi aborts the run to open another session; `/tree` mid-run it refuses itself and `/export` leaves the run alone; `/model` and `/thinking` only open a picker, and pi's refusal of the pick shows as a notice.
        let keeps_run = matches!(
            command_name(text.trim()),
            "/tree" | "/export" | "/model" | "/thinking"
        );
        if !images.is_empty() {
            return Err(Error::Message(
                "Remove the images, then change the session".into(),
            ));
        }
        let running = self.running.load(Ordering::SeqCst);
        if running && command_name(text.trim()) == "/new" {
            return self.restart();
        }
        if running && !keeps_run {
            return Err(Error::Message(
                "Wait for the agent to finish, then change the session".into(),
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
        process
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .end_reported();
    }
}

// One pi process on `session`, or on a new session without one; its reader keeps `running` and hands each event to `on_event`.
fn launch(
    dir: &Path,
    extension: &str,
    session: Option<&str>,
    tools: Option<&Path>,
    running: &Arc<AtomicBool>,
    on_event: &Arc<Mutex<impl Fn(HarnessEvent) + Send + 'static>>,
) -> Result<Process, Error> {
    let mut args = vec!["--mode", "rpc", "--extension", extension];
    // The same file on every start puts the pane back in its exact session.
    if let Some(file) = session.filter(|file| Path::new(file).is_file()) {
        args.extend(["--session", file]);
    }
    let reader_running = Arc::clone(running);
    let on_event = Arc::clone(on_event);
    let emit = move |event| (lock(&on_event))(event);
    let mut records = Reader::default();
    let mut command = login_shell(BINARY, &args);
    command.current_dir(dir);
    if let Some(socket) = tools {
        command.env(TOOLS_SOCKET_ENV, socket).env(WORKTREE_ENV, dir);
    }
    Process::start(command, move |output, stdin| match output {
        Output::Line(line) => {
            let Read { events, requests } = records.feed(line);
            for event in events {
                if let HarnessEvent::State { running } = event {
                    reader_running.store(running, Ordering::SeqCst);
                }
                emit(event);
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
            records.end().into_iter().for_each(&emit);
            if let Some(code) = exited {
                emit(HarnessEvent::Exited { code });
            }
        }
    })
    .map_err(spawn_error)
}

fn spawn_error(error: std::io::Error) -> Error {
    Error::start_failed(BINARY, INSTALL, error)
}

fn missing_pi() -> Error {
    Error::MissingTool {
        tool: BINARY.into(),
        install: INSTALL.into(),
    }
}

static CHECKED: Mutex<bool> = Mutex::new(false);

// A pi that passed is not asked again while the app runs: each ask costs a login shell and a pi start.
fn check_version() -> Result<(), Error> {
    let mut checked = lock(&CHECKED);
    if *checked {
        return Ok(());
    }
    let output = login_shell(BINARY, &["--version"])
        .output()
        .map_err(spawn_error)?;
    if output.status.code() == Some(127) {
        return Err(missing_pi());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text.lines().rev().find_map(parse_version);
    match version {
        Some(found) if found < OLDEST_TESTED => Err(Error::Message(format!(
            "pi {}.{}.{} is older than the tested {}.{}.{}. Upgrade it with: {INSTALL}",
            found.0, found.1, found.2, OLDEST_TESTED.0, OLDEST_TESTED.1, OLDEST_TESTED.2
        ))),
        _ => {
            *checked = true;
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
