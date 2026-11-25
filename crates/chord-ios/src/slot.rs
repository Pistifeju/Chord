use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct Run {
    pub id: u64,
    pub pid: Option<u32>,
    pub stopped: bool,
    pub app: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Start {
    pub id: u64,
    pub kill: Vec<u32>,
    pub beside_app: bool,
}

#[derive(Default)]
pub(crate) struct Slots {
    runs: HashMap<PathBuf, Run>,
    beside_app: HashMap<PathBuf, Run>,
    previews: HashMap<PathBuf, Run>,
    waiting_previews: HashSet<PathBuf>,
    stop_requests: HashSet<u64>,
    next_id: u64,
}

impl Slots {
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

    fn building(&self, worktree: &Path) -> bool {
        self.runs.get(worktree).is_some_and(|run| !run.app)
            || self.beside_app.contains_key(worktree)
    }

    pub fn holds_preview(&self, worktree: &Path) -> bool {
        self.building(worktree) || self.preview_runs(worktree) || self.preview_waits(worktree)
    }

    pub fn preview_runs(&self, worktree: &Path) -> bool {
        self.previews.contains_key(worktree)
    }

    pub fn preview_waits(&self, worktree: &Path) -> bool {
        self.waiting_previews.contains(worktree)
    }

    pub fn follow_with_preview(&mut self, worktree: &Path) {
        self.waiting_previews.insert(worktree.to_path_buf());
    }

    pub fn take_waiting_preview(&mut self, worktree: &Path) -> bool {
        !self.building(worktree) && self.waiting_previews.remove(worktree)
    }

    pub fn forget_preview(&mut self, worktree: &Path) -> Option<u32> {
        self.waiting_previews.remove(worktree);
        self.previews.remove(worktree).and_then(|run| run.pid)
    }

    fn run_mut(&mut self, worktree: &Path, id: u64) -> Option<&mut Run> {
        [&mut self.runs, &mut self.beside_app, &mut self.previews]
            .into_iter()
            .find_map(|slots| slots.get_mut(worktree).filter(|run| run.id == id))
    }

    pub fn register(&mut self, worktree: &Path, id: u64, pid: u32) -> bool {
        match self.run_mut(worktree, id) {
            Some(run) => {
                run.pid = Some(pid);
                run.stopped
            }
            None => true,
        }
    }

    pub fn clear_pid(&mut self, worktree: &Path, id: u64) {
        if let Some(run) = self.run_mut(worktree, id) {
            run.pid = None;
        }
    }

    pub fn app_launched(&mut self, worktree: &Path, id: u64) {
        if let Some(run) = self.run_mut(worktree, id) {
            run.app = true;
        }
    }

    pub fn held_worktrees(&self) -> Vec<PathBuf> {
        self.runs.keys().cloned().collect()
    }

    pub fn is_current(&mut self, worktree: &Path, id: u64) -> bool {
        self.run_mut(worktree, id).is_some()
    }

    pub fn stopped(&mut self, worktree: &Path, id: u64) -> bool {
        self.run_mut(worktree, id).is_none_or(|run| run.stopped)
    }

    pub fn take_stop_request(&mut self, id: u64) -> bool {
        self.stop_requests.remove(&id)
    }

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
    fn a_run_holds_its_worktree_before_its_app_launches() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let id = slots.ask(w, true).id;
        assert_eq!(slots.held_worktrees(), vec![PathBuf::from("/w")]);
        slots.finished(w, id);
        assert!(slots.held_worktrees().is_empty());
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
