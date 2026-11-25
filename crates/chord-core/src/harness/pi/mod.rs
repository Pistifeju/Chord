//! The pi adapter: runs `pi --mode rpc` as a process and hands its stdout to the reader. The only module that knows pi's records.

mod commands;
mod reader;
mod records;
pub(super) mod spend;
#[cfg(test)]
mod tests;

use crate::lock;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

use super::installed::require;
use super::models::{self, Step, listed};
use super::process::{Output, Process, login_shell};
use super::session_commands::{CLONE, COMPACT, Entry, FORK, MODEL, NEW, RESUME, THINKING};
use super::{Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, ModelOption};
use crate::Error;
use commands::{
    command_name, dialog_response, fork_command, model_command, prompt_command, session_command,
    session_requests, thinking_command,
};
use reader::{Read, Reader};
use records::{models_event, newest_first};

/// The harness name a pane saves.
pub const NAME: &str = "pi";

// `pi auth check` needs a provider, so pi has no sign-in check.
pub const HARNESS: Harness = Harness {
    name: NAME,
    title: "pi",
    binary: "pi",
    install: "curl -fsSL https://pi.dev/install.sh | sh",
    sign_in: None,
    upgrade: "pi update",
    oldest_tested: (1, 5, 0),
    starts: true,
};

/// The extension Chord ships: the session commands RPC lacks, the subscription usage windows, and the build and run tools.
const EXTENSION: &str = include_str!("extension.ts");
const EXTENSION_FILE: &str = "chord-extension.ts";
/// The commands the extension adds start with this; the slash menu lists Chord's own names for them.
const EXTENSION_PREFIX: &str = "chord-";
/// The status key the extension reports the usage windows under.
const USAGE_STATUS: &str = "chord-usage";
/// The extension registers the build and run tools when pi starts with these: the socket Chord takes the calls on, the worktree they build, and the tools' texts.
const TOOLS_SOCKET_ENV: &str = "CHORD_TOOLS";
const WORKTREE_ENV: &str = "CHORD_WORKTREE";
const TOOL_TEXTS_ENV: &str = "CHORD_TOOL_TEXTS";
/// A subagent extension's widget carries its background runs as one line with this prefix, then a JSON snapshot.
const SUBAGENTS_PREFIX: &str = "PI_SUBAGENT_ASYNC_JSON:";
/// The request id of a prompt that runs one of the extension's commands; its response means the session may have changed.
const SESSION_PROMPT: &str = "chord-session";
/// The request id of the prompt that runs `/chord-reload`; its response is the one thing that tells the user the reload happened.
const RELOAD_PROMPT: &str = "chord-reload";
/// A fork dialog's id starts with this; the ids of the entries it lists follow, comma-separated, in the order of its options.
const FORK_DIALOG: &str = "chord-fork:";
/// The id of the model dialog; its options are `provider/model`.
const MODEL_DIALOG: &str = "chord-model";
/// The id of the thinking level dialog; its options are the levels, capitalized.
const THINKING_DIALOG: &str = "chord-thinking";
/// The request id of the model list the agent picker shows, so its response is no model dialog.
const MODELS_REQUEST: &str = "chord-models";
/// A `set_model` request id starts with this when the pick carries a level, which follows it; pi runs requests side by side, and a model switch resets the level once it lands.
const PICKED_LEVEL: &str = "chord-level:";

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
const SESSION_COMMANDS: [Entry; 10] = [
    MODEL,
    THINKING,
    NEW,
    RESUME,
    FORK,
    CLONE,
    ("tree", "Go back to an earlier point of this session"),
    (
        "reload",
        "Reload extensions, skills, prompts and context files",
    ),
    COMPACT,
    (
        "export",
        "Write this session to an HTML file, optionally at a path",
    ),
];

/// pi's models, read from a pi that saves no session.
pub(crate) fn models() -> Result<Vec<ModelOption>, Error> {
    let request = json!({ "id": MODELS_REQUEST, "type": "get_available_models" });
    let mut models = models::ask(
        &HARNESS,
        &["--mode", "rpc", "--no-session"],
        &[request],
        |record| {
            if record["id"] != MODELS_REQUEST {
                return Step::Wait;
            }
            listed(models_event(&record["data"]["models"]))
        },
    )?;
    newest_first(&mut models, None, |model| &model.id);
    Ok(models)
}

/// One running `pi --mode rpc` process.
pub struct Session {
    process: Mutex<Process>,
    running: Arc<AtomicBool>,
    /// The session file pi holds, from its last `SessionInfo`.
    file: Arc<Mutex<Option<String>>>,
    on_event: Arc<Mutex<dyn Fn(HarnessEvent) + Send>>,
    /// Starts another pi with the same setup on the given session file, or on a new session without one.
    launch: Box<Launch>,
}

type Launch = dyn Fn(Option<&str>) -> Result<Process, Error> + Send;

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
        if let Some(text) = require(&HARNESS)? {
            on_event(HarnessEvent::Notice { text });
        }
        let extension = installed_extension(extension_dir)?;
        let extension = extension.to_string_lossy().into_owned();
        let running = Arc::new(AtomicBool::new(false));
        let file = Arc::new(Mutex::new(None));
        let on_event = Arc::new(Mutex::new(on_event));
        let launch = {
            let (dir, tools) = (dir.to_path_buf(), tools.map(Path::to_path_buf));
            let (running, file, on_event) = (
                Arc::clone(&running),
                Arc::clone(&file),
                Arc::clone(&on_event),
            );
            move |session: Option<&str>| {
                launch(
                    &dir,
                    &extension,
                    session,
                    tools.as_deref(),
                    &running,
                    &file,
                    &on_event,
                )
            }
        };
        let process = launch(session)?;
        let session = Session {
            process: Mutex::new(process),
            running,
            file,
            on_event,
            launch: Box::new(launch),
        };
        session.read_session();
        Ok(session)
    }

    // pi's own `/new` first waits for the run to settle, which a hung tool never does, and pi refuses `/reload` mid-run; a new pi on `session`, or on a new session, starts at once and the old one is killed.
    fn restart(&self, session: Option<&str>) -> Result<(), Error> {
        let new = (self.launch)(session)?;
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
        let _ = self.write(&json!({ "id": MODELS_REQUEST, "type": "get_available_models" }));
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
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<bool, Error> {
        let Some(command) = session_command(text.trim(), held_elsewhere) else {
            return self.write(&prompt_command(text, images)).map(|()| true);
        };
        // A new session mid-run replaces pi; pi aborts the run to open another session; `/tree` mid-run it refuses itself and `/export` leaves the run alone; `/model` and `/thinking` only open a picker, and pi's refusal of the pick shows as a notice.
        let keeps_run = matches!(
            command_name(text.trim()),
            "/tree" | "/export" | "/model" | "/thinking"
        );
        if self.running.load(Ordering::SeqCst) {
            let file = lock(&self.file).clone();
            match (command_name(text.trim()), file) {
                ("/new", _) => return self.restart(None).map(|()| false),
                ("/reload", Some(file)) => return self.restart(Some(&file)).map(|()| false),
                _ if !keeps_run => {
                    return Err(Error::Message(
                        "Wait for the agent to finish, then change the session".into(),
                    ));
                }
                _ => {}
            }
        }
        self.write(&command).map(|()| false)
    }

    fn abort(&self) -> Result<(), Error> {
        self.write(&json!({ "type": "abort" }))
    }

    /// `id` is `provider/model`; the reader sends a picked level once pi answers the model switch.
    fn pick_model(&self, id: &str, level: Option<&str>) -> Result<(), Error> {
        let (provider, model) = id
            .split_once('/')
            .ok_or_else(|| Error::Message(format!("pi has no model \"{id}\"")))?;
        let mut command = json!({ "type": "set_model", "provider": provider, "modelId": model });
        if let Some(level) = level {
            command["id"] = format!("{PICKED_LEVEL}{level}").into();
        }
        self.write(&command)
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
    file: &Arc<Mutex<Option<String>>>,
    on_event: &Arc<Mutex<impl Fn(HarnessEvent) + Send + 'static>>,
) -> Result<Process, Error> {
    let mut args = vec!["--mode", "rpc", "--extension", extension];
    // The same file on every start puts the pane back in its exact session.
    if let Some(file) = session.filter(|file| Path::new(file).is_file()) {
        args.extend(["--session", file]);
    }
    let reader_running = Arc::clone(running);
    let file = Arc::clone(file);
    let on_event = Arc::clone(on_event);
    let emit = move |event| (lock(&on_event))(event);
    let mut records = Reader::default();
    let mut command = login_shell(HARNESS.binary, &args);
    command.current_dir(dir);
    if let Some(socket) = tools {
        command
            .env(TOOLS_SOCKET_ENV, socket)
            .env(WORKTREE_ENV, dir)
            .env(TOOL_TEXTS_ENV, super::agent_tools::TEXTS);
    }
    Process::start(command, move |output, stdin| match output {
        Output::Line(line) => {
            let Read { events, requests } = records.feed(line);
            for event in events {
                match &event {
                    HarnessEvent::State { running } => {
                        reader_running.store(*running, Ordering::SeqCst);
                    }
                    HarnessEvent::SessionInfo { id } => *lock(&file) = Some(id.clone()),
                    _ => {}
                }
                emit(event);
            }
            // A `/tree` pi refused mid-run left the session as it was, so reading it again would restart the live transcript; only the state and the stats, which are read-only, go out mid-run.
            let running = reader_running.load(Ordering::SeqCst);
            for request in requests {
                if !running
                    || matches!(
                        request["type"].as_str(),
                        Some("get_state" | "get_session_stats" | "set_thinking_level")
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
    Error::start_failed(HARNESS.binary, HARNESS.install, error)
}
