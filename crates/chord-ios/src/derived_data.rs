//! The DerivedData folder Xcode gives a project path, and who holds its build database.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use md5::{Digest, Md5};

use crate::simulator;

/// `lsof` ships with macOS at this path.
pub(crate) const LSOF: &str = "/usr/sbin/lsof";

/// `~/Library/Developer`, where Xcode and CoreSimulator keep their data. `None` without a home folder.
pub(crate) fn developer_folder() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(Path::new(&home).join("Library/Developer"))
}

fn root() -> Option<PathBuf> {
    Some(developer_folder()?.join("Xcode/DerivedData"))
}

/// Xcode's folder for the project at `project`: its name, a dash and a hash of its path. `None` without a home folder.
pub(crate) fn folder_of(project: &Path) -> Option<PathBuf> {
    Some(root()?.join(folder_name(project)?))
}

/// Each project folder in DerivedData with the project path Xcode recorded in its `info.plist`; a folder without one, such as the module cache, is no project's.
pub(crate) fn projects() -> Vec<(PathBuf, PathBuf)> {
    let Some(root) = root() else {
        return Vec::new();
    };
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let folder = entry.path();
            let recorded =
                simulator::plist_value(&folder.join("info.plist"), "WorkspacePath", &|_| {});
            Some((folder, PathBuf::from(recorded.ok()?)))
        })
        .collect()
}

/// Xcode writes a space in the project's name as an underscore.
fn folder_name(project: &Path) -> Option<String> {
    let name = project.file_stem()?.to_string_lossy().replace(' ', "_");
    Some(format!("{name}-{}", path_hash(&project.to_string_lossy())))
}

pub(crate) fn build_db(folder: &Path) -> PathBuf {
    folder.join("Build/Intermediates.noindex/XCBuildData/build.db")
}

/// MD5 of the path; each half is a big-endian 64-bit number written as 14 letters in base 26.
pub(crate) fn path_hash(path: &str) -> String {
    let digest = Md5::digest(path.as_bytes());
    digest
        .chunks(8)
        .map(|half| {
            let mut number = half.iter().fold(0u64, |n, &byte| n << 8 | u64::from(byte));
            let mut letters = [b'a'; 14];
            for letter in letters.iter_mut().rev() {
                *letter = b'a' + (number % 26) as u8;
                number /= 26;
            }
            String::from_utf8_lossy(&letters).into_owned()
        })
        .collect()
}

/// The processes that have the file open. Empty when `lsof` finds none; `require_tools` has checked it exists.
pub(crate) fn holders(file: &Path) -> Vec<u32> {
    let Ok(output) = Command::new(LSOF)
        .arg("-t")
        .arg("--")
        .arg(file)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_a_project_path_like_xcode() {
        assert_eq!(
            path_hash("/tmp/Example/Example.xcodeproj"),
            "bvpgkaocquoraeafghcjoyuszgza"
        );
    }

    // Folder names Xcode 27 gave projects at these paths on the developer's machine.
    #[test]
    fn names_the_folder_like_xcode_for_a_neutral_path_and_one_with_spaces() {
        assert_eq!(
            folder_name(Path::new("/tmp/neutral/Example/Example.xcodeproj")).as_deref(),
            Some("Example-cqbowptkchhfoacftbdmmagekkzu")
        );
        assert_eq!(
            folder_name(Path::new("/tmp/Chord Probe/My App/My App.xcodeproj")).as_deref(),
            Some("My_App-azhlkdaqhdwqkfgplcsnypjggokw")
        );
    }
}
