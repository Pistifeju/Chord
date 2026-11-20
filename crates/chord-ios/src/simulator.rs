//! The worktree's simulators, its run simulator in the default device set and its preview simulator in Chord's own: found or created by name, booted, the app installed and launched on it with its output and OSLog read, and the Simulator app opened.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use chord_core::{Error, RunDestination, login_shell};
use serde_json::Value;

use crate::console::{self, ConsoleTone};
use crate::derived_data;
use crate::preview;
use crate::process::{self, end_group, xcrun};
use crate::slim;
use crate::xcode;

pub(crate) const PLUTIL: &str = "/usr/bin/plutil";
pub(crate) const OPEN: &str = "/usr/bin/open";
/// How long the launch waits for the log stream to start; it takes about a second.
const STREAM_START: Duration = Duration::from_secs(3);
/// How long a boot may take: a first boot takes about 30 s alone and a few times that beside cold builds; past this it hangs.
const BOOT_LIMIT: Duration = Duration::from_secs(180);

const PREFIX: &str = "Chord ";
const DEV_PREFIX: &str = "ChordDev ";
static DEV_NAMES: AtomicBool = AtomicBool::new(false);

/// Call once at launch from the dev build: its simulators get names of their own, so neither build prunes the other's.
pub fn use_dev_simulator_names() {
    DEV_NAMES.store(true, Ordering::Relaxed);
}

fn prefix() -> &'static str {
    if DEV_NAMES.load(Ordering::Relaxed) {
        DEV_PREFIX
    } else {
        PREFIX
    }
}

/// Whether either build of Chord named the simulator.
pub(crate) fn is_chord_name(name: &str) -> bool {
    name.starts_with(PREFIX) || name.starts_with(DEV_PREFIX)
}

/// Where a simulator lives: the default device set, which `xcodebuild`, Simulator and DeviceHub see, or Chord's own set of preview simulators, which they do not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DeviceSet {
    Run,
    Preview,
}

/// The `xcrun` arguments that run `simctl` with `args` in the set.
fn simctl_args(set: DeviceSet, args: &[&str]) -> Result<Vec<String>, Error> {
    let mut all = vec!["simctl".to_string()];
    if set == DeviceSet::Preview {
        let folder = preview::devices_folder()
            .ok_or_else(|| Error::Message("Chord has no preview device folder.".into()))?;
        all.extend(["--set".into(), folder.to_string_lossy().into_owned()]);
    }
    all.extend(args.iter().map(|arg| arg.to_string()));
    Ok(all)
}

/// The set and UDID of the worktree's booted run simulator, or with `preview` its preview simulator.
pub(crate) fn booted(worktree: &Path, preview: bool) -> Result<(DeviceSet, String), Error> {
    xcode::require_tools()?;
    let set = if preview {
        DeviceSet::Preview
    } else {
        DeviceSet::Run
    };
    let device = find(set, worktree)?
        .filter(|device| device.booted)
        .ok_or_else(|| Error::Message("The simulator is off. Run boots it.".into()))?;
    Ok((set, device.udid))
}

/// `simctl` with `args` in the set, through `xcrun`, run to its end.
pub(crate) fn simctl(
    set: DeviceSet,
    args: &[&str],
    register: &dyn Fn(u32),
) -> Result<String, Error> {
    let args = simctl_args(set, args)?;
    xcrun(
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
        register,
    )
}

/// `simctl` with `args` in the set, through `xcrun` in a login shell, for a caller that streams its output or sets its environment.
pub(crate) fn simctl_command(set: DeviceSet, args: &[&str]) -> Result<Command, Error> {
    let args = simctl_args(set, args)?;
    Ok(login_shell(
        "xcrun",
        &args.iter().map(String::as_str).collect::<Vec<_>>(),
    ))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Simulator {
    pub udid: String,
    pub name: String,
    pub booted: bool,
    /// The device type identifier.
    pub model: String,
    /// The runtime identifier.
    pub runtime: String,
}

/// `Chord <worktree folder> <6 letters of the path hash>`, `ChordDev …` in the dev build: the name is the only link between a worktree and its simulators.
pub(crate) fn name_of(worktree: &Path) -> String {
    let folder = worktree
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let hash = derived_data::path_hash(&worktree.to_string_lossy());
    format!("{}{folder} {}", prefix(), &hash[..6])
}

fn available(device: &Value) -> bool {
    device["isAvailable"] == true
}

/// The devices of the list that `matches`, those of a removed runtime too.
fn devices_where<'a>(
    listed: &'a Value,
    matches: impl Fn(&Value) -> bool + 'a,
) -> impl Iterator<Item = Simulator> + 'a {
    listed["devices"]
        .as_object()
        .into_iter()
        .flatten()
        .flat_map(|(runtime, devices)| {
            devices
                .as_array()
                .into_iter()
                .flatten()
                .map(move |device| (runtime, device))
        })
        .filter(move |(_, device)| matches(device))
        .map(|(runtime, device)| Simulator {
            udid: device["udid"].as_str().unwrap_or_default().into(),
            name: device["name"].as_str().unwrap_or_default().into(),
            booted: device["state"] == "Booted",
            model: device["deviceTypeIdentifier"]
                .as_str()
                .unwrap_or_default()
                .into(),
            runtime: runtime.clone(),
        })
}

fn device_where(listed: &Value, matches: impl Fn(&Value) -> bool) -> Option<Simulator> {
    devices_where(listed, |device| available(device) && matches(device)).next()
}

/// The simulators this build of Chord created in the set, by their name's shape: `Chord <folder> <6 letters>`. Those of a removed runtime count, so a delete and the build data's size reach them.
pub(crate) fn owned(set: DeviceSet) -> Result<Vec<Simulator>, Error> {
    Ok(devices_where(&list_devices(set)?, |device| {
        device["name"].as_str().and_then(worktree_folder).is_some()
    })
    .collect())
}

/// The worktree folder a Chord simulator's name holds; None for any other name.
pub(crate) fn worktree_folder(name: &str) -> Option<&str> {
    let (folder, hash) = name.strip_prefix(prefix())?.rsplit_once(' ')?;
    let is_hash = hash.len() == 6 && hash.bytes().all(|byte| byte.is_ascii_lowercase());
    (is_hash && !folder.is_empty()).then_some(folder)
}

/// Shuts the simulator down and waits for it. No error when it is not booted.
pub(crate) fn shut_down(set: DeviceSet, udid: &str) {
    let _ = simctl(set, &["shutdown", udid], &|_| {});
}

/// Shuts the simulators down, all at once, and waits for every one.
pub(crate) fn shut_down_all(set: DeviceSet, udids: Vec<String>) {
    let ending: Vec<_> = udids
        .into_iter()
        .map(|udid| thread::spawn(move || shut_down(set, &udid)))
        .collect();
    for handle in ending {
        let _ = handle.join();
    }
}

/// Shuts the simulator down and deletes it with its data.
pub(crate) fn delete(set: DeviceSet, udid: &str) -> Result<(), Error> {
    shut_down(set, udid);
    simctl(set, &["delete", udid], &|_| {}).map(drop)
}

/// Starts the simulator's shutdown and does not wait for it.
pub(crate) fn start_shut_down(set: DeviceSet, udid: &str) {
    let Ok(mut command) = simctl_command(set, &["shutdown", udid]) else {
        return;
    };
    let spawned = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    if let Ok(mut child) = spawned {
        thread::spawn(move || child.wait());
    }
}

fn devices_named(listed: &Value, name: &str) -> Vec<Simulator> {
    devices_where(listed, |device| available(device) && device["name"] == name).collect()
}

/// A worktree has one run simulator per model it ran on: the booted one, else the first.
fn device_named(listed: &Value, name: &str) -> Option<Simulator> {
    let mut named = devices_named(listed, name);
    let at = named.iter().position(|device| device.booted).unwrap_or(0);
    (!named.is_empty()).then(|| named.swap_remove(at))
}

fn list_devices(set: DeviceSet) -> Result<Value, Error> {
    let listed = simctl(set, &["list", "devices", "--json"], &|_| {})?;
    Ok(serde_json::from_str(&listed)?)
}

/// The worktree's simulator in the set, if it exists.
pub(crate) fn find(set: DeviceSet, worktree: &Path) -> Result<Option<Simulator>, Error> {
    Ok(device_named(&list_devices(set)?, &name_of(worktree)))
}

/// Whether the device of the set reports Booted.
pub(crate) fn is_booted(set: DeviceSet, udid: &str) -> bool {
    list_devices(set)
        .ok()
        .and_then(|listed| device_where(&listed, |device| device["udid"] == udid))
        .is_some_and(|device| device.booted)
}

/// The worktree's simulator in the set, created on first use: `wanted` (model and runtime identifiers), else the newest iOS runtime and the model it lists that reads best. The worktree's run simulators of another model or runtime shut down and keep their apps' data; a preview simulator of another one holds none and is deleted.
pub(crate) fn find_or_create(
    set: DeviceSet,
    worktree: &Path,
    wanted: Option<(&str, &str)>,
    register: &dyn Fn(u32),
) -> Result<Simulator, Error> {
    let (mut fitting, others): (Vec<Simulator>, Vec<Simulator>) =
        devices_named(&list_devices(set)?, &name_of(worktree))
            .into_iter()
            .partition(|device| {
                wanted.is_none_or(|(model, runtime)| {
                    device.model == model && device.runtime == runtime
                })
            });
    for other in others {
        if other.booted || set == DeviceSet::Preview {
            let _ = simctl(set, &["shutdown", &other.udid], register);
        }
        if set == DeviceSet::Preview {
            simctl(set, &["delete", &other.udid], register)?;
        }
    }
    if !fitting.is_empty() {
        let at = fitting.iter().position(|device| device.booted).unwrap_or(0);
        return Ok(fitting.swap_remove(at));
    }
    let (device_type, runtime) = match wanted {
        Some((model, runtime)) => (model.to_string(), runtime.to_string()),
        None => {
            let runtimes = simctl(DeviceSet::Run, &["list", "runtimes", "--json"], register)?;
            match newest_ios_model(&serde_json::from_str(&runtimes)?) {
                Some(RunDestination::Simulator { model, runtime, .. }) => (model, runtime),
                _ => {
                    return Err(Error::Message(
                        "No iOS simulator runtime is installed. Add one in Xcode > Settings > Components."
                            .into(),
                    ));
                }
            }
        }
    };
    let udid = simctl(
        set,
        &["create", &name_of(worktree), &device_type, &runtime],
        register,
    )?;
    Ok(Simulator {
        udid: udid.trim().into(),
        name: name_of(worktree),
        booted: false,
        model: device_type,
        runtime,
    })
}

fn version_of(runtime: &Value) -> Vec<u32> {
    dotted_version(runtime["version"].as_str().unwrap_or_default())
}

/// `1174.9.2` as `[1174, 9, 2]`, comparable part by part.
pub(crate) fn dotted_version(text: &str) -> Vec<u32> {
    text.split('.')
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// The newest available iOS runtime and its iPhone model: a Pro that is not a Max, else the last iPhone it lists. A worktree without a picked model gets a simulator of it.
pub(crate) fn newest_ios_model(listed: &Value) -> Option<RunDestination> {
    let runtime = listed["runtimes"]
        .as_array()?
        .iter()
        .filter(|runtime| runtime["platform"] == "iOS" && runtime["isAvailable"] == true)
        .max_by_key(|runtime| version_of(runtime))?;
    let phones: Vec<&Value> = runtime["supportedDeviceTypes"]
        .as_array()?
        .iter()
        .filter(|model| model["productFamily"] == "iPhone")
        .collect();
    let named = |model: &&&Value| model["name"].as_str().unwrap_or_default().to_string();
    let model = phones
        .iter()
        .rev()
        .find(|model| {
            let name = named(model);
            name.contains("Pro") && !name.contains("Max")
        })
        .or(phones.last())?;
    Some(RunDestination::Simulator {
        model: model["identifier"].as_str()?.into(),
        runtime: runtime["identifier"].as_str()?.into(),
        name: model["name"].as_str()?.into(),
        os: runtime["name"].as_str()?.into(),
    })
}

/// Boots the simulator and waits until it reports Booted. A device that does not fails with its state; a boot past `BOOT_LIMIT` is ended and fails.
pub(crate) fn boot(set: DeviceSet, udid: &str) -> Result<(), Error> {
    let group = AtomicU32::new(0);
    let (done, waits) = mpsc::channel::<()>();
    let late = thread::scope(|scope| {
        let group = &group;
        let watchdog = scope.spawn(move || {
            let late = waits.recv_timeout(BOOT_LIMIT) == Err(RecvTimeoutError::Timeout);
            let pid = group.load(Ordering::Relaxed);
            if late && pid != 0 {
                end_group(pid);
            }
            late
        });
        let booted = simctl(set, &["bootstatus", udid, "-b"], &|pid| {
            group.store(pid, Ordering::Relaxed)
        });
        drop(done);
        match watchdog.join() {
            Ok(true) => Ok(true),
            _ => booted.map(|_| false),
        }
    })?;
    if late {
        start_shut_down(set, udid);
        return Err(Error::Message(format!(
            "The simulator did not boot within {} minutes. Chord shut it down; run again.",
            BOOT_LIMIT.as_secs() / 60
        )));
    }
    if is_booted(set, udid) {
        Ok(())
    } else {
        Err(Error::Message(
            "The simulator does not report Booted after its boot.".into(),
        ))
    }
}

/// After a boot, the first install or uninstall waits about 30 s while the device registers its system apps again; uninstalling an app that does not exist pays that wait early, during the build.
pub(crate) fn warm_up(set: DeviceSet, udid: &str) {
    let _ = simctl(set, &["uninstall", udid, "chord.warm-up"], &|_| {});
}

/// Boots the simulator, slim when it is one of Chord's own, shut down and on iOS 18.5 or newer: the overrides of the services the user does not keep go in before the boot and are read back after it. Returns a note when slimming was skipped or did not fully hold; the run goes on either way.
pub(crate) fn boot_slim(
    device: &Simulator,
    kept_services: &BTreeMap<String, bool>,
) -> Result<Option<&'static str>, Error> {
    let slims = !device.booted
        && worktree_folder(&device.name).is_some()
        && slim::runtime_slims(&device.runtime);
    let written = slims.then(|| slim::write_overrides(&device.udid, kept_services));
    boot(DeviceSet::Run, &device.udid)?;
    Ok(match written {
        Some(Err(_)) => Some("Chord could not slim the simulator; it runs with every service."),
        Some(Ok(off)) => (slim::holds(&device.udid, &off) == Some(false))
            .then_some("The simulator did not keep all of its slimming; some services may run."),
        None => None,
    })
}

/// The selected Xcode's `Contents/Developer` folder.
pub(crate) fn developer_dir() -> Option<PathBuf> {
    let xcodebuild = xcrun(&["--find", "xcodebuild"], &|_| {}).ok()?;
    // `<Xcode>/Contents/Developer/usr/bin/xcodebuild`
    Some(
        Path::new(xcodebuild.trim())
            .ancestors()
            .nth(3)?
            .to_path_buf(),
    )
}

/// The selected Xcode's simulator app: DeviceHub from Xcode 27, Simulator.app before.
pub(crate) fn simulator_app() -> Option<PathBuf> {
    let developer = developer_dir()?;
    [
        developer.join("../Applications/DeviceHub.app"),
        developer.join("Applications/Simulator.app"),
    ]
    .into_iter()
    .find(|app| app.exists())
}

/// Opens the selected Xcode's simulator app, behind the window unless `front`.
pub fn open_app(front: bool) -> Result<(), Error> {
    xcode::require_tools()?;
    let app = simulator_app()
        .ok_or_else(|| Error::Message("The selected Xcode has no simulator app.".into()))?;
    let mut open = Command::new(OPEN);
    if !front {
        open.arg("-g");
    }
    if open.arg(&app).status()?.success() {
        Ok(())
    } else {
        Err(Error::Message(format!("{} did not open.", app.display())))
    }
}

/// Shake: the Darwin notification Simulator.app's Device > Shake posts; UIKit turns it into a shake motion.
pub(crate) fn shake(udid: &str) -> Result<(), Error> {
    let notification = "com.apple.UIKit.SimulatorShake";
    simctl(
        DeviceSet::Run,
        &["spawn", udid, "notifyutil", "-p", notification],
        &|_| {},
    )
    .map(drop)
}

const LOCK_STATE: &str = "com.apple.springboard.lockstate";

/// The lock state in a `notifyutil` line: `<key> <state>`, or `-v`'s `[n] <time> <key> <state> <type>`.
fn locked_in(line: &str) -> Option<bool> {
    let mut words = line.split_whitespace();
    words.find(|word| *word == LOCK_STATE)?;
    words.next()?.parse::<u64>().ok().map(|state| state != 0)
}

/// SpringBoard's lock state now; None when the simulator does not answer.
pub(crate) fn lock_state(udid: &str) -> Option<bool> {
    let output = simctl(
        DeviceSet::Run,
        &["spawn", udid, "notifyutil", "-g", LOCK_STATE],
        &|_| {},
    );
    locked_in(&output.ok()?)
}

/// Follows SpringBoard's lock state: `on_change` gets it now and on every change, on a thread of its own, until the simulator shuts down or `stop_lock_watch` ends it. The watch's process group, or None when xcrun does not start.
pub(crate) fn watch_lock(udid: &str, on_change: impl Fn(bool) + Send + 'static) -> Option<u32> {
    let watch = simctl_command(
        DeviceSet::Run,
        &["spawn", udid, "notifyutil", "-v", "-w", LOCK_STATE],
    )
    .ok()?;
    let (mut child, reader) = process::merged(watch, None).ok()?;
    let group = child.id();
    let udid = udid.to_string();
    thread::spawn(move || {
        // The watch starts first: a change between the two still arrives. `-g` goes alone because notifyutil buffers it until the first notification.
        if let Some(locked) = lock_state(&udid) {
            on_change(locked);
        }
        for line in std::io::BufRead::lines(std::io::BufReader::new(reader)).map_while(Result::ok) {
            if let Some(locked) = locked_in(&line) {
                on_change(locked);
            }
        }
        let _ = child.wait();
    });
    Some(group)
}

pub(crate) fn stop_lock_watch(group: u32) {
    process::signal_group(group, libc::SIGTERM);
}

/// Home where the Indigo home press is dropped: launching SpringBoard shows the home screen.
pub(crate) fn show_home_screen(udid: &str) -> Result<(), Error> {
    simctl(
        DeviceSet::Run,
        &["launch", udid, "com.apple.springboard"],
        &|_| {},
    )
    .map(drop)
}

/// A text value of the app's Info.plist, such as `CFBundleIdentifier`.
pub(crate) fn plist_text(app: &Path, key: &str, register: &dyn Fn(u32)) -> Result<String, Error> {
    plist_value(&app.join("Info.plist"), key, register)
}

/// A text value of a plist file.
pub(crate) fn plist_value(file: &Path, key: &str, register: &dyn Fn(u32)) -> Result<String, Error> {
    let mut command = Command::new(PLUTIL);
    command.args(["-extract", key, "raw", "-o", "-"]).arg(file);
    let output = process::captured(command, register)?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !value.is_empty() {
        Ok(value)
    } else {
        Err(Error::Message(format!("No {key} in {}", file.display())))
    }
}

pub(crate) fn install(
    set: DeviceSet,
    udid: &str,
    app: &Path,
    register: &dyn Fn(u32),
) -> Result<(), Error> {
    simctl(set, &["install", udid, &app.to_string_lossy()], register).map(drop)
}

/// Ends the app; no error when it does not run.
pub(crate) fn terminate(udid: &str, bundle_id: &str) {
    let _ = simctl(DeviceSet::Run, &["terminate", udid, bundle_id], &|_| {});
}

/// One record of `log stream --style ndjson` as a console line and its tone; None for a line that is no record, such as the stream's own notices. A multi-line message stays one record.
fn oslog_line(json: &str) -> Option<(String, ConsoleTone)> {
    let record: Value = serde_json::from_str(json).ok()?;
    let message = record["eventMessage"].as_str()?;
    let subsystem = record["subsystem"].as_str().unwrap_or_default();
    let tone = match record["messageType"].as_str() {
        Some("Error" | "Fault") => ConsoleTone::Error,
        _ => ConsoleTone::Output,
    };
    let text = if subsystem.is_empty() {
        format!("{message}\n")
    } else {
        format!("{subsystem}: {message}\n")
    };
    Some((text, tone))
}

/// Ends the log stream with its group, or its process inside the simulator outlives the run, and reaps it.
fn end_stream(stream: &mut Child) {
    let id = stream.id();
    thread::spawn(move || end_group(id));
    let _ = stream.wait();
}

/// Launches the app, replacing a running copy, and returns its exit code (None when a signal ended it) when it ends. Its stdout and stderr, and the OSLog messages of its process `executable`, go to `console` as they come. `launched` is called once simctl reports the app's process. An app that never launches fails with simctl's message.
pub(crate) fn launch(
    udid: &str,
    bundle_id: &str,
    executable: &str,
    register: &dyn Fn(u32),
    launched: &dyn Fn(),
    console: &dyn Fn(String, ConsoleTone),
) -> Result<Option<i32>, Error> {
    // The log stream starts first and the launch joins its process group, so one stop ends both.
    // NSLog writes to stderr and to OSLog (sender Foundation): the predicate keeps it to one copy.
    let predicate = format!(
        "process == \"{}\" AND sender != \"Foundation\"",
        executable.replace('"', "\\\"")
    );
    let stream = simctl_command(
        DeviceSet::Run,
        &[
            "spawn",
            udid,
            "log",
            "stream",
            "--style",
            "ndjson",
            "--predicate",
            &predicate,
        ],
    )?;
    // --console-pty gives the app a terminal, so its print output is line-buffered, and simctl exits with the app's exit code.
    let command = simctl_command(
        DeviceSet::Run,
        &[
            "launch",
            "--console-pty",
            "--terminate-running-process",
            udid,
            bundle_id,
        ],
    )?;
    let (mut stream, stream_output) = process::merged(stream, None)?;
    register(stream.id());
    // Read on a thread that may outlive the launch: the stream's process in the simulator can hold the pipe open after a kill.
    let (records, received_records) = mpsc::channel();
    let (first_line, stream_started) = mpsc::channel();
    let stream_lines = process::read_lines(stream_output);
    thread::spawn(move || {
        for line in stream_lines {
            let _ = first_line.send(());
            let record = oslog_line(&String::from_utf8_lossy(&line));
            if record.is_some_and(|record| records.send(record).is_err()) {
                return;
            }
        }
    });
    // The stream prints its filter notice once it runs; the launch waits for it so the app's first lines are caught.
    let _ = stream_started.recv_timeout(STREAM_START);
    let (mut child, reader) = match process::merged_on_terminal(command, Some(stream.id())) {
        Ok(started) => started,
        Err(error) => {
            let ended = stream.try_wait();
            end_stream(&mut stream);
            return Err(match ended {
                Ok(Some(status)) => Error::Message(format!(
                    "The OSLog stream ended before the app launched: {status}"
                )),
                _ => error,
            });
        }
    };
    let lines = process::read_lines(reader);
    // The pty ends lines with \r\n.
    let pumped = console::pump(
        &lines,
        Some(&received_records),
        |text| text.starts_with(&format!("{bundle_id}: ")),
        launched,
        console,
        || end_stream(&mut stream),
    );
    let (up, output) = (pumped.up, pumped.output);
    let status = child.wait()?;
    if up {
        return Ok(status.code());
    }
    let output = output.trim();
    Err(Error::Message(if output.is_empty() {
        format!("The app did not launch: simctl exited with {status}")
    } else {
        output.to_string()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_simulator_by_folder_and_six_letters_of_the_path_hash() {
        assert_eq!(name_of(Path::new("/tmp/Example")), "Chord Example euhuvy");
    }

    #[test]
    fn finds_the_available_device_with_the_name() {
        let listed: Value = serde_json::from_str(
            r#"{"devices":{"rt-a":[{"name":"Chord w abcdef","udid":"U1","isAvailable":false,"state":"Shutdown"}],
                "rt-b":[{"name":"Other","udid":"U2","isAvailable":true,"state":"Booted"},
                        {"name":"Chord w abcdef","udid":"U3","isAvailable":true,"state":"Booted","deviceTypeIdentifier":"type-b"}]}}"#,
        )
        .unwrap();
        assert_eq!(
            device_named(&listed, "Chord w abcdef"),
            Some(Simulator {
                udid: "U3".into(),
                name: "Chord w abcdef".into(),
                booted: true,
                model: "type-b".into(),
                runtime: "rt-b".into()
            })
        );
        assert_eq!(device_named(&listed, "Missing"), None);
    }

    #[test]
    fn reads_the_worktree_folder_of_a_chord_simulator_name_only() {
        assert_eq!(
            worktree_folder("Chord feature-login euhuvy"),
            Some("feature-login")
        );
        assert_eq!(worktree_folder("Chord my app euhuvy"), Some("my app"));
        assert_eq!(worktree_folder("iPhone 17 Pro"), None);
        assert_eq!(worktree_folder("Chord feature-login Euhuvy"), None);
        assert_eq!(worktree_folder("Chord euhuvy"), None);
    }

    #[test]
    fn picks_the_newest_available_ios_runtime_and_a_pro_that_is_no_max() {
        let listed: Value = serde_json::from_str(
            r#"{"runtimes":[
              {"platform":"iOS","isAvailable":true,"version":"26.4.1","identifier":"ios-26-4","supportedDeviceTypes":[]},
              {"platform":"iOS","isAvailable":true,"version":"27.1","identifier":"ios-27-1","name":"iOS 27.1","supportedDeviceTypes":[
                {"productFamily":"iPhone","name":"iPhone 18","identifier":"p18"},
                {"productFamily":"iPhone","name":"iPhone 18 Pro","identifier":"p18pro"},
                {"productFamily":"iPhone","name":"iPhone 18 Pro Max","identifier":"p18max"},
                {"productFamily":"iPad","name":"iPad Pro","identifier":"ipad"}]},
              {"platform":"iOS","isAvailable":false,"version":"28.0","identifier":"ios-28-0","supportedDeviceTypes":[]},
              {"platform":"watchOS","isAvailable":true,"version":"29.0","identifier":"watch","supportedDeviceTypes":[]}]}"#,
        )
        .unwrap();
        assert_eq!(
            newest_ios_model(&listed),
            Some(RunDestination::Simulator {
                model: "p18pro".into(),
                runtime: "ios-27-1".into(),
                name: "iPhone 18 Pro".into(),
                os: "iOS 27.1".into(),
            })
        );
    }

    #[test]
    fn reads_the_lock_state_of_both_notifyutil_lines() {
        assert_eq!(locked_in("com.apple.springboard.lockstate 1\n"), Some(true));
        assert_eq!(
            locked_in("[3] 1794833259.759 com.apple.springboard.lockstate 0 port"),
            Some(false)
        );
        assert_eq!(locked_in("notifyutil: no such device"), None);
    }

    #[test]
    fn reads_an_oslog_record_whole_and_tones_errors() {
        assert_eq!(
            oslog_line(
                r#"{"eventMessage":"line one\nline two","messageType":"Default","subsystem":"com.example.app"}"#
            ),
            Some((
                "com.example.app: line one\nline two\n".into(),
                ConsoleTone::Output
            ))
        );
        assert_eq!(
            oslog_line(r#"{"eventMessage":"boom","messageType":"Fault","subsystem":""}"#),
            Some(("boom\n".into(), ConsoleTone::Error))
        );
        assert_eq!(oslog_line("Filtering the log data using"), None);
    }
}
