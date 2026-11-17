//! The Preview view's session in a worktree: its preview simulator booted on open, the pick it shows, the saves that hot reload or rebuild it, and the preview build that launches the app on it.

use chord_core::lock;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chord_core::{Error, PreviewPick, RunDestination, SchemePick};

use super::{Build, BuildEvent, BuildPhase, Builds, Lane};
use crate::hot_reload;
use crate::preview::{self, PreviewState};
use crate::preview_list;
use crate::simulator::{self, DeviceSet};
use crate::worktree_saves::{self, Saves};
use crate::xcode;

/// What an opened Preview view does next.
#[derive(Debug, PartialEq)]
enum OnOpen {
    ShowPick,
    Live,
    Waiting,
    Building,
    StartBuild,
}

/// `launched`: the preview app runs the current code on this simulator. `picked`: the view opened with another pick than before.
fn on_open(launched: bool, picked: bool, waits: bool, runs: bool) -> OnOpen {
    match (launched, picked) {
        (true, true) => OnOpen::ShowPick,
        (true, false) => OnOpen::Live,
        _ if waits => OnOpen::Waiting,
        _ if runs => OnOpen::Building,
        _ => OnOpen::StartBuild,
    }
}

/// A worktree whose Preview view opened, with its preview simulator.
pub(super) struct Preview {
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

impl Builds {
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
        match on_open(launched, picked, waits, runs) {
            OnOpen::ShowPick => self.show_pick(&worktree),
            OnOpen::Live => on_state(PreviewState::Live),
            OnOpen::Waiting => on_state(PreviewState::Waiting),
            OnOpen::Building => on_state(PreviewState::Building),
            OnOpen::StartBuild => self.start_preview(&worktree),
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
    pub(super) fn tell_preview(&self, worktree: &Path, state: PreviewState) {
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
    pub(super) fn follow_with_preview(&self, worktree: &Path) {
        let waits = lock(&self.0.slots).take_waiting_preview(worktree);
        if waits {
            self.start_preview(worktree);
        }
    }
}

impl Build {
    /// A preview build: builds like ⌘B for a simulator, then launches the app on the preview simulator. One whose preview simulator was shut down meanwhile builds nothing.
    pub(super) fn preview_outcome(&self) -> BuildEvent {
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
