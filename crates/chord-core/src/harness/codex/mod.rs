//! The Codex adapter: runs `codex app-server` as a process and hands its stdout to the reader. The only module that knows Codex's records.

mod asks;
mod commands;
mod items;
mod permissions;
mod reader;
mod requests;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;

use super::installed::require;
use super::process::{Output, Process, login_shell};
use super::{Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, OnEvent, SignIn};
use crate::{Error, lock};
use reader::{Read, Reader};

/// The harness name a pane saves.
pub const NAME: &str = "codex";

pub const HARNESS: Harness = Harness {
    name: NAME,
    title: "Codex",
    binary: "codex",
    install: "npm install -g @openai/codex",
    sign_in: Some(SignIn {
        check: &["login", "status"],
        step: "codex login",
    }),
    upgrade: "codex update",
    oldest_tested: (0, 158, 0),
    starts: true,
};

/// One running `codex app-server` process, on one thread.
pub struct Session {
    process: Mutex<Process>,
    reader: Arc<Mutex<Reader>>,
    on_event: OnEvent,
}

impl Session {
    /// Starts Codex in `dir` on the saved thread `session`, else on a new thread.
    pub fn start(
        dir: &Path,
        session: Option<&str>,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        require(&HARNESS)?;
        let on_event: OnEvent = Arc::new(Mutex::new(on_event));
        let reader = Arc::new(Mutex::new(Reader::new(dir, session)));
        let mut command = login_shell(HARNESS.binary, &["app-server"]);
        command.current_dir(dir);
        let (records, emit) = (Arc::clone(&reader), Arc::clone(&on_event));
        let process = Process::start(command, move |output, stdin| {
            let Read { events, lines } = match output {
                Output::Line(line) => lock(&records).feed(line),
                Output::Closed { exited } => {
                    let mut events = lock(&records).end();
                    events.extend(exited.map(|code| HarnessEvent::Exited { code }));
                    Read {
                        events,
                        lines: vec![],
                    }
                }
            };
            for line in lines {
                let _ = stdin.write(&line);
            }
            events.into_iter().for_each(|event| (lock(&emit))(event));
        })
        .map_err(|error| Error::start_failed(HARNESS.binary, HARNESS.install, error))?;
        let initialize = lock(&reader).initialize();
        process.write(&initialize)?;
        Ok(Session {
            process: Mutex::new(process),
            reader,
            on_event,
        })
    }

    fn write(&self, line: &Value) -> Result<(), Error> {
        lock(&self.process).write(line)
    }

    // The events go out before the lines, so a steer's queue lands before the steer's echo, which empties it.
    fn deliver(&self, Read { events, lines }: Read) -> Result<(), Error> {
        events
            .into_iter()
            .for_each(|event| (lock(&self.on_event))(event));
        lines.iter().try_for_each(|line| self.write(line))
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
        ]
    }

    /// Starts a turn, or steers the running one.
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<(), Error> {
        let read = lock(&self.reader).send(text, images, held_elsewhere)?;
        self.deliver(read)
    }

    fn abort(&self) -> Result<(), Error> {
        let interrupt = lock(&self.reader).interrupt();
        interrupt.map_or(Ok(()), |line| self.write(&line))
    }

    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        let read = lock(&self.reader).answer(id, answer)?;
        self.deliver(read)
    }

    fn end_reported(self: Box<Self>) {
        let Session { process, .. } = *self;
        process
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .end_reported();
    }
}
