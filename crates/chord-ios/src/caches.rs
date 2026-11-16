//! The caches a worktree leaves outside its folder: its DerivedData folder, its run simulator and its preview simulator.

use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use chord_core::{Error, Project, worktrees};
use specta::Type;

use crate::derived_data;
use crate::preview;
use crate::simulator::{self, DeviceSet};
use crate::simulator_kit;

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct RunningSimulator {
    /// The worktree's folder name.
    pub worktree: String,
    /// "iPhone 17 Pro".
    pub model: String,
    /// It is the open worktree's.
    pub open: bool,
    /// It shows the worktree's preview.
    pub preview: bool,
}

/// The run and preview simulators Chord created in the set; a set that cannot be listed has none.
fn owned_in(set: DeviceSet) -> Vec<simulator::Simulator> {
    simulator::owned(set).unwrap_or_default()
}

/// The booted simulators Chord created, in any worktree, its preview simulators after its run simulators; `open` marks those of the open worktree.
pub fn running_simulators(open: Option<&Path>) -> Result<Vec<RunningSimulator>, Error> {
    let open = open.map(simulator::name_of);
    let previews = owned_in(DeviceSet::Preview);
    Ok(simulator::owned(DeviceSet::Run)?
        .into_iter()
        .map(|device| (device, false))
        .chain(previews.into_iter().map(|device| (device, true)))
        .filter(|(device, _)| device.booted)
        .filter_map(|(device, preview)| {
            Some(RunningSimulator {
                worktree: simulator::worktree_folder(&device.name)?.into(),
                model: model_name(&device.model),
                open: open.as_deref() == Some(device.name.as_str()),
                preview,
            })
        })
        .collect())
}

thread_local! {
    static WATCHING_SIMULATORS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Main thread, after `prepare_screen` loaded the frameworks. Calls `on_change` whenever a simulator is added, removed, boots or shuts down. Registers once per app run, since CoreSimulator offers no unregister: a later call keeps the first `on_change`. False when the frameworks did not load.
pub fn watch_simulators(on_change: impl Fn() + 'static) -> bool {
    let Ok(kit) = simulator_kit::load() else {
        return false;
    };
    if !WATCHING_SIMULATORS.get() {
        // The handler fires for the app's life, so its block is never released.
        let watched = simulator_kit::guarded(|| std::mem::forget(kit.watch(on_change)));
        WATCHING_SIMULATORS.set(watched.is_some());
    }
    WATCHING_SIMULATORS.get()
}

/// "com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro" reads "iPhone 17 Pro".
fn model_name(identifier: &str) -> String {
    let last = identifier.rsplit('.').next().unwrap_or(identifier);
    last.split('-')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Shuts down every booted simulator Chord created except the worktree's own run and preview simulators, all at once.
pub fn shut_down_others(keep: &Path) -> Result<(), Error> {
    let keep = simulator::name_of(keep);
    let others = |devices: Vec<simulator::Simulator>| {
        devices
            .into_iter()
            .filter(|device| device.booted && device.name != keep)
            .map(|device| device.udid)
            .collect()
    };
    let runs = others(simulator::owned(DeviceSet::Run)?);
    let previews = others(owned_in(DeviceSet::Preview));
    let ending = std::thread::spawn(move || simulator::shut_down_all(DeviceSet::Preview, previews));
    simulator::shut_down_all(DeviceSet::Run, runs);
    let _ = ending.join();
    Ok(())
}

/// The worktree is already gone from disk, so the DerivedData folder is found by its recorded path, not hashed. Best effort: what stays goes at the next prune.
pub fn forget(worktree: &Path) {
    let real = resolved(worktree);
    remove_derived_data(|project| resolved(project).starts_with(&real));
    let name = simulator::name_of(worktree);
    for set in [DeviceSet::Run, DeviceSet::Preview] {
        for device in owned_in(set).iter().filter(|device| device.name == name) {
            let _ = simulator::delete(set, &device.udid);
        }
    }
}

/// Removes the build data of worktrees that are gone; never a project outside the worktrees folders or a simulator Chord did not create. It does nothing while any project's worktrees cannot be listed: a project that is not mounted must not look empty. Best effort past that check.
pub fn prune(projects: &[Project]) -> Result<(), Error> {
    let live = live_worktrees(projects, true)?;
    let folders: Vec<PathBuf> = projects
        .iter()
        .map(|project| resolved(&worktrees::worktrees_folder(project)))
        .collect();
    remove_derived_data(|project| {
        !project.exists() && {
            let project = resolved(project);
            folders.iter().any(|folder| project.starts_with(folder))
        }
    });
    let live = simulator_names(&live);
    for set in [DeviceSet::Run, DeviceSet::Preview] {
        for device in owned_in(set)
            .iter()
            .filter(|device| !live.contains(&device.name))
        {
            let _ = simulator::delete(set, &device.udid);
        }
    }
    Ok(())
}

fn live_worktrees(projects: &[Project], with_main: bool) -> Result<Vec<PathBuf>, Error> {
    let mut live = Vec::new();
    for project in projects {
        live.extend(
            worktrees::list(&project.path)?
                .into_iter()
                .map(|worktree| worktree.path)
                .filter(|path| with_main || *path != project.path),
        );
    }
    Ok(live)
}

fn remove_derived_data(gone: impl Fn(&Path) -> bool) {
    for (folder, project) in derived_data::projects() {
        if gone(&project) {
            let _ = fs::remove_dir_all(folder);
        }
    }
}

/// The disk the DerivedData folders and simulators of the projects' created worktrees take, in bytes, without the main checkouts'.
pub fn build_data_size(projects: &[Project]) -> Result<u64, Error> {
    let live = live_worktrees(projects, false)?;
    let resolved_live: Vec<PathBuf> = live.iter().map(|path| resolved(path)).collect();
    let derived: u64 = derived_data::projects()
        .iter()
        .filter(|(_, project)| {
            let project = resolved(project);
            resolved_live
                .iter()
                .any(|worktree| project.starts_with(worktree))
        })
        .map(|(folder, _)| disk_size(folder))
        .sum();
    let names = simulator_names(&live);
    let folders = [
        (
            DeviceSet::Run,
            derived_data::developer_folder()
                .map(|developer| developer.join("CoreSimulator/Devices")),
        ),
        (DeviceSet::Preview, preview::devices_folder()),
    ];
    let simulators: u64 = folders
        .into_iter()
        .filter_map(|(set, folder)| Some((set, folder?)))
        .flat_map(|(set, folder)| {
            owned_in(set)
                .into_iter()
                .filter(|device| names.contains(&device.name))
                .map(move |device| disk_size(&folder.join(&device.udid)))
        })
        .sum();
    Ok(derived + simulators)
}

fn simulator_names(worktrees: &[PathBuf]) -> HashSet<String> {
    worktrees
        .iter()
        .map(|path| simulator::name_of(path))
        .collect()
}

/// The path with symbolic links followed, also when its last folders are gone.
fn resolved(path: &Path) -> PathBuf {
    if let Ok(real) = fs::canonicalize(path) {
        return real;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => resolved(parent).join(name),
        _ => path.to_path_buf(),
    }
}

/// The blocks a folder's files take; symbolic links count themselves, not what they point at.
fn disk_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    let own = meta.blocks() * 512;
    if !meta.is_dir() {
        return own;
    }
    own + fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| disk_size(&entry.path()))
        .sum::<u64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_model_from_its_device_type_identifier() {
        assert_eq!(
            model_name("com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro"),
            "iPhone 17 Pro"
        );
    }

    #[test]
    fn prune_stops_when_a_project_cannot_be_listed() {
        let unmounted = Project {
            path: PathBuf::from("/Volumes/chord-not-mounted/app"),
            name: "app".into(),
        };
        assert!(prune(&[unmounted]).is_err());
    }
}
