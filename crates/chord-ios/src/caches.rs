//! The caches a worktree leaves outside its folder: its DerivedData folder and its simulator.

use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::thread;

use serde::Serialize;
use chord_core::Error;
use specta::Type;

use crate::derived_data;
use crate::simulator;

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct RunningSimulator {
    /// The worktree's folder name.
    pub worktree: String,
    /// "iPhone 17 Pro".
    pub model: String,
    /// It is the open worktree's.
    pub open: bool,
}

/// The booted simulators Chord created, in any worktree; `open` marks the one of the open worktree.
pub fn running_simulators(open: Option<&Path>) -> Result<Vec<RunningSimulator>, Error> {
    let open = open.map(simulator::name_of);
    Ok(simulator::owned()?
        .into_iter()
        .filter(|device| device.booted)
        .filter_map(|device| {
            Some(RunningSimulator {
                worktree: simulator::worktree_folder(&device.name)?.into(),
                model: model_name(&device.model),
                open: open.as_deref() == Some(device.name.as_str()),
            })
        })
        .collect())
}

/// "com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro" reads "iPhone 17 Pro".
fn model_name(identifier: &str) -> String {
    let last = identifier.rsplit('.').next().unwrap_or(identifier);
    last.split('-')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Shuts down every booted simulator Chord created except the worktree's own, all at once.
pub fn shut_down_others(keep: &Path) -> Result<(), Error> {
    let keep = simulator::name_of(keep);
    let ending: Vec<_> = simulator::owned()?
        .into_iter()
        .filter(|device| device.booted && device.name != keep)
        .map(|device| thread::spawn(move || simulator::shut_down(&device.udid)))
        .collect();
    for handle in ending {
        let _ = handle.join();
    }
    Ok(())
}

/// The worktree is already gone from disk, so the DerivedData folder is found by its recorded path, not hashed. Best effort: what stays goes at the next prune.
pub fn forget(worktree: &Path) {
    let real = resolved(worktree);
    remove_derived_data(|project| resolved(project).starts_with(&real));
    if let Ok(owned) = simulator::owned() {
        let name = simulator::name_of(worktree);
        for device in owned.iter().filter(|device| device.name == name) {
            let _ = simulator::delete(&device.udid);
        }
    }
}

/// Removes the build data of worktrees that are gone; never a project outside `worktree_folders` or a simulator Chord did not create. Best effort.
pub fn prune(worktree_folders: &[PathBuf], live: &[PathBuf]) {
    let folders: Vec<PathBuf> = worktree_folders.iter().map(|path| resolved(path)).collect();
    remove_derived_data(|project| {
        !project.exists() && {
            let project = resolved(project);
            folders.iter().any(|folder| project.starts_with(folder))
        }
    });
    let live = simulator_names(live);
    if let Ok(owned) = simulator::owned() {
        for device in owned.iter().filter(|device| !live.contains(&device.name)) {
            let _ = simulator::delete(&device.udid);
        }
    }
}

fn remove_derived_data(gone: impl Fn(&Path) -> bool) {
    for (folder, project) in derived_data::projects() {
        if gone(&project) {
            let _ = fs::remove_dir_all(folder);
        }
    }
}

/// The disk the DerivedData folders and simulators of the `live` worktrees take, in bytes.
pub fn build_data_size(live: &[PathBuf]) -> u64 {
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
    let Some(developer) = derived_data::developer_folder() else {
        return derived;
    };
    let devices = developer.join("CoreSimulator/Devices");
    let names = simulator_names(live);
    let simulators: u64 = simulator::owned()
        .unwrap_or_default()
        .iter()
        .filter(|device| names.contains(&device.name))
        .map(|device| disk_size(&devices.join(&device.udid)))
        .sum();
    derived + simulators
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
}
