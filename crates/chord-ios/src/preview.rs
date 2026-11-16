//! The worktree's preview simulator in Chord's own device set, at most two booted at once, and the app launched on it with the preview helper (ChordPreview.swift), which shows the picked `#Preview` full screen and answers on a socket for a new pick and the hot reload.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use chord_core::{Error, PreviewPick, login_shell};
use specta::Type;

use crate::derived_data::path_hash;
use crate::hot_reload;
use crate::process;
use crate::simulator::{self, DeviceSet};

const SOURCE: &str = include_str!("ChordPreview.swift");
const MAX_BOOTED: usize = 2;

static CACHE_FOLDER: OnceLock<PathBuf> = OnceLock::new();

/// Where the preview device set and the built helper go: the app's cache folder. Call once at launch, before the Simulator view loads the frameworks.
pub fn set_cache_folder(folder: &Path) {
    let _ = std::fs::create_dir_all(folder.join("PreviewDevices"));
    let _ = CACHE_FOLDER.set(folder.to_path_buf());
}

/// The folder of Chord's preview device set.
pub(crate) fn devices_folder() -> Option<PathBuf> {
    CACHE_FOLDER
        .get()
        .map(|folder| folder.join("PreviewDevices"))
}

/// The folder of the hot reload folders, one per worktree.
pub(crate) fn hot_reload_folders() -> Option<PathBuf> {
    CACHE_FOLDER.get().map(|folder| folder.join("HotReload"))
}

/// The worktree's compile commands and reload files, named like its simulators.
pub(crate) fn hot_reload_folder(worktree: &Path) -> Option<PathBuf> {
    hot_reload_folders().map(|folder| folder.join(simulator::name_of(worktree)))
}

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PreviewState {
    /// The preview simulator boots.
    Starting,
    /// A ⌘R or ⌘B build of the worktree holds its build; the preview build follows it.
    Waiting,
    Building,
    /// The preview app runs on the preview simulator.
    Live,
    /// Two other worktrees' preview simulators run; their folder names.
    Full {
        worktrees: Vec<String>,
    },
    /// No pick, or the picked `#Preview` is gone.
    Unpicked,
    Failed {
        message: String,
    },
}

pub(crate) enum Booted {
    /// `fresh`: this call booted it, so no app runs on it yet.
    Ready {
        udid: String,
        fresh: bool,
    },
    Full(Vec<String>),
}

/// Held from the count of the booted preview simulators to the end of a boot, so two worktrees never boot a third one together.
// shortcut: a second worktree's Preview view waits for the first one's boot, up to 30 s; count booting devices instead if that shows.
static BOOTING: Mutex<()> = Mutex::new(());

/// The worktree's preview simulator of the `wanted` model, booted unless two other preview simulators run. `starting` hears that a boot begins.
pub(crate) fn boot(
    worktree: &Path,
    wanted: Option<(&str, &str)>,
    starting: impl FnOnce(),
) -> Result<Booted, Error> {
    let _booting = BOOTING.lock().unwrap_or_else(PoisonError::into_inner);
    let device = simulator::find_or_create(DeviceSet::Preview, worktree, wanted, &|_| {})?;
    if device.booted {
        return Ok(Booted::Ready {
            udid: device.udid,
            fresh: false,
        });
    }
    let mut others: Vec<String> = simulator::owned(DeviceSet::Preview)?
        .into_iter()
        .filter(|other| other.booted && other.name != device.name)
        .filter_map(|other| simulator::worktree_folder(&other.name).map(String::from))
        .collect();
    if others.len() >= MAX_BOOTED {
        others.sort();
        return Ok(Booted::Full(others));
    }
    starting();
    simulator::boot(DeviceSet::Preview, &device.udid)?;
    Ok(Booted::Ready {
        udid: device.udid,
        fresh: true,
    })
}

/// Installs the app on the worktree's preview simulator and launches it with the helper in place of its entry point.
pub(crate) fn launch(
    worktree: &Path,
    udid: &str,
    app: &Path,
    register: &dyn Fn(u32),
) -> Result<(), Error> {
    let executable = simulator::plist_text(app, "CFBundleExecutable", register)?;
    if !app.join(format!("{executable}.debug.dylib")).exists() {
        return Err(Error::Message(
            "Previews need the debug dylib: set Enable Debug Dylib Support to Yes in the app target's Debug build settings.".into(),
        ));
    }
    let helper = helper()?;
    let bundle = simulator::plist_text(app, "CFBundleIdentifier", register)?;
    simulator::install(DeviceSet::Preview, udid, app, register)?;
    let folder = devices_folder()
        .ok_or_else(|| Error::Message("Chord has no preview device folder.".into()))?;
    let folder = folder.to_string_lossy();
    let mut command = login_shell(
        "xcrun",
        &[
            "simctl",
            "--set",
            &folder,
            "launch",
            "--terminate-running-process",
            udid,
            &bundle,
        ],
    );
    let socket = socket(worktree);
    let _ = std::fs::remove_file(&socket);
    hot_reload::launched(worktree);
    command
        .env("SIMCTL_CHILD_DYLD_INSERT_LIBRARIES", &helper)
        .env("SIMCTL_CHILD_CHORD_PREVIEW_SOCKET", &socket);
    let output = process::captured(command, register)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Error::Message(format!(
            "The preview did not launch: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

/// In the temp folder: a socket path holds 104 bytes at most.
fn socket(worktree: &Path) -> PathBuf {
    let hash = path_hash(&worktree.to_string_lossy());
    std::env::temp_dir().join(format!("chord-preview-{}.sock", &hash[..12]))
}

/// One request to the helper in the worktree's preview app, which may still be starting.
fn request(worktree: &Path, request: &Value) -> Result<Value, String> {
    let path = socket(worktree);
    let start = Instant::now();
    let mut stream = loop {
        match UnixStream::connect(&path) {
            Ok(stream) => break stream,
            Err(_) if start.elapsed() < Duration::from_secs(2) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(format!("the preview app did not answer: {error}")),
        }
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| error.to_string())?;
    writeln!(stream, "{request}").map_err(|error| error.to_string())?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("the preview app did not answer: {error}"))?;
    let reply: Value = serde_json::from_str(&line).map_err(|error| error.to_string())?;
    match reply["error"].as_str() {
        Some(error) => Err(error.into()),
        None => Ok(reply),
    }
}

/// The `fileID` (`Module/File.swift`) of the preview the app shows.
pub(crate) fn shown_file(worktree: &Path) -> Result<String, String> {
    let reply = request(worktree, &json!({ "op": "shown" }))?;
    reply["fileID"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "the preview app named no file".into())
}

/// The helper knows a preview by its `fileID`, `Module/File.swift`, and the worktree path's module is unknown.
// shortcut: two modules' files of the same name with previews mix up; pass the module once the picker reads it from the compile commands.
fn file_name(pick: &PreviewPick) -> &str {
    pick.file.rsplit('/').next().unwrap_or(&pick.file)
}

/// Has the preview app show another `#Preview` from the image it launched with.
pub(crate) fn show(worktree: &Path, pick: &PreviewPick) -> Result<(), String> {
    request(
        worktree,
        &json!({ "op": "show", "file": file_name(pick), "index": pick.index }),
    )
    .map(drop)
}

/// Has the preview app load a hot reload's dylib and show the preview from it.
pub(crate) fn load(worktree: &Path, dylib: &Path) -> Result<(), String> {
    request(worktree, &json!({ "op": "load", "path": dylib })).map(drop)
}

/// The helper dylib, built on first use for each Xcode build and helper source.
fn helper() -> Result<PathBuf, Error> {
    static BUILDING: Mutex<()> = Mutex::new(());
    let _building = BUILDING.lock().unwrap_or_else(PoisonError::into_inner);
    let cache = CACHE_FOLDER
        .get()
        .ok_or_else(|| Error::Message("Chord has no cache folder.".into()))?;
    let mut hasher = DefaultHasher::new();
    SOURCE.hash(&mut hasher);
    let folder = cache.join("PreviewHelper").join(format!(
        "{}-{:08x}",
        xcode_build(),
        hasher.finish() as u32
    ));
    let dylib = folder.join("ChordPreview.dylib");
    if dylib.exists() {
        return Ok(dylib);
    }
    std::fs::create_dir_all(&folder)?;
    let source = folder.join("ChordPreview.swift");
    std::fs::write(&source, SOURCE)?;
    let partial = folder.join("ChordPreview.partial.dylib");
    let (source, partial_text) = (source.to_string_lossy(), partial.to_string_lossy());
    let mut args = vec![
        "-sdk",
        "iphonesimulator",
        "swiftc",
        "-emit-library",
        "-target",
        "arm64-apple-ios17.0-simulator",
        "-module-name",
        "ChordPreview",
        "-parse-as-library",
        "-Onone",
        "-enable-experimental-feature",
        "Extern",
    ];
    let aliases = ["swiftui", "controller", "view"].map(|kind| {
        [
            format!("_chord_{kind}_init"),
            format!("_chord_{kind}_init_alias"),
        ]
    });
    for [function, alias] in &aliases {
        args.extend([
            "-Xlinker", "-alias", "-Xlinker", function, "-Xlinker", alias,
        ]);
    }
    args.extend(["-o", &partial_text, &source]);
    let output = process::captured(login_shell("xcrun", &args), &|_| {})?;
    if !output.status.success() {
        let errors = String::from_utf8_lossy(&output.stderr);
        let first = errors
            .lines()
            .find(|line| line.contains("error"))
            .unwrap_or("swiftc failed");
        return Err(Error::Message(format!(
            "The preview helper did not build: {first}"
        )));
    }
    std::fs::rename(&partial, &dylib)?;
    Ok(dylib)
}

/// "27A9275" from `xcodebuild -version`, read once per app run.
fn xcode_build() -> &'static str {
    static BUILD: OnceLock<String> = OnceLock::new();
    BUILD.get_or_init(|| {
        let output = process::captured(login_shell("xcodebuild", &["-version"]), &|_| {});
        let text = output
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
            .unwrap_or_default();
        text.lines()
            .find_map(|line| line.strip_prefix("Build version "))
            .unwrap_or("unknown")
            .trim()
            .to_string()
    })
}
