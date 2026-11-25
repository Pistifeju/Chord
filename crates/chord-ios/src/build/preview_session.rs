use chord_core::lock;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::thread;

use chord_core::{Error, PreviewPick, RunDestination, SchemePick};

use super::{Build, BuildEvent, BuildPhase, Builds, Lane, POLL};
use crate::hot_reload;
use crate::preview::{self, PreviewState};
use crate::preview_list;
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

pub(super) struct Preview {
    udid: String,
    boot: preview::Boot,
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
        if xcode::mac_support(&worktree) == MacSupport::Only {
            return on_state(PreviewState::Failed {
                message:
                    "Preview runs on an iOS simulator, and this project builds for macOS only."
                        .into(),
            });
        }
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
        let (udid, boot, fresh) =
            match preview::boot(&worktree, wanted, open, Arc::clone(&on_state)) {
                Ok(preview::Booted::Ready { udid }) => {
                    (udid, Arc::new(OnceLock::from(Ok(()))), false)
                }
                Ok(preview::Booted::Booting { udid, boot }) => (udid, boot, true),
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
        let boot_outcome = Arc::clone(&boot);
        let (launched, picked) = {
            let mut previews = lock(&self.0.previews);
            let before = previews.get(&worktree);
            let launch = before.and_then(|preview| preview.launch);
            let launched = !fresh
                && before.is_some_and(|preview| {
                    preview.udid == udid && preview.launched && !preview.missed
                });
            let picked = before.is_some_and(|preview| preview.pick != pick);
            previews.insert(
                worktree.clone(),
                Preview {
                    udid,
                    boot,
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
        if let Some(Err(message)) = boot_outcome.get() {
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
            Ok(None) => self.tell_preview(worktree, PreviewState::Live),
            Ok(Some(issue)) => self.tell_preview(worktree, PreviewState::Error { issue }),
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

    fn preview_ended(&self, worktree: &Path, launch: u64, udid: &str, message: String) {
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
        if simulator::is_booted(DeviceSet::Preview, udid) {
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
            .map(|preview| (preview.udid.clone(), Arc::clone(&preview.boot)));
        let Some((udid, boot)) = found.filter(|(udid, boot)| match boot.get() {
            Some(Ok(())) => simulator::is_booted(DeviceSet::Preview, udid),
            None | Some(Err(_)) => true,
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
        if let Some(Err(message)) = boot.get() {
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
        let target = self.build_simulator().map(|device| device.udid);
        match self.xcodebuild(&scheme, &xcode::destination(target.as_deref())) {
            BuildEvent::Succeeded => {}
            end => return end,
        }
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
        let launched = self
            .step(|register| {
                xcode::built_app(
                    &self.project,
                    &scheme,
                    &self.configuration(&scheme),
                    &xcode::destination(target.as_deref()),
                    &self.worktree,
                    register,
                )
            })
            .and_then(|app| {
                if let Some(preview) = lock(&self.builds.0.previews).get_mut(&self.worktree) {
                    preview.launch = Some(self.id);
                    preview.ended = None;
                }
                let (builds, worktree, id, device) = (
                    self.builds.clone(),
                    self.worktree.clone(),
                    self.id,
                    udid.clone(),
                );
                self.step(|register| {
                    preview::launch(&self.worktree, &udid, &app, register, move |message| {
                        builds.preview_ended(&worktree, id, &device, message)
                    })
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
}
