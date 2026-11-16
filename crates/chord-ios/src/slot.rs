//! The build slots of each worktree: one build at a time, and a newer build takes over from the one that runs. A ⌘B while the app runs builds beside the run that holds the app and leaves it. A preview build waits for the worktree's ⌘R or ⌘B build, then follows it, and either takes over from a preview build. Pure rules, no processes; build.rs keeps one `Slots` behind a mutex and does the signalling.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct Run {
    pub id: u64,
    /// Process group of the tool that runs now; a stop signals the whole group, compilers included. While the app runs it is the app's console, and only a stop may signal it.
    pub pid: Option<u32>,
    pub stopped: bool,
    /// The app this run launched runs.
    pub app: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Start {
    pub id: u64,
    /// The process groups of the builds this one replaced.
    pub kill: Vec<u32>,
    /// A build beside the run that holds the app: it replaced no app run.
    pub beside_app: bool,
}

#[derive(Default)]
pub(crate) struct Slots {
    runs: HashMap<PathBuf, Run>,
    /// The build of a ⌘B that came while the worktree's run held the app.
    beside_app: HashMap<PathBuf, Run>,
    previews: HashMap<PathBuf, Run>,
    /// Worktrees whose preview build follows their ⌘R or ⌘B build.
    waiting_previews: HashSet<PathBuf>,
    /// Runs a stop reached; kept after a takeover so the stopped run still knows it was stopped.
    stop_requests: HashSet<u64>,
    next_id: u64,
}

impl Slots {
    /// Takes a slot of the worktree for a new build. A build without `run` leaves a run that holds the app, and a newer one of its kind replaces it; any other build takes over every slot, so the app's console ends and a launch replaces the app.
    /// A preview build it takes over follows it.
    pub fn ask(&mut self, worktree: &Path, run: bool) -> Start {
        let fresh = self.fresh();
        let id = fresh.id;
        let preview = self.previews.remove(worktree);
        if preview.is_some() {
            self.waiting_previews.insert(worktree.to_path_buf());
        }
        let preview = preview.and_then(|run| run.pid);
        let holds_app = self.runs.get(worktree).is_some_and(|run| run.app);
        if !run && holds_app {
            let kill = self.beside_app.get(worktree).and_then(|run| run.pid);
            self.beside_app.insert(worktree.to_path_buf(), fresh);
            return Start {
                id,
                kill: kill.into_iter().chain(preview).collect(),
                beside_app: true,
            };
        }
        let kill = [self.runs.remove(worktree), self.beside_app.remove(worktree)]
            .into_iter()
            .flatten()
            .filter_map(|run| run.pid)
            .chain(preview)
            .collect();
        self.runs.insert(worktree.to_path_buf(), fresh);
        Start {
            id,
            kill,
            beside_app: false,
        }
    }

    fn fresh(&mut self) -> Run {
        let id = self.next_id;
        self.next_id += 1;
        Run {
            id,
            pid: None,
            stopped: false,
            app: false,
        }
    }

    /// Takes the worktree's preview slot, unless a ⌘R or ⌘B build holds the worktree: then the preview build follows it and this returns None. A newer preview build takes over from the one that runs.
    pub fn ask_preview(&mut self, worktree: &Path) -> Option<Start> {
        if self.building(worktree) {
            self.waiting_previews.insert(worktree.to_path_buf());
            return None;
        }
        let fresh = self.fresh();
        let id = fresh.id;
        let kill = self.previews.insert(worktree.to_path_buf(), fresh);
        Some(Start {
            id,
            kill: kill.and_then(|run| run.pid).into_iter().collect(),
            beside_app: false,
        })
    }

    /// A ⌘R or ⌘B build is on before its app runs.
    fn building(&self, worktree: &Path) -> bool {
        self.runs.get(worktree).is_some_and(|run| !run.app)
            || self.beside_app.contains_key(worktree)
    }

    /// A preview build runs or waits, or a ⌘R or ⌘B build before its app runs: the preview app is about to change.
    pub fn holds_preview(&self, worktree: &Path) -> bool {
        self.building(worktree) || self.preview_runs(worktree) || self.preview_waits(worktree)
    }

    pub fn preview_runs(&self, worktree: &Path) -> bool {
        self.previews.contains_key(worktree)
    }

    pub fn preview_waits(&self, worktree: &Path) -> bool {
        self.waiting_previews.contains(worktree)
    }

    /// A preview build follows the worktree's ⌘R or ⌘B build.
    pub fn follow_with_preview(&mut self, worktree: &Path) {
        self.waiting_previews.insert(worktree.to_path_buf());
    }

    /// True once, when a preview build waits and no ⌘R or ⌘B build holds the worktree any longer.
    pub fn take_waiting_preview(&mut self, worktree: &Path) -> bool {
        !self.building(worktree) && self.waiting_previews.remove(worktree)
    }

    /// For a worktree that was deleted: stops its preview build, drops a waiting one, returns the process group to signal.
    pub fn forget_preview(&mut self, worktree: &Path) -> Option<u32> {
        self.waiting_previews.remove(worktree);
        self.previews.remove(worktree).and_then(|run| run.pid)
    }

    fn run_mut(&mut self, worktree: &Path, id: u64) -> Option<&mut Run> {
        [&mut self.runs, &mut self.beside_app, &mut self.previews]
            .into_iter()
            .find_map(|slots| slots.get_mut(worktree).filter(|run| run.id == id))
    }

    /// Records the process group run `id` started. True when the group must end at once: the run was stopped or replaced meanwhile.
    pub fn register(&mut self, worktree: &Path, id: u64, pid: u32) -> bool {
        match self.run_mut(worktree, id) {
            Some(run) => {
                run.pid = Some(pid);
                run.stopped
            }
            None => true,
        }
    }

    /// Run `id`'s process group ended.
    pub fn clear_pid(&mut self, worktree: &Path, id: u64) {
        if let Some(run) = self.run_mut(worktree, id) {
            run.pid = None;
        }
    }

    /// The app of run `id` runs.
    pub fn app_launched(&mut self, worktree: &Path, id: u64) {
        if let Some(run) = self.run_mut(worktree, id) {
            run.app = true;
        }
    }

    /// The worktrees whose run holds a running app; a build beside it does not count.
    pub fn running_worktrees(&self) -> Vec<PathBuf> {
        self.runs
            .iter()
            .filter(|(_, run)| run.app)
            .map(|(worktree, _)| worktree.clone())
            .collect()
    }

    pub fn is_current(&mut self, worktree: &Path, id: u64) -> bool {
        self.run_mut(worktree, id).is_some()
    }

    /// Run `id` should give up: it was stopped, or a newer build took over.
    pub fn stopped(&mut self, worktree: &Path, id: u64) -> bool {
        self.run_mut(worktree, id).is_none_or(|run| run.stopped)
    }

    /// True once, when a stop reached run `id`, even if a newer build took the slot since: only then does the run end its app. A takeover alone leaves the app.
    pub fn take_stop_request(&mut self, id: u64) -> bool {
        self.stop_requests.remove(&id)
    }

    /// Marks the worktree's runs stopped; returns the process groups to signal.
    pub fn stop(&mut self, worktree: &Path) -> Vec<u32> {
        let Self {
            runs,
            beside_app,
            stop_requests,
            ..
        } = self;
        [runs.get_mut(worktree), beside_app.get_mut(worktree)]
            .into_iter()
            .flatten()
            .filter_map(|run| {
                run.stopped = true;
                stop_requests.insert(run.id);
                run.pid
            })
            .collect()
    }

    pub fn stop_all(&mut self) -> Vec<u32> {
        let Self {
            runs,
            beside_app,
            previews,
            stop_requests,
            ..
        } = self;
        runs.values_mut()
            .chain(beside_app.values_mut())
            .chain(previews.values_mut())
            .filter_map(|run| {
                run.stopped = true;
                stop_requests.insert(run.id);
                run.pid
            })
            .collect()
    }

    /// Run `id` ended: frees its slot and returns the run, unless a newer build holds the slot.
    pub fn finished(&mut self, worktree: &Path, id: u64) -> Option<Run> {
        self.stop_requests.remove(&id);
        [&mut self.runs, &mut self.beside_app, &mut self.previews]
            .into_iter()
            .find(|slots| slots.get(worktree).is_some_and(|run| run.id == id))?
            .remove(worktree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_pid(slots: &mut Slots, worktree: &Path, id: u64, pid: u32) {
        slots.register(worktree, id, pid);
    }

    #[test]
    fn a_new_build_takes_over_and_kills_the_group_of_the_old_one() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let first = slots.ask(w, true).id;
        assert_eq!(
            slots.ask(w, true),
            Start {
                id: first + 1,
                kill: vec![],
                beside_app: false
            }
        );
        let second = first + 1;
        slots.register(w, second, 42);
        assert_eq!(
            slots.ask(w, false),
            Start {
                id: second + 1,
                kill: vec![42],
                beside_app: false
            }
        );
        assert!(slots.stopped(w, second));
        assert!(slots.finished(w, second).is_none());
        assert!(!slots.stopped(w, second + 1));
    }

    #[test]
    fn a_build_beside_a_running_app_signals_nothing_of_the_app_run() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let run = slots.ask(w, true).id;
        set_pid(&mut slots, w, run, 42);
        slots.app_launched(w, run);
        let build = slots.ask(w, false);
        assert_eq!(build.kill, Vec::<u32>::new());
        assert!(build.beside_app);
        assert!(!slots.stopped(w, run));
        set_pid(&mut slots, w, build.id, 43);
        let newer = slots.ask(w, false);
        assert_eq!(newer.kill, vec![43]);
        assert!(!slots.stopped(w, run));
        assert!(slots.stopped(w, build.id));
    }

    #[test]
    fn a_run_takes_over_the_app_run_and_the_build_beside_it() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let run = slots.ask(w, true).id;
        set_pid(&mut slots, w, run, 42);
        slots.app_launched(w, run);
        let build = slots.ask(w, false).id;
        set_pid(&mut slots, w, build, 43);
        let mut kill = slots.ask(w, true).kill;
        kill.sort();
        assert_eq!(kill, vec![42, 43]);
        assert!(slots.stopped(w, run));
        assert!(slots.stopped(w, build));
    }

    #[test]
    fn a_stop_reaches_the_app_run_and_the_build_beside_it() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let run = slots.ask(w, true).id;
        set_pid(&mut slots, w, run, 42);
        slots.app_launched(w, run);
        let build = slots.ask(w, false).id;
        set_pid(&mut slots, w, build, 43);
        let mut stopped = slots.stop(w);
        stopped.sort();
        assert_eq!(stopped, vec![42, 43]);
        assert!(slots.take_stop_request(run));
    }

    #[test]
    fn only_a_stop_ends_the_app_never_a_takeover() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let first = slots.ask(w, true).id;
        let second = slots.ask(w, true).id;
        assert!(!slots.take_stop_request(first));
        assert!(!slots.take_stop_request(second));
        slots.stop(w);
        let third = slots.ask(w, true).id;
        assert!(slots.take_stop_request(second));
        assert!(!slots.take_stop_request(second));
        assert!(!slots.take_stop_request(third));
    }

    #[test]
    fn a_preview_build_waits_for_a_run_and_follows_once_its_app_runs() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let run = slots.ask(w, true).id;
        assert_eq!(slots.ask_preview(w), None);
        assert!(!slots.take_waiting_preview(w));
        slots.app_launched(w, run);
        assert!(slots.take_waiting_preview(w));
        assert!(!slots.take_waiting_preview(w));
        assert!(slots.ask_preview(w).is_some());
        assert!(!slots.stopped(w, run));
    }

    #[test]
    fn a_build_takes_over_a_preview_build_which_then_follows_it() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let preview = slots.ask_preview(w).unwrap().id;
        slots.register(w, preview, 42);
        let build = slots.ask(w, false);
        assert_eq!(build.kill, vec![42]);
        assert!(slots.stopped(w, preview));
        assert!(!slots.take_waiting_preview(w));
        assert!(slots.finished(w, build.id).is_some());
        assert!(slots.take_waiting_preview(w));
    }

    #[test]
    fn a_newer_preview_build_takes_over_and_a_stop_leaves_it() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let first = slots.ask_preview(w).unwrap().id;
        slots.register(w, first, 42);
        let second = slots.ask_preview(w).unwrap();
        assert_eq!(second.kill, vec![42]);
        assert!(slots.stopped(w, first));
        assert_eq!(slots.stop(w), Vec::<u32>::new());
        assert!(!slots.stopped(w, second.id));
    }

    #[test]
    fn two_worktrees_hold_two_slots() {
        let mut slots = Slots::default();
        let a = slots.ask(Path::new("/a"), true).id;
        let b = slots.ask(Path::new("/b"), true).id;
        assert!(!slots.stopped(Path::new("/a"), a));
        assert!(!slots.stopped(Path::new("/b"), b));
        set_pid(&mut slots, Path::new("/a"), a, 1);
        set_pid(&mut slots, Path::new("/b"), b, 2);
        assert_eq!(slots.stop(Path::new("/a")), vec![1]);
        assert!(slots.stopped(Path::new("/a"), a));
        assert!(!slots.stopped(Path::new("/b"), b));
    }

    #[test]
    fn stop_marks_the_run_and_the_end_frees_the_slot() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let id = slots.ask(w, true).id;
        assert_eq!(slots.stop(w), Vec::<u32>::new());
        assert!(slots.stopped(w, id));
        set_pid(&mut slots, w, id, 7);
        assert_eq!(slots.stop_all(), vec![7]);
        assert!(slots.finished(w, id).is_some_and(|run| run.stopped));
        assert!(slots.stopped(w, id));
        assert_eq!(slots.stop(Path::new("/other")), Vec::<u32>::new());
    }
}
