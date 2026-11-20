//! One build per worktree: its xcodebuild with its arguments, its own process group and its log in batches, the wait while a build Chord did not start holds the build database, the run of the app on the run destination after it with its console, and the stop that signals the whole group.

use chord_core::lock;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chord_core::{Error, RunDestination, SchemePick, login_shell};
use serde::Serialize;
use specta::Type;

use crate::capacity::{self, Booted};
use crate::compile_commands::DriverLines;
use crate::console::ConsoleTone;
use crate::derived_data;
use crate::device;
use crate::hot_reload;
use crate::issues::{BuildIssue, Issues};
use crate::preview::{self, PreviewState};
use crate::process::{self, BATCH_BYTES, BATCH_INTERVAL, GROUP_GRACE, end_group};
use crate::simulator::{self, DeviceSet, Simulator};
use crate::simulator_kit;
use crate::slot::{Slots, Start};
use crate::xcode::{self, Project};

mod preview_session;

use preview_session::Preview;

const POLL: Duration = Duration::from_millis(100);

#[derive(Serialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuildPhase {
    /// Before xcodebuild: the project's scheme is looked up.
    Preparing,
    /// A build Chord did not start holds the build database.
    Waiting,
    Building,
    /// The build succeeded: the simulator boots, then the app installs.
    Installing,
    Launching,
    /// The app runs on the destination.
    Running,
}

/// What a build targets.
enum Target {
    /// A build that finds no simulator of the worktree's builds for any simulator.
    AnySimulator,
    Simulator(Simulator),
    /// A connected device, by its hardware UDID.
    Device(String),
}

impl Target {
    fn id(&self) -> Option<&str> {
        match self {
            Target::AnySimulator => None,
            Target::Simulator(device) => Some(&device.udid),
            Target::Device(udid) => Some(udid),
        }
    }
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
    /// Whole lines of the running app's output.
    Console {
        text: String,
        tone: ConsoleTone,
    },
    Succeeded,
    /// The app that ran ended on its own.
    Exited,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Lane {
    Main,
    BesideApp,
    Preview,
}

type TurnKey = (PathBuf, Lane);

#[derive(Default)]
struct Inner {
    slots: Mutex<Slots>,
    /// Held by a worktree's build thread from its start to its end (a build beside the app run and a preview build have their own turns), so a newer build starts after the old one ended.
    turns: Mutex<HashMap<TurnKey, Arc<Mutex<()>>>>,
    previews: Mutex<HashMap<PathBuf, Preview>>,
    /// The saves of each worktree whose Preview view opened.
    watchers: Mutex<HashMap<PathBuf, notify::RecommendedWatcher>>,
    /// The scheme of each project from its last build, so the next one skips `xcodebuild -list`.
    schemes: Mutex<HashMap<PathBuf, String>>,
    /// Held while a worktree's simulator is looked up and created, so two builds never create it twice.
    simulators: Mutex<()>,
    /// When each worktree's simulator (by name) was last opened in the app.
    opened: Mutex<HashMap<String, Instant>>,
    /// The tools were found once; later builds skip the login shells that check them.
    tools_found: AtomicBool,
}

impl Builds {
    /// Starts a build of the worktree's project in the scheme and configuration of `pick`, and with `run` the app's install and launch, and returns. The app runs on the `destination`: a connected device, or a simulator of that model that is the worktree's own; without one, the worktree's own simulator of the newest iPhone model. `kept_services` are the user's switches of the service groups a boot keeps on. `on_event` hears its phases, log and end from a thread of its own. A build already running there stops first. A build that cannot start, such as a missing `xcodebuild` with its install step, ends as a failed one.
    pub fn start(
        &self,
        worktree: PathBuf,
        run: bool,
        destination: Option<RunDestination>,
        pick: SchemePick,
        kept_services: BTreeMap<String, bool>,
        on_event: impl Fn(BuildEvent) + Send + Sync + 'static,
    ) {
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
        let (start, preview_waits) = {
            let mut slots = lock(&self.0.slots);
            let start = slots.ask(&worktree, run);
            (start, slots.preview_waits(&worktree))
        };
        if preview_waits {
            self.tell_preview(&worktree, PreviewState::Waiting);
        }
        let lane = if start.beside_app {
            Lane::BesideApp
        } else {
            Lane::Main
        };
        self.spawn(
            start,
            lane,
            Build {
                builds: self.clone(),
                worktree,
                id: 0,
                run,
                preview: false,
                destination,
                pick,
                kept_services,
                project,
                sink: Box::new(on_event),
            },
        );
    }

    /// Ends the builds `start` replaced and runs `build` with the slot's id on a thread, after the lane's older build ended.
    fn spawn(&self, start: Start, lane: Lane, mut build: Build) {
        for pid in start.kill {
            thread::spawn(move || end_group(pid));
        }
        let turn = Arc::clone(
            lock(&self.0.turns)
                .entry((build.worktree.clone(), lane))
                .or_default(),
        );
        build.id = start.id;
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
        for pid in lock(&self.0.slots).stop(worktree) {
            thread::spawn(move || end_group(pid));
        }
    }

    /// Stops every build and returns once the builds ended, build database released: the app is quitting.
    pub fn stop_all(&self) {
        let pids = lock(&self.0.slots).stop_all();
        end_groups(pids);
        let turns: Vec<_> = lock(&self.0.turns).values().cloned().collect();
        for turn in turns {
            drop(lock(&turn));
        }
    }

    /// Records that the user opened the worktree in the app.
    pub fn opened(&self, worktree: &Path) {
        lock(&self.0.opened).insert(simulator::name_of(worktree), Instant::now());
    }

    /// When each worktree's simulator (by name) was last opened, and the names of those with a build or a running app.
    fn opened_and_running(&self) -> (HashMap<String, Instant>, HashSet<String>) {
        let busy = lock(&self.0.slots)
            .held_worktrees()
            .iter()
            .map(|worktree| simulator::name_of(worktree))
            .collect();
        (lock(&self.0.opened).clone(), busy)
    }

    /// For a worktree that was deleted: stops its build, waits for the build database to be released, forgets its schemes, then removes its build data.
    pub fn forget(&self, worktree: &Path) {
        let pids = {
            let mut slots = lock(&self.0.slots);
            let mut pids = slots.stop(worktree);
            pids.extend(slots.forget_preview(worktree));
            pids
        };
        end_groups(pids);
        let mut turns = lock(&self.0.turns);
        let forgotten: Vec<_> = turns.extract_if(|(path, _), _| path == worktree).collect();
        drop(turns);
        for (_, turn) in forgotten {
            drop(lock(&turn));
        }
        lock(&self.0.schemes).retain(|project, _| !project.starts_with(worktree));
        lock(&self.0.opened).remove(&simulator::name_of(worktree));
        lock(&self.0.previews).remove(worktree);
        lock(&self.0.watchers).remove(worktree);
        hot_reload::forget(worktree);
        crate::caches::forget(worktree);
    }

    /// Clean Build Folder, as Xcode's: stops the worktree's build and run, waits for them to end, then removes the project's `Build` folder in DerivedData and the worktree's hot reload folder, whose compile commands point into it.
    pub fn clean(&self, worktree: &Path) -> Result<(), Error> {
        let project = self.project_of(worktree)?;
        let pids = lock(&self.0.slots).stop(worktree);
        end_groups(pids);
        let turns: Vec<_> = {
            let turns = lock(&self.0.turns);
            [Lane::Main, Lane::BesideApp]
                .into_iter()
                .filter_map(|lane| turns.get(&(worktree.to_path_buf(), lane)).cloned())
                .collect()
        };
        // Holding the turns keeps a queued build from writing into the folders while they go.
        let _held: Vec<_> = turns.iter().map(|turn| lock(turn)).collect();
        let build = derived_data::folder_of(&project.path).map(|folder| folder.join("Build"));
        for folder in [build, preview::hot_reload_folder(worktree)]
            .into_iter()
            .flatten()
        {
            match fs::remove_dir_all(&folder) {
                Err(error) if error.kind() != ErrorKind::NotFound => {
                    return Err(Error::Message(format!(
                        "Could not remove {}: {error}",
                        folder.display()
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Ends the process groups and returns once each one ended.
fn end_groups(pids: Vec<u32>) {
    let ending: Vec<_> = pids
        .into_iter()
        .map(|pid| thread::spawn(move || end_group(pid)))
        .collect();
    for handle in ending {
        let _ = handle.join();
    }
}

/// Shuts down the simulators beyond what the RAM allows before `udid` boots. Best effort: a list that fails leaves them booted.
fn make_room(udid: &str, opened: &HashMap<String, Instant>, busy: &HashSet<String>) {
    let Ok(owned) = simulator::owned(DeviceSet::Run) else {
        return;
    };
    // The target takes a place only when it is one of Chord's own.
    let keep = if owned.iter().any(|device| device.udid == udid) {
        capacity::limit(capacity::installed_ram()).saturating_sub(1)
    } else {
        capacity::limit(capacity::installed_ram())
    };
    let booted = owned
        .into_iter()
        .filter(|device| device.booted && device.udid != udid)
        .map(|device| Booted {
            opened: opened.get(&device.name).copied(),
            busy: busy.contains(&device.name),
            udid: device.udid,
        })
        .collect();
    simulator::shut_down_all(DeviceSet::Run, capacity::to_shut_down(booted, keep));
}

struct Build {
    builds: Builds,
    worktree: PathBuf,
    id: u64,
    /// Install and launch the app after the build.
    run: bool,
    /// Install and launch the app on the worktree's preview simulator after the build.
    preview: bool,
    destination: Option<RunDestination>,
    pick: SchemePick,
    kept_services: BTreeMap<String, bool>,
    project: Project,
    sink: Box<dyn Fn(BuildEvent) + Send + Sync>,
}

impl Build {
    fn run(&self) {
        let end = if self.preview {
            self.preview_outcome()
        } else {
            self.outcome()
        };
        self.say(end);
        lock(&self.builds.0.slots).finished(&self.worktree, self.id);
        self.builds.follow_with_preview(&self.worktree);
    }

    fn app_launched(&self) {
        lock(&self.builds.0.slots).app_launched(&self.worktree, self.id);
        self.say(BuildEvent::Phase {
            phase: BuildPhase::Running,
        });
        self.builds.follow_with_preview(&self.worktree);
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
        let target = match self.target() {
            Ok(target) => target,
            Err(_) if self.stopped() => return BuildEvent::Stopped,
            Err(error) => return self.failed(error.to_string()),
        };
        // The first boot hides inside the build.
        let boot = match &target {
            Target::Simulator(device) if self.run => {
                let device = device.clone();
                let (opened, busy) = self.builds.opened_and_running();
                let kept_services = self.kept_services.clone();
                Some(thread::spawn(move || {
                    make_room(&device.udid, &opened, &busy);
                    let booted = simulator::boot_slim(&device, &kept_services);
                    if booted.is_ok() && !device.booted {
                        simulator::warm_up(DeviceSet::Run, &device.udid);
                    }
                    booted
                }))
            }
            _ => None,
        };
        self.say(BuildEvent::Phase {
            phase: BuildPhase::Building,
        });
        let end = self.xcodebuild(&scheme, target.id());
        if end == BuildEvent::Succeeded
            && lock(&self.builds.0.previews).contains_key(&self.worktree)
        {
            lock(&self.builds.0.slots).follow_with_preview(&self.worktree);
        }
        match (end, target, boot) {
            (BuildEvent::Succeeded, Target::Simulator(device), Some(boot)) => {
                self.run_app(&scheme, &device, boot)
            }
            (BuildEvent::Succeeded, Target::Device(udid), _) if self.run => {
                self.run_on_device(&scheme, &udid)
            }
            (end, _, _) => end,
        }
    }

    /// What the build targets: a device; the worktree's own simulator, created in the destination's model; a build without a run that can create none builds for any simulator.
    fn target(&self) -> Result<Target, Error> {
        if let Some(RunDestination::Device { id, .. }) = &self.destination {
            return Ok(Target::Device(id.clone()));
        }
        if !self.run {
            return Ok(self
                .build_simulator()
                .map_or(Target::AnySimulator, Target::Simulator));
        }
        let _lookup = lock(&self.builds.0.simulators);
        self.step(|register| {
            simulator::find_or_create(DeviceSet::Run, &self.worktree, self.wanted(), register)
        })
        .map(Target::Simulator)
    }

    /// The destination's model and runtime identifiers.
    fn wanted(&self) -> Option<(&str, &str)> {
        match &self.destination {
            Some(RunDestination::Simulator { model, runtime, .. }) => {
                Some((model.as_str(), runtime.as_str()))
            }
            _ => None,
        }
    }

    /// The worktree's run simulator for a build that boots none, created shut down when the worktree has none: a build for any simulator compiles every architecture and differs from ⌘R's. It leaves the simulator that exists as it is, whatever its model.
    fn build_simulator(&self) -> Option<Simulator> {
        let _lookup = lock(&self.builds.0.simulators);
        if let Ok(Some(device)) = simulator::find(DeviceSet::Run, &self.worktree) {
            return Some(device);
        }
        self.step(|register| {
            simulator::find_or_create(DeviceSet::Run, &self.worktree, self.wanted(), register)
        })
        .ok()
    }

    /// Runs `step` with the way to register its process group in the slot, so a stop ends it.
    fn step<T>(&self, step: impl FnOnce(&dyn Fn(u32)) -> Result<T, Error>) -> Result<T, Error> {
        let result = step(&|pid| self.register(pid));
        lock(&self.builds.0.slots).clear_pid(&self.worktree, self.id);
        result
    }

    /// After a successful build: waits for the boot, installs the app and launches it, and returns when the app ends.
    fn run_app(
        &self,
        scheme: &str,
        device: &Simulator,
        boot: thread::JoinHandle<Result<Option<&'static str>, Error>>,
    ) -> BuildEvent {
        self.say(BuildEvent::Phase {
            phase: BuildPhase::Installing,
        });
        while !boot.is_finished() {
            if self.stopped() {
                return BuildEvent::Stopped;
            }
            thread::sleep(POLL);
        }
        let booted = boot
            .join()
            .unwrap_or_else(|_| Err(Error::Message("The simulator's boot crashed.".into())));
        let udid = device.udid.as_str();
        let mut bundle = None;
        let ended = booted.and_then(|slimming_note| {
            if let Some(note) = slimming_note {
                self.log(format!("{note}\n"));
            }
            if self.stopped() {
                return Ok(None);
            }
            // The Simulator view shows the device; the external app only stands in when it cannot.
            if simulator_kit::load().is_err() {
                let _ = simulator::open_app(false);
            }
            let app = self.step(|register| {
                xcode::built_app(
                    &self.project,
                    scheme,
                    &self.configuration(scheme),
                    Some(udid),
                    &self.worktree,
                    register,
                )
            })?;
            let id =
                self.step(|register| simulator::plist_text(&app, "CFBundleIdentifier", register))?;
            let executable =
                self.step(|register| simulator::plist_text(&app, "CFBundleExecutable", register))?;
            self.step(|register| simulator::install(DeviceSet::Run, udid, &app, register))?;
            if self.stopped() {
                return Ok(None);
            }
            self.say(BuildEvent::Phase {
                phase: BuildPhase::Launching,
            });
            let id = bundle.insert(id);
            self.step(|register| {
                simulator::launch(
                    udid,
                    id,
                    &executable,
                    register,
                    &|| self.app_launched(),
                    &|text, tone| self.say(BuildEvent::Console { text, tone }),
                )
            })
        });
        self.finished_run(ended, |stop_requested| {
            if stop_requested && let Some(bundle) = &bundle {
                simulator::terminate(udid, bundle);
            }
        })
    }

    /// After a successful build for a device: installs the app and launches it, and returns when the app ends. A stop ends the console, and `devicectl` ends the app with it.
    fn run_on_device(&self, scheme: &str, udid: &str) -> BuildEvent {
        let ended = (|| {
            if self.stopped() {
                return Ok(None);
            }
            self.say(BuildEvent::Phase {
                phase: BuildPhase::Installing,
            });
            let app = self.step(|register| {
                xcode::built_app(
                    &self.project,
                    scheme,
                    &self.configuration(scheme),
                    Some(udid),
                    &self.worktree,
                    register,
                )
            })?;
            let id =
                self.step(|register| simulator::plist_text(&app, "CFBundleIdentifier", register))?;
            self.step(|register| device::install(udid, &app, register))?;
            if self.stopped() {
                return Ok(None);
            }
            self.say(BuildEvent::Phase {
                phase: BuildPhase::Launching,
            });
            self.step(|register| {
                device::launch(
                    udid,
                    &id,
                    register,
                    &|| self.app_launched(),
                    &|text, tone| self.say(BuildEvent::Console { text, tone }),
                )
            })
        })();
        self.finished_run(ended, drop)
    }

    /// The end of a run: Stopped (`on_stop` hears whether a stop reached the run, the only case that ends its app), Exited with the app's exit code on the console, or Failed.
    fn finished_run(
        &self,
        ended: Result<Option<i32>, Error>,
        on_stop: impl FnOnce(bool),
    ) -> BuildEvent {
        if self.stopped() {
            on_stop(lock(&self.builds.0.slots).take_stop_request(self.id));
            return BuildEvent::Stopped;
        }
        match ended {
            Ok(code) => {
                let how = code.map_or("a signal".into(), |code| format!("code {code}"));
                self.say(BuildEvent::Console {
                    text: format!("Process exited with {how}\n"),
                    tone: ConsoleTone::Own,
                });
                BuildEvent::Exited
            }
            Err(error) => self.failed(error.to_string()),
        }
    }

    fn failed(&self, message: String) -> BuildEvent {
        self.log(format!("{message}\n"));
        failure(message)
    }

    fn scheme(&self) -> Result<String, Error> {
        if let Some(scheme) = &self.pick.scheme {
            return Ok(scheme.clone());
        }
        if let Some(scheme) = lock(&self.builds.0.schemes).get(&self.project.path) {
            return Ok(scheme.clone());
        }
        let scheme =
            self.step(|register| xcode::scheme_of(&self.project, &self.worktree, register))?;
        lock(&self.builds.0.schemes).insert(self.project.path.clone(), scheme.clone());
        Ok(scheme)
    }

    /// The picked configuration, else the one the scheme runs in.
    fn configuration(&self, scheme: &str) -> String {
        self.pick
            .configuration
            .clone()
            .unwrap_or_else(|| xcode::run_configuration(&self.worktree, scheme))
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

    fn xcodebuild(&self, scheme: &str, target: Option<&str>) -> BuildEvent {
        let mut args =
            xcode::build_args(&self.project, scheme, &self.configuration(scheme), target);
        args.push("build".into());
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut command = login_shell("xcodebuild", &arg_refs);
        command.current_dir(&self.worktree);
        self.log(format!("$ xcodebuild {}\n", args.join(" ")));
        let (mut child, reader) = match process::merged(command, None) {
            Ok(started) => started,
            Err(error) => return self.failed(format!("xcodebuild did not start: {error}")),
        };
        self.register(child.id());

        let received = process::read_lines(reader);
        let mut issues = Issues::default();
        let mut drivers = DriverLines::default();
        let mut batch = String::new();
        let mut flushed = Instant::now();
        loop {
            match received.recv_timeout(BATCH_INTERVAL) {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    issues.push_line(&text);
                    drivers.push(&text);
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
        if self.pick == SchemePick::default() {
            drivers.record(&self.worktree);
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
