//! A harness process a session command can swap for a new one: the running process and the reader of its stdout, the swap, and the order events and lines go out in. The Codex and Claude Code adapters build on it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::Value;

use super::installed::require;
use super::process::{Output, Process, login_shell};
use super::{Harness, HarnessEvent, OnEvent};
use crate::{Error, lock};

/// What one record or call leads to: events for the chat, lines to write the harness, and what a new process opens in place of this one.
pub(super) struct Read<O> {
    pub(super) events: Vec<HarnessEvent>,
    pub(super) lines: Vec<Value>,
    pub(super) relaunch: Option<O>,
}

impl<O> Default for Read<O> {
    fn default() -> Self {
        Read {
            events: vec![],
            lines: vec![],
            relaunch: None,
        }
    }
}

/// The reader of one process's stdout.
pub(super) trait Reader: Send + 'static {
    /// What a new process opens.
    type Open;
    /// The first line the process gets.
    fn initialize(&mut self) -> Value;
    fn feed(&mut self, line: &[u8]) -> Read<Self::Open>;
    /// Stdout closed: the events that end what the process left open.
    fn end(&mut self) -> Vec<HarnessEvent>;
}

/// The arguments and the reader of a new process that opens `Open`.
type Launch<R> = Box<dyn Fn(<R as Reader>::Open) -> (Vec<String>, R) + Send>;

struct Running<R> {
    process: Process,
    reader: Arc<Mutex<R>>,
    /// Cleared when a relaunch replaces the process.
    current: Arc<AtomicBool>,
}

pub(super) struct Relaunchable<R: Reader> {
    harness: &'static Harness,
    dir: PathBuf,
    launch: Launch<R>,
    running: Mutex<Running<R>>,
    on_event: OnEvent,
}

impl<R: Reader> Relaunchable<R> {
    /// Starts `harness` in `dir` on `open`, with the arguments and reader `launch` builds.
    pub(super) fn start(
        harness: &'static Harness,
        dir: &Path,
        open: R::Open,
        launch: impl Fn(R::Open) -> (Vec<String>, R) + Send + 'static,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Self, Error> {
        if let Some(text) = require(harness)? {
            on_event(HarnessEvent::Notice { text });
        }
        let on_event: OnEvent = Arc::new(Mutex::new(on_event));
        let launch: Launch<R> = Box::new(launch);
        let running = run(harness, dir, &launch, open, &on_event)?;
        Ok(Relaunchable {
            harness,
            dir: dir.to_path_buf(),
            launch,
            running: Mutex::new(running),
            on_event,
        })
    }

    // The old process ends after the new one started, so a failed start leaves the pane on its session; a turn the old one ran ends with it.
    pub(super) fn relaunch(&self, open: R::Open) -> Result<(), Error> {
        // Holding the channel keeps the new process's events behind the old one's last ones.
        let emit = lock(&self.on_event);
        let new = run(self.harness, &self.dir, &self.launch, open, &self.on_event)?;
        let old = std::mem::replace(&mut *lock(&self.running), new);
        old.current.store(false, Ordering::SeqCst);
        let ended = lock(&old.reader).end();
        ended.into_iter().for_each(&*emit);
        emit(HarnessEvent::State { running: false });
        drop(emit);
        drop(old);
        Ok(())
    }

    pub(super) fn reader(&self) -> Arc<Mutex<R>> {
        Arc::clone(&lock(&self.running).reader)
    }

    pub(super) fn write(&self, line: &Value) -> Result<(), Error> {
        lock(&self.running).process.write(line)
    }

    // The events go out before the lines, so a steer's queue lands before the steer's echo, which empties it.
    pub(super) fn deliver(&self, read: Read<R::Open>) -> Result<(), Error> {
        read.events
            .into_iter()
            .for_each(|event| (lock(&self.on_event))(event));
        read.lines.iter().try_for_each(|line| self.write(line))?;
        read.relaunch.map_or(Ok(()), |open| self.relaunch(open))
    }

    pub(super) fn end_reported(self) {
        self.running
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner)
            .process
            .end_reported();
    }
}

fn run<R: Reader>(
    harness: &Harness,
    dir: &Path,
    launch: &Launch<R>,
    open: R::Open,
    on_event: &OnEvent,
) -> Result<Running<R>, Error> {
    let (args, reader) = launch(open);
    let reader = Arc::new(Mutex::new(reader));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut command = login_shell(harness.binary, &args);
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
    .map_err(|error| Error::start_failed(harness.binary, harness.install, error))?;
    let initialize = lock(&reader).initialize();
    process.write(&initialize)?;
    Ok(Running {
        process,
        reader,
        current,
    })
}
