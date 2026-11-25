use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::installed::require;
use super::process::{Output, Process, login_shell};
use super::{Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, OnEvent};
use crate::{Error, lock};

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

pub(super) trait Reader: Send + 'static {
    type Open;
    fn initialize(&mut self) -> Value;
    fn feed(&mut self, line: &[u8]) -> Read<Self::Open>;
    fn takes_images(&self, text: &str) -> bool;
    fn send(
        &mut self,
        text: &str,
        images: &[Image],
        held_elsewhere: &[String],
    ) -> Result<Read<Self::Open>, Error>;
    fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Result<Read<Self::Open>, Error>;
    fn pick_model(&mut self, id: &str, level: Option<&str>) -> Read<Self::Open>;
    fn interrupt(&mut self) -> Read<Self::Open>;
    fn end(&mut self) -> Vec<HarnessEvent>;
    fn quiet_after(&self) -> Option<Duration> {
        None
    }
    fn quiet(&mut self) -> Vec<HarnessEvent> {
        vec![]
    }
}

struct Deadline {
    at: Option<Instant>,
    ended: bool,
}

struct Quiet {
    deadline: Mutex<Deadline>,
    wake: Condvar,
}

impl Quiet {
    fn arm(&self, after: Option<Duration>) {
        lock(&self.deadline).at = after.map(|after| Instant::now() + after);
        self.wake.notify_one();
    }

    fn end(&self) {
        lock(&self.deadline).ended = true;
        self.wake.notify_one();
    }

    fn wait(&self) -> bool {
        let mut deadline = lock(&self.deadline);
        loop {
            if deadline.ended {
                return false;
            }
            let now = Instant::now();
            deadline = match deadline.at {
                Some(at) if at <= now => {
                    deadline.at = None;
                    return true;
                }
                Some(at) => {
                    self.wake
                        .wait_timeout(deadline, at - now)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
                None => self
                    .wake
                    .wait(deadline)
                    .unwrap_or_else(PoisonError::into_inner),
            };
        }
    }
}

type Launch<R> = Box<dyn Fn(<R as Reader>::Open) -> (Vec<String>, R) + Send>;

struct Running<R> {
    process: Process,
    reader: Arc<Mutex<R>>,
    quiet: Arc<Quiet>,
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

    pub(super) fn relaunch(&self, open: R::Open) -> Result<(), Error> {
        let emit = lock(&self.on_event);
        let new = run(self.harness, &self.dir, &self.launch, open, &self.on_event)?;
        let old = std::mem::replace(&mut *lock(&self.running), new);
        old.current.store(false, Ordering::SeqCst);
        let ended = lock(&old.reader).end();
        ended.into_iter().for_each(&*emit);
        emit(HarnessEvent::State { running: false });
        emit(HarnessEvent::HistoryStart);
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

    pub(super) fn deliver(&self, read: Read<R::Open>) -> Result<(), Error> {
        read.events
            .into_iter()
            .for_each(|event| (lock(&self.on_event))(event));
        read.lines.iter().try_for_each(|line| self.write(line))?;
        {
            let running = lock(&self.running);
            running.quiet.arm(lock(&running.reader).quiet_after());
        }
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

impl<R: Reader> Agent for Relaunchable<R> {
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

    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<bool, Error> {
        let reader = self.reader();
        let took = lock(&reader).takes_images(text);
        let images = if took { images } else { &[] };
        let read = lock(&reader).send(text, images, held_elsewhere)?;
        self.deliver(read).map(|()| took)
    }

    fn abort(&self) -> Result<(), Error> {
        let read = lock(&self.reader()).interrupt();
        self.deliver(read)
    }

    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        let read = lock(&self.reader()).answer(id, answer)?;
        self.deliver(read)
    }

    fn pick_model(&self, id: &str, level: Option<&str>) -> Result<(), Error> {
        let read = lock(&self.reader()).pick_model(id, level);
        self.deliver(read)
    }

    fn end_reported(self: Box<Self>) {
        Relaunchable::end_reported(*self);
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
    let quiet = Arc::new(Quiet {
        deadline: Mutex::new(Deadline {
            at: None,
            ended: false,
        }),
        wake: Condvar::new(),
    });
    let (records, emit, live, silence) = (
        Arc::clone(&reader),
        Arc::clone(on_event),
        Arc::clone(&current),
        Arc::clone(&quiet),
    );
    let process = Process::start(command, move |output, stdin| {
        let emit = lock(&emit);
        let read = match output {
            Output::Line(line) => {
                let read = lock(&records).feed(line);
                silence.arm(lock(&records).quiet_after());
                read
            }
            Output::Closed { exited } => {
                silence.end();
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
        if live.load(Ordering::SeqCst) {
            read.events.into_iter().for_each(&*emit);
        }
    })
    .map_err(|error| Error::start_failed(harness.binary, harness.install, error))?;
    let (records, emit, live, silence) = (
        Arc::clone(&reader),
        Arc::clone(on_event),
        Arc::clone(&current),
        Arc::clone(&quiet),
    );
    thread::spawn(move || {
        while silence.wait() {
            let emit = lock(&emit);
            let events = lock(&records).quiet();
            if live.load(Ordering::SeqCst) {
                events.into_iter().for_each(&*emit);
            }
        }
    });
    let initialize = lock(&reader).initialize();
    process.write(&initialize)?;
    Ok(Running {
        process,
        reader,
        quiet,
        current,
    })
}
