use chord_core::lock;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::thread;

use chord_core::{Error, PreviewPick, RunDestination, SchemePick};

use super::{Build, BuildEvent, BuildPhase, Builds, Lane, POLL};
use crate::hot_reload;
use crate::preview::{self, Platform, PreviewState};
use crate::preview_list;
use crate::process::end_group;
use crate::screen_view;
use crate::simulator::{self, DeviceSet};
use crate::worktree_saves::{self, Saves};
use crate::xcode::{self, MacSupport};

#[derive(Debug, PartialEq)]
enum OnOpen {
    ShowPick,
    Live,
    Waiting,
    Building,
    StartBuild,
}

fn on_open(launched: bool, picked: bool, waits: bool, runs: bool) -> OnOpen {
    match (launched, picked) {
        (true, true) => OnOpen::ShowPick,
        (true, false) => OnOpen::Live,
        _ if waits => OnOpen::Waiting,
        _ if runs => OnOpen::Building,
        _ => OnOpen::StartBuild,
    }
}

#[derive(Clone)]
enum Host {
    Simulator { udid: String, boot: preview::Boot },
    Mac,
}

impl Host {
    fn same_device(&self, other: &Host) -> bool {
        match (self, other) {
            (Host::Simulator { udid, .. }, Host::Simulator { udid: other, .. }) => udid == other,
            (Host::Mac, Host::Mac) => true,
            _ => false,
        }
    }

    fn boot(&self) -> Option<&preview::Boot> {
        match self {
            Host::Simulator { boot, .. } => Some(boot),
            Host::Mac => None,
        }
    }

    fn shut_down(&self) -> bool {
        match self {
            Host::Simulator { udid, .. } => !simulator::is_booted(DeviceSet::Preview, udid),
            Host::Mac => false,
        }
    }
}

fn platform(mac: MacSupport, model: Option<&RunDestination>) -> Platform {
    match (mac, model) {
        (MacSupport::Only, _) | (MacSupport::Also, Some(RunDestination::Mac { .. })) => {
            Platform::Mac
        }
        _ => Platform::Simulator,
    }
}

pub(super) struct Preview {
    host: Host,
    launched: bool,
    launch: Option<u64>,
    ended: Option<String>,
    open: bool,
    missed: bool,
    pick: Option<PreviewPick>,
    on_state: Arc<dyn Fn(PreviewState) + Send + Sync>,
}

impl Builds {
    pub fn open_preview(
        &self,
        worktree: PathBuf,
        destination: Option<RunDestination>,
        pick: Option<PreviewPick>,
        on_state: impl Fn(PreviewState) + Send + Sync + 'static,
    ) {
        let on_state: Arc<dyn Fn(PreviewState) + Send + Sync> = Arc::new(on_state);
        if pick.is_none() {
            return on_state(PreviewState::Unpicked);
        }
        if let Err(error) = self.project_of(&worktree) {
            return on_state(PreviewState::Failed {
                message: error.to_string(),
            });
        }
        let platform = platform(xcode::mac_support(&worktree), destination.as_ref());
        screen_view::host_on_mac(&worktree, platform == Platform::Mac);
        let (host, fresh) = match platform {
            Platform::Mac => (Host::Mac, false),
            Platform::Simulator => {
                let wanted = match &destination {
                    Some(RunDestination::Simulator { model, runtime, .. }) => {
                        Some((model.as_str(), runtime.as_str()))
                    }
                    _ => None,
                };
                let open: HashSet<String> = lock(&self.0.previews)
                    .iter()
                    .filter(|(_, preview)| preview.open)
                    .map(|(path, _)| simulator::name_of(path))
                    .collect();
                match preview::boot(&worktree, wanted, open, Arc::clone(&on_state)) {
                    Ok(preview::Booted::Ready { udid }) => {
                        let boot = Arc::new(OnceLock::from(Ok(())));
                        (Host::Simulator { udid, boot }, false)
                    }
                    Ok(preview::Booted::Booting { udid, boot }) => {
                        (Host::Simulator { udid, boot }, true)
                    }
                    Ok(preview::Booted::Full(worktrees)) => {
                        lock(&self.0.previews).remove(&worktree);
                        return on_state(PreviewState::Full { worktrees });
                    }
                    Err(error) => {
                        return on_state(PreviewState::Failed {
                            message: error.to_string(),
                        });
                    }
                }
            }
        };
        let boot_outcome = host.boot().cloned();
        let (launched, picked) = {
            let mut previews = lock(&self.0.previews);
            let before = previews.get(&worktree);
            let launch = before.and_then(|preview| preview.launch);
            let launched = !fresh
                && before.is_some_and(|preview| {
                    preview.host.same_device(&host) && preview.launched && !preview.missed
                });
            let picked = before.is_some_and(|preview| preview.pick != pick);
            previews.insert(
                worktree.clone(),
                Preview {
                    host,
                    launched,
                    launch,
                    ended: None,
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
        match on_open(launched, picked, waits, runs) {
            OnOpen::ShowPick => self.show_pick(&worktree),
            OnOpen::Live => on_state(PreviewState::Live),
            OnOpen::Waiting => on_state(PreviewState::Waiting),
            OnOpen::Building => on_state(PreviewState::Building),
            OnOpen::StartBuild => self.start_preview(&worktree),
        }
        if let Some(Err(message)) = boot_outcome.as_ref().and_then(|boot| boot.get()) {
            on_state(PreviewState::Failed {
                message: message.clone(),
            });
        }
    }

    pub fn pick_preview(&self, worktree: &Path, pick: PreviewPick) {
        match lock(&self.0.previews).get_mut(worktree) {
            Some(preview) => preview.pick = Some(pick),
            None => return,
        }
        self.show_pick(worktree);
    }

    fn show_pick(&self, worktree: &Path) {
        let (launched, pick) = match lock(&self.0.previews).get(worktree) {
            Some(preview) => (preview.launched, preview.pick.clone()),
            None => return,
        };
        {
            let mut slots = lock(&self.0.slots);
            if slots.holds_preview(worktree) {
                return slots.follow_with_preview(worktree);
            }
        }
        let shown = pick.filter(|_| launched).and_then(|pick| {
            preview::show(worktree, &pick)
                .and_then(|()| hot_reload::reload(worktree))
                .ok()
        });
        match shown {
            Some(state) => self.tell_preview(worktree, state),
            None => self.start_preview(worktree),
        }
    }

    fn watch(&self, worktree: &Path) {
        let mut watchers = lock(&self.0.watchers);
        if watchers.contains_key(worktree) {
            return;
        }
        let (builds, path) = (self.clone(), worktree.to_path_buf());
        if let Ok(watcher) =
            worktree_saves::watch(worktree, move |saves| builds.saved(&path, saves))
        {
            watchers.insert(worktree.to_path_buf(), watcher);
        }
    }

    pub fn close_preview(&self, worktree: &Path) {
        if let Some(preview) = lock(&self.0.previews).get_mut(worktree) {
            preview.open = false;
        }
    }

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
            Ok(state) => self.tell_preview(worktree, state),
            Err(_) => self.start_preview(worktree),
        }
    }

    pub(super) fn tell_preview(&self, worktree: &Path, state: PreviewState) {
        let on_state = lock(&self.0.previews)
            .get(worktree)
            .map(|preview| Arc::clone(&preview.on_state));
        if let Some(on_state) = on_state {
            on_state(state);
        }
    }

    fn start_preview(&self, worktree: &Path) {
        let pick = match lock(&self.0.previews).get_mut(worktree) {
            Some(preview) if !preview.open => {
                preview.missed = true;
                return;
            }
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
                pick: SchemePick::default(),
                kept_services: BTreeMap::new(),
                project,
                sink: Box::new(sink),
            },
        );
    }

    fn preview_ended(&self, worktree: &Path, launch: u64, host: &Host, message: String) {
        match lock(&self.0.previews).get_mut(worktree) {
            Some(preview) if preview.launch == Some(launch) && preview.launched => {
                preview.launched = false;
            }
            Some(preview) if preview.launch == Some(launch) => {
                preview.ended = Some(message);
                return;
            }
            _ => return,
        }
        if !host.shut_down() {
            self.tell_preview(worktree, PreviewState::Failed { message });
        }
    }

    pub(super) fn follow_with_preview(&self, worktree: &Path) {
        let waits = lock(&self.0.slots).take_waiting_preview(worktree);
        if waits {
            self.start_preview(worktree);
        }
    }
}

impl Build {
    pub(super) fn preview_outcome(&self) -> BuildEvent {
        let found = lock(&self.builds.0.previews)
            .get(&self.worktree)
            .map(|preview| preview.host.clone());
        let Some(host) = found.filter(|host| {
            !matches!(host.boot().and_then(|boot| boot.get()), Some(Ok(()))) || !host.shut_down()
        }) else {
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
        if let Some(Err(message)) = host.boot().and_then(|boot| boot.get()) {
            let message = message.clone();
            self.builds
                .tell_preview(&self.worktree, PreviewState::Failed { message });
            return BuildEvent::Stopped;
        }
        let scheme = match self.scheme() {
            Ok(scheme) => scheme,
            Err(_) if self.stopped() => return BuildEvent::Stopped,
            Err(error) => return self.failed(error.to_string()),
        };
        if !self.wait_for_database() {
            return BuildEvent::Stopped;
        }
        let destination = match &host {
            Host::Simulator { .. } => {
                xcode::destination(self.build_simulator().map(|device| device.udid).as_deref())
            }
            Host::Mac => xcode::mac_destination(),
        };
        match self.xcodebuild(&scheme, &destination) {
            BuildEvent::Succeeded => {}
            end => return end,
        }
        if let Some(boot) = host.boot() {
            let booted = loop {
                if self.stopped() {
                    return BuildEvent::Stopped;
                }
                match boot.get() {
                    Some(booted) => break booted.clone(),
                    None => thread::sleep(POLL),
                }
            };
            if let Err(message) = booted {
                self.builds
                    .tell_preview(&self.worktree, PreviewState::Failed { message });
                return BuildEvent::Stopped;
            }
        }
        let launched = self
            .step(|register| {
                xcode::built_app(
                    &self.project,
                    &scheme,
                    &self.configuration(&scheme),
                    &destination,
                    &self.worktree,
                    register,
                )
            })
            .and_then(|app| {
                if let Some(preview) = lock(&self.builds.0.previews).get_mut(&self.worktree) {
                    preview.launch = Some(self.id);
                    preview.ended = None;
                }
                if let Some(pid) = preview::take_mac(&self.worktree) {
                    thread::spawn(move || end_group(pid));
                }
                let (builds, worktree, id, ended) = (
                    self.builds.clone(),
                    self.worktree.clone(),
                    self.id,
                    host.clone(),
                );
                let on_exit = move |message| builds.preview_ended(&worktree, id, &ended, message);
                self.step(|register| match &host {
                    Host::Simulator { udid, .. } => {
                        preview::launch(&self.worktree, udid, &app, register, on_exit)
                    }
                    Host::Mac => preview::launch_mac(&self.worktree, &app, register, on_exit),
                })
            })
            .and_then(|()| {
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
                if let Some(message) = self.mark_launched() {
                    return self.failed(message);
                }
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

    fn mark_launched(&self) -> Option<String> {
        let mut previews = lock(&self.builds.0.previews);
        let preview = previews.get_mut(&self.worktree)?;
        let ended = preview.ended.take();
        preview.launched = ended.is_none();
        ended
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opened_preview_shows_a_new_pick_stays_live_or_waits_for_its_build() {
        assert_eq!(on_open(true, true, true, true), OnOpen::ShowPick);
        assert_eq!(on_open(true, false, true, true), OnOpen::Live);
        assert_eq!(on_open(false, true, true, true), OnOpen::Waiting);
        assert_eq!(on_open(false, false, false, true), OnOpen::Building);
        assert_eq!(on_open(false, true, false, false), OnOpen::StartBuild);
    }

    #[test]
    fn a_mac_only_project_or_a_my_mac_model_previews_on_the_mac() {
        let mac = RunDestination::Mac {
            name: "My Mac".into(),
        };
        let iphone = RunDestination::Simulator {
            model: "iPhone 17".into(),
            runtime: "iOS 26.0".into(),
            name: "iPhone 17".into(),
            os: "iOS 26.0".into(),
        };
        assert_eq!(platform(MacSupport::Only, None), Platform::Mac);
        assert_eq!(platform(MacSupport::Only, Some(&iphone)), Platform::Mac);
        assert_eq!(platform(MacSupport::Also, Some(&mac)), Platform::Mac);
        assert_eq!(
            platform(MacSupport::Also, Some(&iphone)),
            Platform::Simulator
        );
        assert_eq!(platform(MacSupport::Also, None), Platform::Simulator);
        assert_eq!(platform(MacSupport::None, Some(&mac)), Platform::Simulator);
    }
}
