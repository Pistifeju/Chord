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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

use super::installed::require;
use super::process::{Output, Process, login_shell};
use super::{
    Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, OnEvent, SignIn, agent_tools,
};
use crate::{Error, lock};
use reader::{Open, Read, Reader};

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
    /// Cleared when a relaunch replaces the process.
    current: Arc<AtomicBool>,
}

/// The pane's Claude Code; a session command that opens another session, or the same one again, swaps its process.
pub struct Session {
    dir: PathBuf,
    /// The worktree's folder of session files.
    folder: Option<PathBuf>,
    /// The socket of the build and run tools, for a resume's new process.
    tools: Option<PathBuf>,
    running: Mutex<Running>,
    on_event: OnEvent,
}

impl Session {
    /// Starts Claude Code in `dir` on the saved session `session` when its file exists, else on a new session. With `tools`, the socket Chord's tools listen on, the agent gets the build and run tools.
    pub fn start(
        dir: &Path,
        session: Option<&str>,
        tools: Option<&Path>,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        require(&HARNESS)?;
        let on_event: OnEvent = Arc::new(Mutex::new(on_event));
        let folder = transcript::folder(dir);
        let open = Open {
            session: session.map(String::from),
            ..Open::default()
        };
        let running = launch(dir, folder.as_deref(), open, tools, &on_event)?;
        Ok(Session {
            dir: dir.to_path_buf(),
            folder,
            tools: tools.map(Path::to_path_buf),
            running: Mutex::new(running),
            on_event,
        })
    }

    // The old process ends after the new one started, so a failed start leaves the pane on its session; a turn the old one ran ends with it.
    fn relaunch(&self, open: Open) -> Result<(), Error> {
        // Holding the channel keeps the new process's events behind the old one's last ones.
        let emit = lock(&self.on_event);
        let new = launch(
            &self.dir,
            self.folder.as_deref(),
            open,
            self.tools.as_deref(),
            &self.on_event,
        )?;
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
    fn deliver(&self, read: Read) -> Result<(), Error> {
        read.events
            .into_iter()
            .for_each(|event| (lock(&self.on_event))(event));
        read.lines.iter().try_for_each(|line| self.write(line))?;
        read.relaunch.map_or(Ok(()), |open| self.relaunch(open))
    }
}

// A saved session without its file starts a new one: `--resume` on an unknown id prints an error and exits.
fn launch(
    dir: &Path,
    folder: Option<&Path>,
    open: Open,
    tools: Option<&Path>,
    on_event: &OnEvent,
) -> Result<Running, Error> {
    let found = |id: &String| folder.is_some_and(|folder| transcript::file(folder, id).is_file());
    let open = match open.session {
        Some(id) if found(&id) => Open {
            session: Some(id),
            ..open
        },
        _ => Open {
            picks: open.picks,
            ..Open::default()
        },
    };
    let mut args: Vec<String> = ARGS.map(String::from).to_vec();
    if let Some(id) = &open.session {
        args.extend(["--resume".into(), id.clone()]);
    }
    if open.fork {
        args.push("--fork-session".into());
    }
    if let Some(at) = &open.at {
        args.extend(["--resume-session-at".into(), at.clone()]);
    }
    let picks = [
        ("--model", &open.picks.model),
        ("--effort", &open.picks.effort),
        ("--permission-mode", &open.picks.mode),
    ];
    for (flag, pick) in picks {
        args.extend(pick.iter().flat_map(|pick| [flag.into(), pick.clone()]));
    }
    // The tools are Chord's own, so they ask no permission, as in pi.
    if let Some((command, server_args)) = tools.and_then(|socket| agent_tools::command(socket, dir))
    {
        let config =
            json!({ "mcpServers": { "chord": { "command": command, "args": server_args } } });
        args.extend([
            "--mcp-config".into(),
            config.to_string(),
            "--allowedTools".into(),
            "mcp__chord__build,mcp__chord__run".into(),
        ]);
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let reader = Arc::new(Mutex::new(Reader::new(
        dir,
        folder.map(Path::to_path_buf),
        open,
    )));
    let mut command = login_shell(HARNESS.binary, &args);
    command.current_dir(dir);
    let current = Arc::new(AtomicBool::new(true));
    let (records, emit, live) = (
        Arc::clone(&reader),
        Arc::clone(on_event),
        Arc::clone(&current),
    );
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
        let emit = lock(&emit);
        if live.load(Ordering::SeqCst) {
            read.events.into_iter().for_each(&*emit);
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
