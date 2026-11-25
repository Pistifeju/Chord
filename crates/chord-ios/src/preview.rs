use chord_core::lock;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
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
use crate::plist;
use crate::process;
use crate::signature;
use crate::simulator::{self, DeviceSet};

const CAPTURE: &str = include_str!("ChordPreview.swift");
const MAX_BOOTED: usize = 2;
const HELPER_START: Duration = Duration::from_secs(60);
const KEPT_LINES: usize = 50;

static CACHE_FOLDER: OnceLock<PathBuf> = OnceLock::new();

pub fn set_cache_folder(folder: &Path) {
    let _ = std::fs::create_dir_all(folder.join("PreviewDevices"));
    let _ = CACHE_FOLDER.set(folder.to_path_buf());
}

pub(crate) fn devices_folder() -> Option<PathBuf> {
    CACHE_FOLDER
        .get()
        .map(|folder| folder.join("PreviewDevices"))
}

pub(crate) fn hot_reload_folders() -> Option<PathBuf> {
    CACHE_FOLDER.get().map(|folder| folder.join("HotReload"))
}

pub(crate) fn hot_reload_folder(worktree: &Path) -> Option<PathBuf> {
    hot_reload_folders().map(|folder| folder.join(simulator::name_of(worktree)))
}

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PreviewState {
    Starting,
    Waiting,
    Building,
    Live,
    Full { worktrees: Vec<String> },
    Unpicked,
    Error { issue: BuildIssue },
    Failed { message: String },
}

pub(crate) type Boot = Arc<OnceLock<Result<(), String>>>;

pub(crate) enum Booted {
    Ready { udid: String },
    Booting { udid: String, boot: Boot },
    Full(Vec<String>),
}

static BOOTING: Mutex<()> = Mutex::new(());

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

struct MacPreview {
    pid: u32,
    stream: Arc<Mutex<UnixStream>>,
}

static MAC_PREVIEWS: Mutex<BTreeMap<PathBuf, MacPreview>> = Mutex::new(BTreeMap::new());

pub(crate) fn take_mac(worktree: &Path) -> Option<u32> {
    lock(&MAC_PREVIEWS)
        .remove(worktree)
        .map(|preview| preview.pid)
}

pub(crate) fn take_all_mac() -> Vec<u32> {
    std::mem::take(&mut *lock(&MAC_PREVIEWS))
        .into_values()
        .map(|preview| preview.pid)
        .collect()
}

fn no_debug_dylib() -> Error {
    Error::Message(
        "Previews need the debug dylib: set Enable Debug Dylib Support to Yes in the app target's Debug build settings.".into(),
    )
}

pub(crate) fn launch(
    worktree: &Path,
    udid: &str,
    app: &Path,
    register: &dyn Fn(u32),
    on_exit: impl FnOnce(String) + Send + 'static,
) -> Result<(), Error> {
    let executable = plist::app_value(app, "CFBundleExecutable", register)?;
    if !app.join(format!("{executable}.debug.dylib")).exists() {
        return Err(no_debug_dylib());
    }
    let helper = helper(Platform::Simulator, None)?;
    let bundle = plist::app_value(app, "CFBundleIdentifier", register)?;
    simulator::install(DeviceSet::Preview, udid, app, register)?;
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
    hot_reload::launched(
        worktree,
        Platform::Simulator,
        &app.join(format!("{executable}.debug.dylib")),
        None,
    );
    command
        .env("SIMCTL_CHILD_DYLD_INSERT_LIBRARIES", &helper)
        .env("SIMCTL_CHILD_CHORD_PREVIEW_SOCKET", &socket);
    let (child, reader) = process::merged_on_terminal(command, None)?;
    register(child.id());
    follow(
        child,
        reader,
        Some(format!("{bundle}: ")),
        || socket.exists(),
        on_exit,
    )
}

pub(crate) fn launch_mac(
    worktree: &Path,
    app: &Path,
    register: &dyn Fn(u32),
    on_exit: impl FnOnce(String) + Send + 'static,
) -> Result<(), Error> {
    let executable = plist::value(
        &app.join("Contents/Info.plist"),
        "CFBundleExecutable",
        register,
    )?;
    let folder = app.join("Contents/MacOS");
    let debug_dylib = folder.join(format!("{executable}.debug.dylib"));
    if !debug_dylib.exists() {
        return Err(no_debug_dylib());
    }
    let identity = signature::identity(app).map_err(Error::Message)?;
    let helper = helper(Platform::Mac, identity.as_deref())?;
    hot_reload::launched(worktree, Platform::Mac, &debug_dylib, identity);
    let (ours, theirs) = UnixStream::pair()?;
    let fd = theirs.as_raw_fd();
    let mut command = Command::new(folder.join(executable));
    command
        .env("OS_ACTIVITY_DT_MODE", "YES")
        .env("DYLD_INSERT_LIBRARIES", &helper)
        .env("CHORD_PREVIEW_FD", fd.to_string());
    unsafe {
        command.pre_exec(move || match libc::fcntl(fd, libc::F_SETFD, 0) {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
    let (child, reader) = process::merged_on_terminal(command, None)?;
    drop(theirs);
    let pid = child.id();
    register(pid);
    writeln!(&ours, "{}", json!({ "op": "shown" }))?;
    let mut answer = BufReader::new(ours.try_clone()?);
    let (answered, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let read = answer.read_line(&mut line);
        let _ = answered.send(read.is_ok() && line.ends_with('\n'));
    });
    let path = worktree.to_path_buf();
    let followed = follow(
        child,
        reader,
        None,
        move || heard.try_recv() == Ok(true),
        move |message| {
            let mut previews = lock(&MAC_PREVIEWS);
            if previews
                .get(&path)
                .is_some_and(|preview| preview.pid == pid)
            {
                previews.remove(&path);
            }
            drop(previews);
            on_exit(message)
        },
    );
    match followed {
        Ok(()) => {
            lock(&MAC_PREVIEWS).insert(
                worktree.to_path_buf(),
                MacPreview {
                    pid,
                    stream: Arc::new(Mutex::new(ours)),
                },
            );
            Ok(())
        }
        Err(error) => {
            std::thread::spawn(move || process::end_group(pid));
            Err(error)
        }
    }
}

fn follow(
    mut child: Child,
    reader: File,
    simctl_prefix: Option<String>,
    mut ready: impl FnMut() -> bool,
    on_exit: impl FnOnce(String) + Send + 'static,
) -> Result<(), Error> {
    let lines = process::read_lines(reader);
    let mut said = VecDeque::new();
    let keep = move |line: Vec<u8>, said: &mut VecDeque<String>| {
        let text = String::from_utf8_lossy(&line).trim().to_string();
        let simctl = simctl_prefix
            .as_ref()
            .is_some_and(|prefix| text.starts_with(prefix.as_str()));
        if !text.is_empty() && !simctl {
            if said.len() == KEPT_LINES {
                said.pop_front();
            }
            said.push_back(text);
        }
    };
    let until = Instant::now() + HELPER_START;
    while !ready() {
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

fn socket(worktree: &Path) -> PathBuf {
    let hash = path_hash(&worktree.to_string_lossy());
    std::env::temp_dir().join(format!("chord-preview-{}.sock", &hash[..12]))
}

fn request(worktree: &Path, request: &Value) -> Result<Value, String> {
    let mac = lock(&MAC_PREVIEWS)
        .get(worktree)
        .map(|preview| Arc::clone(&preview.stream));
    if let Some(stream) = mac {
        let stream = lock(&stream);
        writeln!(&*stream, "{request}").map_err(|error| error.to_string())?;
        return read_answer(&stream);
    }
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
    writeln!(stream, "{request}").map_err(|error| error.to_string())?;
    read_answer(&stream)
}

fn read_answer(stream: &UnixStream) -> Result<Value, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| error.to_string())?;
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

pub(crate) fn shown_file(worktree: &Path) -> Result<String, String> {
    let reply = request(worktree, &json!({ "op": "shown" }))?;
    reply["fileID"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "the preview app named no file".into())
}

pub(crate) fn slide(worktree: &Path, image: &str) -> Result<u64, String> {
    let reply = request(worktree, &json!({ "op": "slide", "image": image }))?;
    reply["slide"]
        .as_i64()
        .map(|slide| slide as u64)
        .ok_or_else(|| "the preview app named no slide".into())
}

fn file_name(pick: &PreviewPick) -> &str {
    pick.file.rsplit('/').next().unwrap_or(&pick.file)
}

pub(crate) fn show(worktree: &Path, pick: &PreviewPick) -> Result<(), String> {
    request(
        worktree,
        &json!({ "op": "show", "file": file_name(pick), "index": pick.index }),
    )
    .map(drop)
}

pub(crate) fn load(worktree: &Path, dylib: &Path) -> Result<(), String> {
    request(worktree, &json!({ "op": "load", "path": dylib })).map(drop)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Platform {
    Simulator,
    Mac,
}

struct Shell {
    sdk: &'static str,
    target: &'static str,
    file: &'static str,
    source: &'static str,
}

impl Platform {
    fn shell(self) -> Shell {
        match self {
            Platform::Simulator => Shell {
                sdk: "iphonesimulator",
                target: "arm64-apple-ios17.0-simulator",
                file: "ChordPreviewUIKit.swift",
                source: include_str!("ChordPreviewUIKit.swift"),
            },
            Platform::Mac => Shell {
                sdk: "macosx",
                target: "arm64-apple-macos14.0",
                file: "ChordPreviewAppKit.swift",
                source: include_str!("ChordPreviewAppKit.swift"),
            },
        }
    }
}

fn helper(platform: Platform, identity: Option<&str>) -> Result<PathBuf, Error> {
    static BUILDING: Mutex<()> = Mutex::new(());
    let _building = lock(&BUILDING);
    let cache = CACHE_FOLDER
        .get()
        .ok_or_else(|| Error::Message("Chord has no cache folder.".into()))?;
    let shell = platform.shell();
    let mut hasher = DefaultHasher::new();
    CAPTURE.hash(&mut hasher);
    shell.source.hash(&mut hasher);
    let mut name = format!(
        "{}-{}-{:08x}",
        shell.sdk,
        xcode_build(),
        hasher.finish() as u32
    );
    if let Some(identity) = identity {
        name = format!("{name}-{identity}");
    }
    let folder = cache.join("PreviewHelper").join(name);
    let dylib = folder.join("ChordPreview.dylib");
    if dylib.exists() {
        return Ok(dylib);
    }
    std::fs::create_dir_all(&folder)?;
    let capture = folder.join("ChordPreview.swift");
    std::fs::write(&capture, CAPTURE)?;
    let shell_path = folder.join(shell.file);
    std::fs::write(&shell_path, shell.source)?;
    let partial = folder.join("ChordPreview.partial.dylib");
    let (capture, shell_path, partial_text) = (
        capture.to_string_lossy(),
        shell_path.to_string_lossy(),
        partial.to_string_lossy(),
    );
    let mut args = vec![
        "-sdk",
        shell.sdk,
        "swiftc",
        "-emit-library",
        "-target",
        shell.target,
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
    args.extend(["-o", &partial_text, &capture, &shell_path]);
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
    if let Some(identity) = identity {
        signature::sign(&partial, identity).map_err(Error::Message)?;
    }
    std::fs::rename(&partial, &dylib)?;
    Ok(dylib)
}

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
