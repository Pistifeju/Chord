//! The build slot of each worktree: one build at a time, and a newer build takes over from the one that runs. Pure rules, no processes; build.rs keeps one `Slots` behind a mutex and does the signalling.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) struct Run {
    pub id: u64,
    /// Process group of the xcodebuild that runs now; a stop signals the whole group, compilers included.
    pub pid: Option<u32>,
    pub stopped: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Start {
    pub id: u64,
    /// The process group of the build this one replaced, if it ran xcodebuild.
    pub kill: Option<u32>,
}

#[derive(Default)]
pub(crate) struct Slots {
    runs: HashMap<PathBuf, Run>,
    next_id: u64,
}

impl Slots {
    /// Takes the worktree's slot for a new build.
    pub fn ask(&mut self, worktree: &Path) -> Start {
        let kill = self.runs.get(worktree).and_then(|run| run.pid);
        let id = self.next_id;
        self.next_id += 1;
        self.runs.insert(
            worktree.to_path_buf(),
            Run {
                id,
                pid: None,
                stopped: false,
            },
        );
        Start { id, kill }
    }

    /// Records the process group run `id` started. True when the group must end at once: the run was stopped or replaced meanwhile.
    pub fn register(&mut self, worktree: &Path, id: u64, pid: u32) -> bool {
        match self.runs.get_mut(worktree).filter(|run| run.id == id) {
            Some(run) => {
                run.pid = Some(pid);
                run.stopped
            }
            None => true,
        }
    }

    /// Run `id`'s process group ended.
    pub fn clear_pid(&mut self, worktree: &Path, id: u64) {
        if let Some(run) = self.runs.get_mut(worktree).filter(|run| run.id == id) {
            run.pid = None;
        }
    }

    pub fn is_current(&self, worktree: &Path, id: u64) -> bool {
        self.runs.get(worktree).is_some_and(|run| run.id == id)
    }

    /// Run `id` should give up: it was stopped, or a newer build took over.
    pub fn stopped(&self, worktree: &Path, id: u64) -> bool {
        self.runs
            .get(worktree)
            .is_none_or(|run| run.id != id || run.stopped)
    }

    /// Marks the worktree's run stopped; returns the process group to signal.
    pub fn stop(&mut self, worktree: &Path) -> Option<u32> {
        let run = self.runs.get_mut(worktree)?;
        run.stopped = true;
        run.pid
    }

    pub fn stop_all(&mut self) -> Vec<u32> {
        self.runs
            .values_mut()
            .filter_map(|run| {
                run.stopped = true;
                run.pid
            })
            .collect()
    }

    /// Run `id` ended: frees the slot and returns the run, unless a newer build holds the slot.
    pub fn finished(&mut self, worktree: &Path, id: u64) -> Option<Run> {
        if !self.is_current(worktree, id) {
            return None;
        }
        self.runs.remove(worktree)
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
        let first = slots.ask(w).id;
        assert_eq!(
            slots.ask(w),
            Start {
                id: first + 1,
                kill: None
            }
        );
        let second = first + 1;
        slots.register(w, second, 42);
        assert_eq!(
            slots.ask(w),
            Start {
                id: second + 1,
                kill: Some(42)
            }
        );
        assert!(slots.stopped(w, second));
        assert!(slots.finished(w, second).is_none());
        assert!(!slots.stopped(w, second + 1));
    }

    #[test]
    fn two_worktrees_hold_two_slots() {
        let mut slots = Slots::default();
        let a = slots.ask(Path::new("/a")).id;
        let b = slots.ask(Path::new("/b")).id;
        assert!(!slots.stopped(Path::new("/a"), a));
        assert!(!slots.stopped(Path::new("/b"), b));
        set_pid(&mut slots, Path::new("/a"), a, 1);
        set_pid(&mut slots, Path::new("/b"), b, 2);
        assert_eq!(slots.stop(Path::new("/a")), Some(1));
        assert!(slots.stopped(Path::new("/a"), a));
        assert!(!slots.stopped(Path::new("/b"), b));
    }

    #[test]
    fn stop_marks_the_run_and_the_end_frees_the_slot() {
        let mut slots = Slots::default();
        let w = Path::new("/w");
        let id = slots.ask(w).id;
        assert_eq!(slots.stop(w), None);
        assert!(slots.stopped(w, id));
        set_pid(&mut slots, w, id, 7);
        assert_eq!(slots.stop_all(), vec![7]);
        assert!(slots.finished(w, id).is_some_and(|run| run.stopped));
        assert!(slots.stopped(w, id));
        assert_eq!(slots.stop(Path::new("/other")), None);
    }
}
