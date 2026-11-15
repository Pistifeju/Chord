//! One xcodebuild per worktree: its arguments, its own process group, its log in batches, the wait while a build Chord did not start holds the build database, and the stop that signals the whole group.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use chord_core::{Error, login_shell};
use specta::Type;

use crate::derived_data;
use crate::issues::{BuildIssue, Issues};
use crate::slot::Slots;
use crate::xcode::{self, Project};

/// The log reaches the UI every 100 ms or 64 KB, whichever comes first: a cold build prints thousands of lines.
const BATCH_INTERVAL: Duration = Duration::from_millis(100);
const BATCH_BYTES: usize = 64 * 1024;
/// How long a signalled group may take to end before it is killed, and how long the build database may stay held after it.
const GROUP_GRACE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(100);

#[derive(Serialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuildPhase {
    /// Before xcodebuild: the project's scheme is looked up.
    Preparing,
    /// A build Chord did not start holds the build database.
    Waiting,
    Building,
}

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BuildEvent {
    Phase {
        phase: BuildPhase,
    },
    Log {
        text: String,
    },
    Succeeded,
    /// Stopped by the user; a build a newer one replaced sends nothing.
    Stopped,
    /// The first errors of the build, and how many more it printed.
    Failed {
        issues: Vec<BuildIssue>,
        more: u32,
    },
}

/// The worktrees' builds, one slot each. Clones share the same builds.
#[derive(Clone, Default)]
pub struct Builds(Arc<Inner>);

#[derive(Default)]
struct Inner {
    slots: Mutex<Slots>,
    /// Held by a worktree's build thread from its start to its end, so a newer build starts after the old one ended.
    turns: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
    /// The scheme of each project from its last build, so the next one skips `xcodebuild -list`.
    schemes: Mutex<HashMap<PathBuf, String>>,
    /// The tools were found once; later builds skip the login shells that check them.
    tools_found: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Builds {
    /// Starts a build of the worktree's project and returns; `on_event` hears its phases, log and end from a thread of its own. A build already running there stops first. A build that cannot start, such as a missing `xcodebuild` with its install step, ends as a failed one.
    pub fn start(&self, worktree: PathBuf, on_event: impl Fn(BuildEvent) + Send + Sync + 'static) {
        let project = match self.project_of(&worktree) {
            Ok(project) => project,
            Err(error) => {
                let message = error.to_string();
                on_event(BuildEvent::Log {
                    text: format!("{message}\n"),
                });
                on_event(failure(message));
                return;
            }
        };
        let start = lock(&self.0.slots).ask(&worktree);
        if let Some(pid) = start.kill {
            thread::spawn(move || end_group(pid));
        }
        let turn = Arc::clone(lock(&self.0.turns).entry(worktree.clone()).or_default());
        let build = Build {
            builds: self.clone(),
            worktree,
            id: start.id,
            project,
            sink: Box::new(on_event),
        };
        thread::spawn(move || {
            let _turn = lock(&turn);
            build.run();
        });
    }

    fn project_of(&self, worktree: &Path) -> Result<Project, Error> {
        if !self.0.tools_found.load(Ordering::Relaxed) {
            xcode::require_tools()?;
            self.0.tools_found.store(true, Ordering::Relaxed);
        }
        xcode::project_in(worktree)
    }

    /// Stops the worktree's build without waiting for it.
    pub fn stop(&self, worktree: &Path) {
        if let Some(pid) = lock(&self.0.slots).stop(worktree) {
            thread::spawn(move || end_group(pid));
        }
    }

    /// Stops every build and returns once the builds ended, build database released: the app is quitting.
    pub fn stop_all(&self) {
        let pids = lock(&self.0.slots).stop_all();
        let ending: Vec<_> = pids
            .into_iter()
            .map(|pid| thread::spawn(move || end_group(pid)))
            .collect();
        for handle in ending {
            let _ = handle.join();
        }
        let turns: Vec<_> = lock(&self.0.turns).values().cloned().collect();
        for turn in turns {
            drop(lock(&turn));
        }
    }

    /// For a worktree that was deleted: stops its build, returns once the build ended, build database released, and forgets its schemes.
    pub fn forget(&self, worktree: &Path) {
        let pid = lock(&self.0.slots).stop(worktree);
        if let Some(pid) = pid {
            end_group(pid);
        }
        let turn = lock(&self.0.turns).remove(worktree);
        if let Some(turn) = turn {
            drop(lock(&turn));
        }
        lock(&self.0.schemes).retain(|project, _| !project.starts_with(worktree));
    }
}

struct Build {
    builds: Builds,
    worktree: PathBuf,
    id: u64,
    project: Project,
    sink: Box<dyn Fn(BuildEvent) + Send + Sync>,
}

impl Build {
    fn run(&self) {
        let end = self.outcome();
        self.say(end);
        lock(&self.builds.0.slots).finished(&self.worktree, self.id);
    }

    /// Says an event unless a newer build took the slot: the old one stays silent.
    fn say(&self, event: BuildEvent) {
        let current = lock(&self.builds.0.slots).is_current(&self.worktree, self.id);
        if current {
            (self.sink)(event);
        }
    }

    fn log(&self, text: String) {
        self.say(BuildEvent::Log { text });
    }

    fn stopped(&self) -> bool {
        lock(&self.builds.0.slots).stopped(&self.worktree, self.id)
    }

    fn outcome(&self) -> BuildEvent {
        if self.stopped() {
            return BuildEvent::Stopped;
        }
        self.say(BuildEvent::Phase {
            phase: BuildPhase::Preparing,
        });
        let scheme = match self.scheme() {
            Ok(scheme) => scheme,
            Err(_) if self.stopped() => return BuildEvent::Stopped,
            Err(error) => return self.failed(error.to_string()),
        };
        if !self.wait_for_database() {
            return BuildEvent::Stopped;
        }
        self.say(BuildEvent::Phase {
            phase: BuildPhase::Building,
        });
        self.xcodebuild(&scheme)
    }

    fn failed(&self, message: String) -> BuildEvent {
        self.log(format!("{message}\n"));
        failure(message)
    }

    fn scheme(&self) -> Result<String, Error> {
        if let Some(scheme) = lock(&self.builds.0.schemes).get(&self.project.path) {
            return Ok(scheme.clone());
        }
        let scheme = xcode::scheme_of(&self.project, &self.worktree, &|pid| self.register(pid));
        lock(&self.builds.0.slots).clear_pid(&self.worktree, self.id);
        let scheme = scheme?;
        lock(&self.builds.0.schemes).insert(self.project.path.clone(), scheme.clone());
        Ok(scheme)
    }

    fn database(&self) -> Option<PathBuf> {
        derived_data::folder_of(&self.project.path).map(|folder| derived_data::build_db(&folder))
    }

    /// Waits while some process holds the build database, and never ends that process. False when the build was stopped meanwhile.
    fn wait_for_database(&self) -> bool {
        let Some(database) = self.database() else {
            return !self.stopped();
        };
        let mut told = false;
        while !self.stopped() && !derived_data::holders(&database).is_empty() {
            if !told {
                told = true;
                self.say(BuildEvent::Phase {
                    phase: BuildPhase::Waiting,
                });
                self.log("Waiting for another build in this worktree\n".into());
            }
            for _ in 0..10 {
                if !self.stopped() {
                    thread::sleep(POLL);
                }
            }
        }
        !self.stopped()
    }

    /// After a stop: gives the processes of the group a moment to let go of the build database, and says so when one does not.
    fn wait_for_release(&self) {
        let Some(database) = self.database() else {
            return;
        };
        let until = Instant::now() + GROUP_GRACE;
        while !derived_data::holders(&database).is_empty() {
            if Instant::now() > until {
                self.log("A process still holds the build database.\n".into());
                return;
            }
            thread::sleep(POLL);
        }
    }

    fn xcodebuild(&self, scheme: &str) -> BuildEvent {
        let args = build_args(&self.project, scheme);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let Ok((reader, writer)) = std::io::pipe() else {
            return self.failed("Could not open a pipe for xcodebuild".into());
        };
        let Ok(writer_for_errors) = writer.try_clone() else {
            return self.failed("Could not open a pipe for xcodebuild".into());
        };
        let mut command = login_shell("xcodebuild", &arg_refs);
        command
            .current_dir(&self.worktree)
            .stdin(Stdio::null())
            .stdout(writer)
            .stderr(writer_for_errors)
            .process_group(0);
        self.log(format!("$ xcodebuild {}\n", args.join(" ")));
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => return self.failed(format!("xcodebuild did not start: {error}")),
        };
        // The command's copies of the pipe's write end must close, or the reader never sees the end.
        drop(command);
        self.register(child.id());

        let (lines, received) = mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            let mut reader = BufReader::new(reader);
            loop {
                let mut line = Vec::new();
                if !matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0)
                    || lines.send(line).is_err()
                {
                    return;
                }
            }
        });
        let mut issues = Issues::default();
        let mut batch = String::new();
        let mut flushed = Instant::now();
        loop {
            match received.recv_timeout(BATCH_INTERVAL) {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    issues.push_line(&text);
                    batch.push_str(&text);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            if !batch.is_empty()
                && (batch.len() >= BATCH_BYTES || flushed.elapsed() >= BATCH_INTERVAL)
            {
                self.log(std::mem::take(&mut batch));
                flushed = Instant::now();
            }
        }
        if !batch.is_empty() {
            self.log(batch);
        }
        let status = child.wait();
        lock(&self.builds.0.slots).clear_pid(&self.worktree, self.id);
        if self.stopped() {
            self.wait_for_release();
            return BuildEvent::Stopped;
        }
        if status.as_ref().is_ok_and(|status| status.success()) {
            return BuildEvent::Succeeded;
        }
        // A renamed scheme may be why: the next build lists the schemes again.
        lock(&self.builds.0.schemes).remove(&self.project.path);
        if issues.total() == 0 {
            let message = match status.ok().and_then(|status| status.code()) {
                Some(code) => format!("xcodebuild exited with code {code}"),
                None => "xcodebuild ended without an exit code".to_string(),
            };
            return self.failed(message);
        }
        let (issues, more) = issues.into_listed();
        BuildEvent::Failed { issues, more }
    }

    /// Records a process group of the build in its slot. A build stopped or replaced before it started has its group ended at once.
    fn register(&self, pid: u32) {
        let end = lock(&self.builds.0.slots).register(&self.worktree, self.id, pid);
        if end {
            thread::spawn(move || end_group(pid));
        }
    }
}

fn failure(message: String) -> BuildEvent {
    BuildEvent::Failed {
        issues: vec![BuildIssue {
            file: None,
            line: None,
            message,
        }],
        more: 0,
    }
}

/// `xcodebuild` and nothing else: no `-derivedDataPath` and no build setting, so Xcode's own folder for the project is used. The destination is generic: no simulator exists before ⌘R.
fn build_args(project: &Project, scheme: &str) -> Vec<String> {
    [
        project.flag,
        &project.path.to_string_lossy(),
        "-scheme",
        scheme,
        "-destination",
        "generic/platform=iOS Simulator",
        "build",
    ]
    .map(String::from)
    .into()
}

fn signal_group(pid: u32, signal: i32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    // SAFETY: `kill` takes plain integers and has no memory effects.
    unsafe { libc::kill(-pid, signal) == 0 }
}

/// Asks the group to end, waits, then kills what is left, so no compiler keeps the build database locked.
fn end_group(pid: u32) {
    signal_group(pid, libc::SIGTERM);
    let until = Instant::now() + GROUP_GRACE;
    while signal_group(pid, 0) {
        if Instant::now() > until {
            signal_group(pid, libc::SIGKILL);
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}
