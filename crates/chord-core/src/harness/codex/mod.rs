//! The Codex adapter: runs `codex app-server` as a process and hands its stdout to the reader. The only module that knows Codex's records.

mod asks;
mod commands;
mod items;
mod permissions;
mod reader;
mod requests;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

use super::installed::require;
use super::process::{Output, Process, login_shell};
use super::{
    Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, OnEvent, SignIn, agent_tools,
};
use crate::{Error, lock};
use reader::{Read, Reader, Relaunch};

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

/// Codex's 60 s tool timeout would end a cold build's call.
const TOOL_TIMEOUT_SEC: u32 = 3600;

/// One running `codex app-server` process, with the reader of its stdout.
struct Running {
    process: Process,
    reader: Arc<Mutex<Reader>>,
    /// Cleared when a relaunch replaces the process.
    current: Arc<AtomicBool>,
}

/// The pane's Codex; `/new` and `/reload` swap its process for another.
pub struct Session {
    dir: PathBuf,
    /// The socket of the build and run tools, for a new process.
    tools: Option<PathBuf>,
    running: Mutex<Running>,
    on_event: OnEvent,
}

impl Session {
    /// Starts Codex in `dir` on the saved thread `session`, else on a new thread. With `tools`, the socket Chord's tools listen on, the agent gets the build and run tools.
    pub fn start(
        dir: &Path,
        session: Option<&str>,
        tools: Option<&Path>,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        require(&HARNESS)?;
        let on_event: OnEvent = Arc::new(Mutex::new(on_event));
        let open = Relaunch {
            thread: session.map(String::from),
            ..Relaunch::default()
        };
        let running = launch(dir, open, tools, &on_event)?;
        Ok(Session {
            dir: dir.to_path_buf(),
            tools: tools.map(Path::to_path_buf),
            running: Mutex::new(running),
            on_event,
        })
    }

    // The old process ends after the new one started, so a failed start leaves the pane on its thread; a turn the old one ran ends with it.
    fn relaunch(&self, open: Relaunch) -> Result<(), Error> {
        // Holding the channel keeps the new process's events behind the old one's last ones.
        let emit = lock(&self.on_event);
        let new = launch(&self.dir, open, self.tools.as_deref(), &self.on_event)?;
        let old = std::mem::replace(&mut *lock(&self.running), new);
        old.current.store(false, Ordering::SeqCst);
        let ended = lock(&old.reader).end();
        ended.into_iter().for_each(&*emit);
        emit(HarnessEvent::State { running: false });
        drop(emit);
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
    fn deliver(&self, Read { events, lines }: Read) -> Result<(), Error> {
        events
            .into_iter()
            .for_each(|event| (lock(&self.on_event))(event));
        lines.iter().try_for_each(|line| self.write(line))
    }
}

fn launch(
    dir: &Path,
    open: Relaunch,
    tools: Option<&Path>,
    on_event: &OnEvent,
) -> Result<Running, Error> {
    let reader = Arc::new(Mutex::new(Reader::new(
        dir,
        open.thread.as_deref(),
        open.picks,
    )));
    let mut args = vec!["app-server".to_string()];
    if let Some((command, server_args)) = tools.and_then(|socket| agent_tools::command(socket, dir))
    {
        let overrides = [
            ("command", json!(command).to_string()),
            ("args", json!(server_args).to_string()),
            ("tool_timeout_sec", TOOL_TIMEOUT_SEC.to_string()),
            ("default_tools_approval_mode", r#""approve""#.into()),
        ];
        for (key, value) in overrides {
            args.extend(["-c".into(), format!("mcp_servers.chord.{key}={value}")]);
        }
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut command = login_shell(HARNESS.binary, &args);
    command.current_dir(dir);
    let current = Arc::new(AtomicBool::new(true));
    let (records, emit, live) = (
        Arc::clone(&reader),
        Arc::clone(on_event),
        Arc::clone(&current),
    );
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
        let emit = lock(&emit);
        if live.load(Ordering::SeqCst) {
            events.into_iter().for_each(&*emit);
        }
    })
    .map_err(|error| Error::start_failed(HARNESS.binary, HARNESS.install, error))?;
    let initialize = lock(&reader).initialize();
    process.write(&initialize)?;
    Ok(Running {
        process,
        reader,
        current,
    })
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

    /// Starts a turn, steers the running one, or runs a session command.
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<(), Error> {
        let reader = self.reader();
        let relaunch = images
            .is_empty()
            .then(|| lock(&reader).relaunch(text))
            .flatten();
        if let Some(open) = relaunch {
            return self.relaunch(open);
        }
        let read = lock(&reader).send(text, images, held_elsewhere)?;
        self.deliver(read)
    }

    fn abort(&self) -> Result<(), Error> {
        let interrupt = lock(&self.reader()).interrupt();
        interrupt.map_or(Ok(()), |line| self.write(&line))
    }

    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        let read = lock(&self.reader()).answer(id, answer)?;
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
