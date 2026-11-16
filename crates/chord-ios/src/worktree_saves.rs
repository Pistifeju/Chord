//! The saves in a worktree whose Preview view opened, from FSEvents: bursts 75 ms apart, each split into its `.swift` files and whether another file the app is built from changed. A file written with the contents it had counts once, so a build that rewrites a file starts no second build.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use chord_core::Error;

/// An editor's save, its temporary file and rename, lands within a few milliseconds.
const BURST: Duration = Duration::from_millis(75);

#[derive(Default, Debug)]
pub(crate) struct Saves {
    /// Real paths.
    pub swift: BTreeSet<String>,
    /// Any other file the app is built from: only a build picks it up.
    pub other: bool,
}

#[derive(Debug, PartialEq)]
enum Change {
    Swift,
    Other,
}

/// What a change means for the worktree's preview; None outside the worktree, in build and version control folders, and for files the app is not built from.
fn change(worktree: &Path, path: &Path) -> Option<Change> {
    let relative = path.strip_prefix(worktree).ok()?;
    let names: Vec<&str> = relative
        .components()
        .filter_map(|part| part.as_os_str().to_str())
        .collect();
    if names.iter().any(|name| {
        matches!(
            *name,
            ".git" | "DerivedData" | ".build" | "build" | ".swiftpm"
        )
    }) {
        return None;
    }
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("swift") => Some(Change::Swift),
        Some("xcstrings" | "m" | "mm" | "c" | "h" | "pbxproj" | "resolved") => Some(Change::Other),
        _ if names.last() == Some(&"Info.plist")
            || names.iter().any(|name| name.ends_with(".xcassets")) =>
        {
            Some(Change::Other)
        }
        _ => None,
    }
}

fn contents_hash(path: &Path) -> Option<u64> {
    let mut hasher = DefaultHasher::new();
    fs::read(path).ok()?.hash(&mut hasher);
    Some(hasher.finish())
}

/// Watches the worktree until the watcher drops. `on_saves` hears each burst on a thread of its own, one at a time.
pub(crate) fn watch(
    worktree: &Path,
    on_saves: impl Fn(Saves) + Send + 'static,
) -> Result<RecommendedWatcher, Error> {
    let (sender, received) = mpsc::channel::<(PathBuf, Change)>();
    // FSEvents names real paths.
    let root = fs::canonicalize(worktree)?;
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && !event.kind.is_access()
        {
            for path in event.paths {
                if let Some(change) = change(&root, &path) {
                    let _ = sender.send((path, change));
                }
            }
        }
    })
    .map_err(|error| Error::Message(format!("The worktree's saves cannot be watched: {error}")))?;
    watcher
        .watch(worktree, RecursiveMode::Recursive)
        .map_err(|error| {
            Error::Message(format!("The worktree's saves cannot be watched: {error}"))
        })?;
    thread::spawn(move || {
        let mut contents: HashMap<PathBuf, Option<u64>> = HashMap::new();
        while let Ok(first) = received.recv() {
            let mut saves = Saves::default();
            let mut next = Some(first);
            while let Some((path, change)) = next {
                let hash = contents_hash(&path);
                if contents.insert(path.clone(), hash) != Some(hash) {
                    match change {
                        Change::Swift => {
                            saves.swift.insert(path.to_string_lossy().into_owned());
                        }
                        Change::Other => saves.other = true,
                    }
                }
                next = received.recv_timeout(BURST).ok();
            }
            if saves.other || !saves.swift.is_empty() {
                on_saves(saves);
            }
        }
    });
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_changes_into_swift_and_other_app_files() {
        let change = |path: &str| change(Path::new("/w/app"), Path::new(path));
        assert_eq!(change("/w/app/Sources/View.swift"), Some(Change::Swift));
        for other in [
            "Sources/View.m",
            "App.xcodeproj/project.pbxproj",
            "App/Info.plist",
            "App/Assets.xcassets/Red.colorset/Contents.json",
            "Localizable.xcstrings",
            "App.xcworkspace/xcshareddata/swiftpm/Package.resolved",
        ] {
            assert_eq!(
                change(&format!("/w/app/{other}")),
                Some(Change::Other),
                "{other}"
            );
        }
        for ignored in [
            "/w/app/README.md",
            "/w/app/Other.plist",
            "/w/app/.build/checkouts/x/A.swift",
            "/w/app/.git/A.swift",
            "/w/app/.swiftpm/xcode/Package.resolved",
            "/elsewhere/A.swift",
        ] {
            assert_eq!(change(ignored), None, "{ignored}");
        }
    }
}
