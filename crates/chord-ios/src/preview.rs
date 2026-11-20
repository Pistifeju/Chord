//! The worktree's preview simulator in Chord's own device set, at most two booted at once, and the app launched on it with the preview helper (ChordPreview.swift), which shows the picked `#Preview` full screen and answers on a socket for a new pick and the hot reload.

use chord_core::lock;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use chord_core::{Error, PreviewPick, login_shell};
use serde::Serialize;
use serde_json::{Value, json};
use specta::Type;

use crate::derived_data::path_hash;
use crate::hot_reload;
use crate::issues::BuildIssue;
use crate::process;
use crate::simulator::{self, DeviceSet};

const SOURCE: &str = include_str!("ChordPreview.swift");
const MAX_BOOTED: usize = 2;
/// How long a launched preview app may take to open its socket: a first launch on a simulator that just booted, beside a build, takes many seconds.
const HELPER_START: Duration = Duration::from_secs(60);
/// The lines of the app's output kept for the reason it ended.
const KEPT_LINES: usize = 50;

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
    /// The first compile error of a preview build or hot reload; the last frame stays.
    Error {
        issue: BuildIssue,
    },
    Failed {
        message: String,
    },
}

/// A preview simulator's boot: empty while it runs, then its outcome.
pub(crate) type Boot = Arc<OnceLock<Result<(), String>>>;

pub(crate) enum Booted {
    /// It was booted already.
    Ready {
        udid: String,
    },
    /// It boots on a thread of its own, so no app runs on it yet.
    Booting {
        udid: String,
        boot: Boot,
    },
    Full(Vec<String>),
}

/// Held from the count of the booted preview simulators to the end of a boot, so two worktrees never boot a third one together.
// shortcut: a second worktree's Preview view waits for the first one's boot, up to 30 s; count booting devices instead if that shows.
static BOOTING: Mutex<()> = Mutex::new(());

/// The worktree's preview simulator of the `wanted` model, booting unless two other preview simulators run; one whose name is not in `open`, its Preview view closed, shuts down for it. Returns once the boot started; the build needs no booted preview simulator. `on_state` hears Starting before this returns and Failed as soon as the boot fails.
pub(crate) fn boot(
    worktree: &Path,
    wanted: Option<(&str, &str)>,
    open: HashSet<String>,
    on_state: Arc<dyn Fn(PreviewState) + Send + Sync>,
) -> Result<Booted, Error> {
    let worktree = worktree.to_path_buf();
    let wanted = wanted.map(|(model, runtime)| (model.to_string(), runtime.to_string()));
    let (verdict, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let _booting = lock(&BOOTING);
        let wanted = wanted
            .as_ref()
            .map(|(model, runtime)| (model.as_str(), runtime.as_str()));
        let found = (|| {
            let device = simulator::find_or_create(DeviceSet::Preview, &worktree, wanted, &|_| {})?;
            if device.booted {
                return Ok(Booted::Ready { udid: device.udid });
            }
            let mut others: Vec<simulator::Simulator> = simulator::owned(DeviceSet::Preview)?
                .into_iter()
                .filter(|other| other.booted && other.name != device.name)
                .collect();
            while others.len() >= MAX_BOOTED
                && let Some(at) = others.iter().position(|other| !open.contains(&other.name))
            {
                simulator::shut_down(DeviceSet::Preview, &others.swap_remove(at).udid);
            }
            if others.len() >= MAX_BOOTED {
                let mut folders: Vec<String> = others
                    .iter()
                    .filter_map(|other| simulator::worktree_folder(&other.name).map(String::from))
                    .collect();
                folders.sort();
                return Ok(Booted::Full(folders));
            }
            Ok(Booted::Booting {
                udid: device.udid,
                boot: Boot::default(),
            })
        })();
        let booting = match &found {
            Ok(Booted::Booting { udid, boot }) => Some((udid.clone(), Arc::clone(boot))),
            _ => None,
        };
        // Before the verdict, so it never lands after the Failed below.
        if booting.is_some() {
            on_state(PreviewState::Starting);
        }
        let _ = verdict.send(found);
        if let Some((udid, boot)) = booting {
            let booted =
                simulator::boot(DeviceSet::Preview, &udid).map_err(|error| error.to_string());
            let _ = boot.set(booted.clone());
            match booted {
                Ok(()) => {
                    std::thread::spawn(move || simulator::warm_up(DeviceSet::Preview, &udid));
                }
                Err(message) => on_state(PreviewState::Failed { message }),
            }
        }
    });
    heard.recv().unwrap_or_else(|_| {
        Err(Error::Message(
            "The preview simulator's boot crashed.".into(),
        ))
    })
}

/// Installs the app on the worktree's preview simulator, launches it with the helper in place of its entry point and returns once the helper's socket is there. The launch stays attached: `on_exit` hears why the app ended, from a thread of its own.
pub(crate) fn launch(
    worktree: &Path,
    udid: &str,
    app: &Path,
    register: &dyn Fn(u32),
    on_exit: impl FnOnce(String) + Send + 'static,
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
    // --console-pty keeps simctl running until the app ends, with the app's output.
    let mut command = simulator::simctl_command(
        DeviceSet::Preview,
        &[
            "launch",
            "--console-pty",
            "--terminate-running-process",
            udid,
            &bundle,
        ],
    )?;
    let socket = socket(worktree);
    let _ = std::fs::remove_file(&socket);
    hot_reload::launched(worktree);
    command
        .env("SIMCTL_CHILD_DYLD_INSERT_LIBRARIES", &helper)
        .env("SIMCTL_CHILD_CHORD_PREVIEW_SOCKET", &socket);
    let (mut child, reader) = process::merged_on_terminal(command, None)?;
    register(child.id());
    let lines = process::read_lines(reader);
    let pid_line = format!("{bundle}: ");
    let mut said = VecDeque::new();
    let keep = move |line: Vec<u8>, said: &mut VecDeque<String>| {
        let text = String::from_utf8_lossy(&line).trim().to_string();
        if !text.is_empty() && !text.starts_with(&pid_line) {
            if said.len() == KEPT_LINES {
                said.pop_front();
            }
            said.push_back(text);
        }
    };
    let until = Instant::now() + HELPER_START;
    while !socket.exists() {
        if Instant::now() > until {
            std::thread::spawn(move || child.wait());
            return Err(Error::Message(format!(
                "The preview app did not start within {} s.",
                HELPER_START.as_secs()
            )));
        }
        match lines.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => keep(line, &mut said),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                let _ = child.wait();
                return Err(Error::Message(end_reason(said.make_contiguous())));
            }
        }
    }
    std::thread::spawn(move || {
        for line in lines {
            keep(line, &mut said);
        }
        let _ = child.wait();
        on_exit(end_reason(said.make_contiguous()));
    });
    Ok(())
}

/// Why the preview app ended, from its last output: the crash line when it printed one, else its last line.
fn end_reason(said: &[String]) -> String {
    let crash = said
        .iter()
        .rev()
        .find(|line| line.contains("Fatal error") || line.contains("Terminating app due to"));
    match crash.or(said.last()) {
        Some(line) => format!("The preview app ended: {line}"),
        None => "The preview app ended.".into(),
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
    let _building = lock(&BUILDING);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_crash_line_else_the_last_line_as_the_end_reason() {
        let said = |lines: &[&str]| {
            lines
                .iter()
                .map(|line| line.to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            end_reason(&said(&[
                "App/Row.swift:12: Fatal error: No Observable object of type Store found",
                "note after the crash"
            ])),
            "The preview app ended: App/Row.swift:12: Fatal error: No Observable object of type Store found"
        );
        assert_eq!(
            end_reason(&said(&["first", "last"])),
            "The preview app ended: last"
        );
        assert_eq!(end_reason(&[]), "The preview app ended.");
    }
}
