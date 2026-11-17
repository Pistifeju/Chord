//! One build per worktree: its xcodebuild with its arguments, its own process group and its log in batches, the wait while a build Chord did not start holds the build database, the run of the app on the run destination after it with its console, the preview build that follows it onto the preview simulator, the hot reload or preview build a save starts, and the stop that signals the whole group.

use chord_core::lock;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use chord_core::{Error, PreviewPick, RunDestination, SchemePick, login_shell};
use specta::Type;

use crate::capacity::{self, Booted};
use crate::compile_commands::DriverLines;
use crate::console::ConsoleTone;
use crate::derived_data;
use crate::device;
use crate::hot_reload;
use crate::issues::{BuildIssue, Issues};
use crate::preview::{self, PreviewState};
use crate::preview_list;
use crate::process::{self, BATCH_BYTES, BATCH_INTERVAL, GROUP_GRACE, end_group};
use crate::simulator::{self, DeviceSet, Simulator};
use crate::simulator_kit;
use crate::slot::{Slots, Start};
use crate::worktree_saves::{self, Saves};
use crate::xcode::{self, Project};

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

/// A worktree whose Preview view opened, with its preview simulator.
struct Preview {
    udid: String,
    /// The preview app launched on the simulator since its boot.
    launched: bool,
    /// The view shows: saves reload or rebuild the preview.
    open: bool,
    /// A save landed while the view was closed: the app runs old code.
    missed: bool,
    /// The `#Preview` the next launch shows.
    pick: Option<PreviewPick>,
    on_state: Arc<dyn Fn(PreviewState) + Send + Sync>,
}

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

    /// The Preview view opened: boots the worktree's preview simulator of the destination's model, unless two others run, and starts a preview build of `pick` unless the preview app already runs there. `on_state` hears the preview's states from then on, from any thread. Blocks during the boot.
    pub fn open_preview(
        &self,
        worktree: PathBuf,
        destination: Option<RunDestination>,
        pick: Option<PreviewPick>,
        on_state: impl Fn(PreviewState) + Send + Sync + 'static,
    ) {
        let on_state: Arc<dyn Fn(PreviewState) + Send + Sync> = Arc::new(on_state);
        // No simulator boots for nothing to show; the first pick opens the view again.
        if pick.is_none() {
            return on_state(PreviewState::Unpicked);
        }
        if let Err(error) = self.project_of(&worktree) {
            return on_state(PreviewState::Failed {
                message: error.to_string(),
            });
        }
        let wanted = match &destination {
            Some(RunDestination::Simulator { model, runtime, .. }) => {
                Some((model.as_str(), runtime.as_str()))
            }
            _ => None,
        };
        let (udid, fresh) =
            match preview::boot(&worktree, wanted, || on_state(PreviewState::Starting)) {
                Ok(preview::Booted::Ready { udid, fresh }) => (udid, fresh),
                Ok(preview::Booted::Full(worktrees)) => {
                    lock(&self.0.previews).remove(&worktree);
                    return on_state(PreviewState::Full { worktrees });
                }
                Err(error) => {
                    return on_state(PreviewState::Failed {
                        message: error.to_string(),
                    });
                }
            };
        let (launched, picked) = {
            let mut previews = lock(&self.0.previews);
            let before = previews.get(&worktree);
            let launched = !fresh
                && before.is_some_and(|preview| {
                    preview.udid == udid && preview.launched && !preview.missed
                });
            let picked = before.is_some_and(|preview| preview.pick != pick);
            previews.insert(
                worktree.clone(),
                Preview {
                    udid,
                    launched,
                    open: true,
                    missed: false,
                    pick,
                    on_state: Arc::clone(&on_state),
                },
            );
            (launched, picked)
        };
        self.watch(&worktree);
        let (runs, waits) = {
            let slots = lock(&self.0.slots);
            (
                slots.preview_runs(&worktree),
                slots.preview_waits(&worktree),
            )
        };
        if launched && picked {
            self.show_pick(&worktree);
        } else if launched {
            on_state(PreviewState::Live);
        } else if waits {
            on_state(PreviewState::Waiting);
        } else if runs {
            on_state(PreviewState::Building);
        } else {
            self.start_preview(&worktree);
        }
    }

    /// The Preview view picked another `#Preview`: the preview app shows it at once, or the next launch does.
    pub fn pick_preview(&self, worktree: &Path, pick: PreviewPick) {
        match lock(&self.0.previews).get_mut(worktree) {
            Some(preview) => preview.pick = Some(pick),
            None => return,
        }
        self.show_pick(worktree);
    }

    /// Shows the pick in the running preview app with the saves since its launch, else rebuilds; a preview build that runs or waits launches with it anyway.
    fn show_pick(&self, worktree: &Path) {
        let (launched, pick) = match lock(&self.0.previews).get(worktree) {
            Some(preview) => (preview.launched, preview.pick.clone()),
            None => return,
        };
        {
            let mut slots = lock(&self.0.slots);
            if slots.holds_preview(worktree) {
                // A build that fails or launched with the old pick still leaves a preview build to follow.
                return slots.follow_with_preview(worktree);
            }
        }
        let shown = pick.filter(|_| launched).and_then(|pick| {
            preview::show(worktree, &pick)
                .and_then(|()| hot_reload::reload(worktree))
                .ok()
        });
        match shown {
            Some(None) => self.tell_preview(worktree, PreviewState::Live),
            Some(Some(issue)) => self.tell_preview(worktree, PreviewState::Error { issue }),
            None => self.start_preview(worktree),
        }
    }

    fn watch(&self, worktree: &Path) {
        let mut watchers = lock(&self.0.watchers);
        if watchers.contains_key(worktree) {
            return;
        }
        let (builds, path) = (self.clone(), worktree.to_path_buf());
        // Without a watcher the preview still works; it catches up on the next build.
        if let Ok(watcher) =
            worktree_saves::watch(worktree, move |saves| builds.saved(&path, saves))
        {
            watchers.insert(worktree.to_path_buf(), watcher);
        }
    }

    /// The Preview view closed: saves no longer reload or rebuild its preview until it opens again.
    pub fn close_preview(&self, worktree: &Path) {
        if let Some(preview) = lock(&self.0.previews).get_mut(worktree) {
            preview.open = false;
        }
    }

    /// A hot reload when only `.swift` files changed and the preview app runs with no build on, else a preview build. A closed view's next open rebuilds instead.
    fn saved(&self, worktree: &Path, saves: Saves) {
        hot_reload::saved(worktree, saves.swift);
        let launched = match lock(&self.0.previews).get_mut(worktree) {
            Some(preview) if !preview.open => {
                preview.missed = true;
                return;
            }
            preview => preview.is_some_and(|preview| preview.launched),
        };
        let busy = lock(&self.0.slots).holds_preview(worktree);
        if saves.other || !launched || busy {
            return self.start_preview(worktree);
        }
        match hot_reload::reload(worktree) {
            Ok(None) => self.tell_preview(worktree, PreviewState::Live),
            Ok(Some(issue)) => self.tell_preview(worktree, PreviewState::Error { issue }),
            Err(_) => self.start_preview(worktree),
        }
    }

    /// Tells the worktree's Preview view, the one that opened last.
    fn tell_preview(&self, worktree: &Path, state: PreviewState) {
        let on_state = lock(&self.0.previews)
            .get(worktree)
            .map(|preview| Arc::clone(&preview.on_state));
        if let Some(on_state) = on_state {
            on_state(state);
        }
    }

    /// A preview build of a worktree whose Preview view opened: it waits for the worktree's ⌘R or ⌘B build, else takes over from its preview build.
    fn start_preview(&self, worktree: &Path) {
        let pick = match lock(&self.0.previews).get(worktree) {
            Some(preview) => preview.pick.clone(),
            None => return,
        };
        if !pick.is_some_and(|pick| preview_list::holds(worktree, &pick)) {
            if let Some(preview) = lock(&self.0.previews).get_mut(worktree) {
                preview.launched = false;
            }
            return self.tell_preview(worktree, PreviewState::Unpicked);
        }
        let project = match self.project_of(worktree) {
            Ok(project) => project,
            Err(error) => {
                let message = error.to_string();
                return self.tell_preview(worktree, PreviewState::Failed { message });
            }
        };
        let Some(start) = lock(&self.0.slots).ask_preview(worktree) else {
            return self.tell_preview(worktree, PreviewState::Waiting);
        };
        let (builds, path) = (self.clone(), worktree.to_path_buf());
        let on_state = move |state| builds.tell_preview(&path, state);
        // The preview build writes no log; its states stand in for the phases.
        let sink = move |event| match event {
            BuildEvent::Phase {
                phase: BuildPhase::Waiting,
            } => on_state(PreviewState::Waiting),
            BuildEvent::Phase {
                phase: BuildPhase::Building,
            } => on_state(PreviewState::Building),
            BuildEvent::Phase {
                phase: BuildPhase::Running,
            } => on_state(PreviewState::Live),
            BuildEvent::Failed { issues, .. } => on_state(match issues.into_iter().next() {
                Some(issue) => PreviewState::Error { issue },
                None => PreviewState::Failed {
                    message: String::new(),
                },
            }),
            _ => {}
        };
        self.spawn(
            start,
            Lane::Preview,
            Build {
                builds: self.clone(),
                worktree: worktree.to_path_buf(),
                id: 0,
                run: false,
                preview: true,
                destination: None,
                // The preview builds the default scheme in its default configuration, whose compile commands the hot reload reuses.
                pick: SchemePick::default(),
                kept_services: BTreeMap::new(),
                project,
                sink: Box::new(sink),
            },
        );
    }

    /// Starts the worktree's preview build if one waits and no ⌘R or ⌘B build holds the worktree any longer.
    fn follow_with_preview(&self, worktree: &Path) {
        let waits = lock(&self.0.slots).take_waiting_preview(worktree);
        if waits {
            self.start_preview(worktree);
        }
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

    /// Records that the user opened the worktree in the app.
    pub fn opened(&self, worktree: &Path) {
        lock(&self.0.opened).insert(simulator::name_of(worktree), Instant::now());
    }

    /// When each worktree's simulator (by name) was last opened, and the names of those whose app runs.
    fn opened_and_running(&self) -> (HashMap<String, Instant>, HashSet<String>) {
        let busy = lock(&self.0.slots)
            .running_worktrees()
            .iter()
            .map(|worktree| simulator::name_of(worktree))
            .collect();
        (lock(&self.0.opened).clone(), busy)
    }

    /// For a worktree that was deleted: stops its build, waits for the build database to be released, forgets its schemes.
    pub fn forget(&self, worktree: &Path) {
        let pids = {
            let mut slots = lock(&self.0.slots);
            let mut pids = slots.stop(worktree);
            pids.extend(slots.forget_preview(worktree));
            pids
        };
        let ending: Vec<_> = pids
            .into_iter()
            .map(|pid| thread::spawn(move || end_group(pid)))
            .collect();
        for handle in ending {
            let _ = handle.join();
        }
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
                    simulator::boot_slim(&device, &kept_services)
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

    /// A preview build: builds like ⌘B for a simulator, then launches the app on the preview simulator. One whose preview simulator was shut down meanwhile builds nothing.
    fn preview_outcome(&self) -> BuildEvent {
        let udid = lock(&self.builds.0.previews)
            .get(&self.worktree)
            .map(|preview| preview.udid.clone());
        let Some(udid) = udid.filter(|udid| simulator::is_booted(DeviceSet::Preview, udid)) else {
            self.set_launched(false);
            return BuildEvent::Stopped;
        };
        self.set_launched(false);
        if self.stopped() {
            return BuildEvent::Stopped;
        }
        self.say(BuildEvent::Phase {
            phase: BuildPhase::Building,
        });
        let scheme = match self.scheme() {
            Ok(scheme) => scheme,
            Err(_) if self.stopped() => return BuildEvent::Stopped,
            Err(error) => return self.failed(error.to_string()),
        };
        if !self.wait_for_database() {
            return BuildEvent::Stopped;
        }
        // The worktree's run simulator, so ⌘R and the preview build the same thing.
        let target = simulator::find(DeviceSet::Run, &self.worktree)
            .ok()
            .flatten()
            .map(|device| device.udid);
        match self.xcodebuild(&scheme, target.as_deref()) {
            BuildEvent::Succeeded => {}
            end => return end,
        }
        let launched = self
            .step(|register| {
                xcode::built_app(
                    &self.project,
                    &scheme,
                    &self.configuration(&scheme),
                    target.as_deref(),
                    &self.worktree,
                    register,
                )
            })
            .and_then(|app| {
                self.step(|register| preview::launch(&self.worktree, &udid, &app, register))
            })
            .and_then(|()| {
                // The pick after launch: a pick made during the build shows without another one.
                let pick = lock(&self.builds.0.previews)
                    .get(&self.worktree)
                    .and_then(|preview| preview.pick.clone());
                pick.map_or(Ok(()), |pick| {
                    preview::show(&self.worktree, &pick).map_err(Error::Message)
                })
            });
        if self.stopped() {
            return BuildEvent::Stopped;
        }
        match launched {
            Ok(()) => {
                self.set_launched(true);
                self.say(BuildEvent::Phase {
                    phase: BuildPhase::Running,
                });
                BuildEvent::Succeeded
            }
            Err(error) => self.failed(error.to_string()),
        }
    }

    fn set_launched(&self, launched: bool) {
        if let Some(preview) = lock(&self.builds.0.previews).get_mut(&self.worktree) {
            preview.launched = launched;
        }
    }

    /// What the build targets: a device; the worktree's own simulator, created by a run in the destination's model; a build that finds none builds for any simulator.
    fn target(&self) -> Result<Target, Error> {
        if let Some(RunDestination::Device { id, .. }) = &self.destination {
            return Ok(Target::Device(id.clone()));
        }
        let _lookup = lock(&self.builds.0.simulators);
        if !self.run {
            return Ok(simulator::find(DeviceSet::Run, &self.worktree)
                .ok()
                .flatten()
                .map_or(Target::AnySimulator, Target::Simulator));
        }
        let wanted = match &self.destination {
            Some(RunDestination::Simulator { model, runtime, .. }) => {
                Some((model.as_str(), runtime.as_str()))
            }
            _ => None,
        };
        self.step(|register| {
            simulator::find_or_create(DeviceSet::Run, &self.worktree, wanted, register)
        })
        .map(Target::Simulator)
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
