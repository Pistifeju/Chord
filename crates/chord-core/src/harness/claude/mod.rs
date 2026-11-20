//! The Claude Code adapter: runs `claude -p` with stream-json in and out as a process and hands its stdout to the reader. The only module that knows Claude Code's records.

mod asks;
mod blocks;
mod commands;
mod permissions;
mod reader;
mod requests;
#[cfg(test)]
mod tests;
mod transcript;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;

use super::installed::require;
use super::process::{Output, Process, login_shell};
use super::{Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, OnEvent, SignIn};
use crate::{Error, lock};
use reader::{Read, Reader};

/// The harness name a pane saves.
pub const NAME: &str = "claude";

pub const HARNESS: Harness = Harness {
    name: NAME,
    title: "Claude Code",
    binary: "claude",
    install: "curl -fsSL https://claude.ai/install.sh | bash",
    sign_in: Some(SignIn {
        check: &["auth", "status"],
        step: "claude auth login",
    }),
    upgrade: "claude update",
    oldest_tested: (2, 1, 328),
    starts: true,
};

// `--replay-user-messages` echoes each message once a turn takes it in: the chat's user message, and the moment a steer leaves the queue.
const ARGS: [&str; 10] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--include-partial-messages",
    "--replay-user-messages",
    "--permission-prompt-tool",
    "stdio",
];

/// One running `claude -p` process, with the reader of its stdout.
struct Running {
    process: Process,
    reader: Arc<Mutex<Reader>>,
}

/// The pane's Claude Code; a resume swaps its process for one on the picked session.
pub struct Session {
    dir: PathBuf,
    /// The worktree's folder of session files.
    folder: Option<PathBuf>,
    running: Mutex<Running>,
    on_event: OnEvent,
}

impl Session {
    /// Starts Claude Code in `dir` on the saved session `session` when its file exists, else on a new session.
    pub fn start(
        dir: &Path,
        session: Option<&str>,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        require(&HARNESS)?;
        let on_event: OnEvent = Arc::new(Mutex::new(on_event));
        let folder = transcript::folder(dir);
        let running = launch(dir, folder.as_deref(), session, &on_event)?;
        Ok(Session {
            dir: dir.to_path_buf(),
            folder,
            running: Mutex::new(running),
            on_event,
        })
    }

    // The old process ends after the new one started, so a failed start leaves the pane on its session.
    fn resume(&self, session: &str) -> Result<(), Error> {
        let new = launch(
            &self.dir,
            self.folder.as_deref(),
            Some(session),
            &self.on_event,
        )?;
        let old = std::mem::replace(&mut *lock(&self.running), new);
        drop(old);
        Ok(())
    }

    fn reader(&self) -> Arc<Mutex<Reader>> {
        Arc::clone(&lock(&self.running).reader)
    }

    fn write(&self, line: &Value) -> Result<(), Error> {
        lock(&self.running).process.write(line)
    }

    // The events go out before the lines, so a steer's queue lands before the steer's echo, which empties it.
    fn deliver(&self, read: Read) -> Result<(), Error> {
        read.events
            .into_iter()
            .for_each(|event| (lock(&self.on_event))(event));
        read.lines.iter().try_for_each(|line| self.write(line))?;
        read.resume.map_or(Ok(()), |session| self.resume(&session))
    }
}

// A saved session without its file starts a new one: `--resume` on an unknown id prints an error and exits.
fn launch(
    dir: &Path,
    folder: Option<&Path>,
    session: Option<&str>,
    on_event: &OnEvent,
) -> Result<Running, Error> {
    let resumed = session
        .filter(|id| folder.is_some_and(|folder| transcript::file(folder, id).is_file()))
        .map(String::from);
    let mut args = ARGS.to_vec();
    if let Some(id) = &resumed {
        args.extend(["--resume", id.as_str()]);
    }
    let reader = Arc::new(Mutex::new(Reader::new(
        dir,
        folder.map(Path::to_path_buf),
        resumed.clone(),
    )));
    let mut command = login_shell(HARNESS.binary, &args);
    command.current_dir(dir);
    let (records, emit) = (Arc::clone(&reader), Arc::clone(on_event));
    let process = Process::start(command, move |output, stdin| {
        let read = match output {
            Output::Line(line) => lock(&records).feed(line),
            Output::Closed { exited } => {
                let mut events = lock(&records).end();
                events.extend(exited.map(|code| HarnessEvent::Exited { code }));
                Read {
                    events,
                    ..Read::default()
                }
            }
        };
        for line in read.lines {
            let _ = stdin.write(&line);
        }
        read.events
            .into_iter()
            .for_each(|event| (lock(&emit))(event));
    })
    .map_err(|error| Error::start_failed(HARNESS.binary, HARNESS.install, error))?;
    let initialize = lock(&reader).initialize();
    process.write(&initialize)?;
    Ok(Running { process, reader })
}

impl Agent for Session {
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Chat,
            Capability::Attachments,
            Capability::Steer,
            Capability::Models,
            Capability::Context,
        ]
    }

    /// Sends a message, or runs a session command; `/resume` leaves out `held_elsewhere`, the sessions other panes hold.
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<(), Error> {
        let read = lock(&self.reader()).send(text, images, held_elsewhere)?;
        self.deliver(read)
    }

    fn abort(&self) -> Result<(), Error> {
        let interrupt = lock(&self.reader()).interrupt();
        interrupt.map_or(Ok(()), |line| self.write(&line))
    }

    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        let read = lock(&self.reader()).answer(id, answer);
        self.deliver(read)
    }

    fn end_reported(self: Box<Self>) {
        let Session { running, .. } = *self;
        running
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .process
            .end_reported();
    }
}
