use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use chord_core::Error;
use serde::Serialize;
use specta::Type;

use crate::simulator;

const SIPS: &str = "/usr/bin/sips";
const PREVIEW_SIDE: &str = "160";

#[derive(Serialize, Type, Clone, Debug)]
pub struct Screenshot {
    pub file: PathBuf,
    pub preview: PathBuf,
}

fn local_stamp() -> String {
    let mut text = [0u8; 32];
    let written = unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut time: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut time);
        libc::strftime(
            text.as_mut_ptr().cast(),
            text.len(),
            c"%Y-%m-%d at %H.%M.%S".as_ptr(),
            &time,
        )
    };
    String::from_utf8_lossy(&text[..written]).into_owned()
}

pub(crate) fn free_file(folder: &Path, kind: &str, extension: &str) -> PathBuf {
    let stamp = local_stamp();
    let mut file = folder.join(format!("Simulator {kind} {stamp}.{extension}"));
    let mut count = 1;
    while file.exists() {
        count += 1;
        file = folder.join(format!("Simulator {kind} {stamp} {count}.{extension}"));
    }
    file
}

pub(crate) fn save_folder(folder: Option<&Path>) -> Result<PathBuf, Error> {
    let folder = match folder {
        Some(folder) => folder.to_path_buf(),
        None => std::env::var_os("HOME")
            .map(|home| Path::new(&home).join("Desktop"))
            .ok_or_else(|| {
                Error::Message("HOME is not set, so Chord cannot find the Desktop.".into())
            })?,
    };
    if !folder.is_dir() {
        return Err(Error::Message(format!(
            "The folder {} is gone. Choose another one in Settings.",
            folder.display()
        )));
    }
    Ok(folder)
}

pub(crate) fn previews_folder() -> Result<PathBuf, Error> {
    let temporary = std::env::temp_dir().join("chord-screenshots");
    std::fs::create_dir_all(&temporary)?;
    Ok(temporary)
}

pub(crate) fn preview_of(file: PathBuf, image: &Path) -> Result<Screenshot, Error> {
    let temporary = previews_folder()?;
    let preview = temporary.join(format!(
        "{}.preview.png",
        file.file_stem().unwrap_or_default().to_string_lossy()
    ));
    let resized = Command::new(SIPS)
        .args(["-Z", PREVIEW_SIDE])
        .arg(image)
        .arg("--out")
        .arg(&preview)
        .output()?;
    if !resized.status.success() {
        return Err(Error::Message("The preview could not be made.".into()));
    }
    delete_old_shots(&temporary);
    Ok(Screenshot { file, preview })
}

pub fn take_screenshot(
    worktree: &Path,
    preview: bool,
    folder: Option<&Path>,
) -> Result<Screenshot, Error> {
    let (set, udid) = simulator::booted(worktree, preview)?;
    let file = free_file(&save_folder(folder)?, "Screenshot", "png");
    simulator::simctl(
        set,
        &[
            "io",
            &udid,
            "screenshot",
            "--type=png",
            &file.to_string_lossy(),
        ],
        &|_| {},
    )?;
    preview_of(file.clone(), &file)
}

const KEPT_SHOTS: usize = 10;

fn files_of_old_shots(files: &[(String, SystemTime)]) -> Vec<&str> {
    let stem = |name: &str| {
        name.strip_suffix(".preview.png")
            .or_else(|| name.strip_suffix(".png"))
            .unwrap_or(name)
            .to_string()
    };
    let mut newest: HashMap<String, SystemTime> = HashMap::new();
    for (name, time) in files {
        let entry = newest.entry(stem(name)).or_insert(*time);
        *entry = (*entry).max(*time);
    }
    let mut shots: Vec<(&String, &SystemTime)> = newest.iter().collect();
    shots.sort_by(|a, b| b.1.cmp(a.1).then(b.0.cmp(a.0)));
    let old: HashSet<&String> = shots
        .into_iter()
        .skip(KEPT_SHOTS)
        .map(|shot| shot.0)
        .collect();
    files
        .iter()
        .filter(|(name, _)| old.contains(&stem(name)))
        .map(|(name, _)| name.as_str())
        .collect()
}

fn delete_old_shots(temporary: &Path) {
    let Ok(entries) = std::fs::read_dir(temporary) else {
        return;
    };
    let files: Vec<(String, SystemTime)> = entries
        .flatten()
        .filter_map(|entry| {
            let time = entry.metadata().ok()?.modified().ok()?;
            Some((entry.file_name().into_string().ok()?, time))
        })
        .filter(|(name, _)| name.ends_with(".png"))
        .collect();
    for name in files_of_old_shots(&files) {
        let _ = std::fs::remove_file(temporary.join(name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn old_shots_lose_both_files_and_the_ten_newest_stay() {
        let at = |seconds| SystemTime::UNIX_EPOCH + Duration::from_secs(seconds);
        let mut files = Vec::new();
        for n in 1..=12u64 {
            files.push((format!("s{n:02}.png"), at(n)));
            files.push((format!("s{n:02}.preview.png"), at(n)));
        }
        let mut old = files_of_old_shots(&files);
        old.sort();
        assert_eq!(
            old,
            ["s01.png", "s01.preview.png", "s02.png", "s02.preview.png"]
        );
    }
}
