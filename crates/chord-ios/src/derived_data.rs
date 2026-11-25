use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use md5::{Digest, Md5};

use crate::plist;

pub(crate) const LSOF: &str = "/usr/sbin/lsof";

pub(crate) fn developer_folder() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(Path::new(&home).join("Library/Developer"))
}

fn root() -> Option<PathBuf> {
    Some(developer_folder()?.join("Xcode/DerivedData"))
}

pub(crate) fn folder_of(project: &Path) -> Option<PathBuf> {
    Some(root()?.join(folder_name(project)?))
}

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
            let recorded = plist::value(&folder.join("info.plist"), "WorkspacePath", &|_| {});
            Some((folder, PathBuf::from(recorded.ok()?)))
        })
        .collect()
}

fn folder_name(project: &Path) -> Option<String> {
    let name = project.file_stem()?.to_string_lossy().replace(' ', "_");
    Some(format!("{name}-{}", path_hash(&project.to_string_lossy())))
}

pub(crate) fn build_db(folder: &Path) -> PathBuf {
    folder.join("Build/Intermediates.noindex/XCBuildData/build.db")
}

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

    #[test]
    fn names_the_folder_like_xcode() {
        assert_eq!(
            folder_name(Path::new("/tmp/neutral/Example/Example.xcodeproj")).as_deref(),
            Some("Example-cqbowptkchhfoacftbdmmagekkzu")
        );
    }
}
