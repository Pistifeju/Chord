//! The worktree's simulator: found or created by name, booted, the app installed and launched on it with its output and OSLog read, and the Simulator app opened.

use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use serde_json::Value;
use chord_core::{Error, login_shell};

use crate::console::{self, ConsoleTone};
use crate::derived_data;
use crate::process::{self, end_group, xcrun};

pub(crate) const PLUTIL: &str = "/usr/bin/plutil";
pub(crate) const OPEN: &str = "/usr/bin/open";
/// How long the launch waits for the log stream to start; it takes about a second.
const STREAM_START: Duration = Duration::from_secs(3);

#[derive(Debug, PartialEq)]
pub(crate) struct Simulator {
    pub udid: String,
    pub name: String,
    pub booted: bool,
    /// The device type identifier.
    pub model: String,
    /// The runtime identifier.
    pub runtime: String,
}

/// `Chord <worktree folder> <6 letters of the path hash>`: the name is the only link between a worktree and its simulator.
pub(crate) fn name_of(worktree: &Path) -> String {
    let folder = worktree
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let hash = derived_data::path_hash(&worktree.to_string_lossy());
    format!("Chord {folder} {}", &hash[..6])
}

/// The available devices of the list that `matches`.
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
        .filter(move |(_, device)| device["isAvailable"] == true && matches(device))
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
    devices_where(listed, matches).next()
}

/// The simulators Chord created, by their name's shape: `Chord <folder> <6 letters>`.
pub(crate) fn owned() -> Result<Vec<Simulator>, Error> {
    Ok(devices_where(&list_devices()?, |device| {
        device["name"].as_str().and_then(worktree_folder).is_some()
    })
    .collect())
}

/// The worktree folder a Chord simulator's name holds; None for any other name.
pub(crate) fn worktree_folder(name: &str) -> Option<&str> {
    let (folder, hash) = name.strip_prefix("Chord ")?.rsplit_once(' ')?;
    let is_hash = hash.len() == 6 && hash.bytes().all(|byte| byte.is_ascii_lowercase());
    (is_hash && !folder.is_empty()).then_some(folder)
}

/// Shuts the simulator down and waits for it. No error when it is not booted.
pub(crate) fn shut_down(udid: &str) {
    let _ = xcrun(&["simctl", "shutdown", udid], &|_| {});
}

/// Shuts the simulator down and deletes it with its data.
pub(crate) fn delete(udid: &str) -> Result<(), Error> {
    shut_down(udid);
    xcrun(&["simctl", "delete", udid], &|_| {}).map(drop)
}

fn device_named(listed: &Value, name: &str) -> Option<Simulator> {
    device_where(listed, |device| device["name"] == name)
}

fn list_devices() -> Result<Value, Error> {
    let listed = xcrun(&["simctl", "list", "devices", "--json"], &|_| {})?;
    Ok(serde_json::from_str(&listed)?)
}

/// The worktree's simulator, if it exists.
pub(crate) fn find(worktree: &Path) -> Result<Option<Simulator>, Error> {
    Ok(device_named(&list_devices()?, &name_of(worktree)))
}

/// The worktree's simulator, created on the first run: `wanted` (model and runtime identifiers), else the newest iOS runtime and the model it lists that reads best. A simulator of another model or runtime is deleted and created again; it is Chord's own.
pub(crate) fn find_or_create(
    worktree: &Path,
    wanted: Option<(&str, &str)>,
    register: &dyn Fn(u32),
) -> Result<Simulator, Error> {
    if let Some(device) = find(worktree)? {
        if wanted.is_none_or(|(model, runtime)| device.model == model && device.runtime == runtime)
        {
            return Ok(device);
        }
        let _ = xcrun(&["simctl", "shutdown", &device.udid], register);
        xcrun(&["simctl", "delete", &device.udid], register)?;
    }
    let (device_type, runtime) = match wanted {
        Some((model, runtime)) => (model.to_string(), runtime.to_string()),
        None => {
            let runtimes = xcrun(&["simctl", "list", "runtimes", "--json"], register)?;
            newest_ios_model(&serde_json::from_str(&runtimes)?).ok_or_else(|| {
                Error::Message(
                    "No iOS simulator runtime is installed. Add one in Xcode > Settings > Components."
                        .into(),
                )
            })?
        }
    };
    let udid = xcrun(
        &[
            "simctl",
            "create",
            &name_of(worktree),
            &device_type,
            &runtime,
        ],
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
    runtime["version"]
        .as_str()
        .unwrap_or_default()
        .split('.')
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// The newest available iOS runtime and its iPhone model: a Pro that is not a Max, else the last iPhone it lists. Returns the model's and the runtime's identifiers.
fn newest_ios_model(listed: &Value) -> Option<(String, String)> {
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
    Some((
        model["identifier"].as_str()?.into(),
        runtime["identifier"].as_str()?.into(),
    ))
}

/// Boots the simulator and waits until it reports Booted. A device that does not fails with its state.
pub(crate) fn boot(udid: &str) -> Result<(), Error> {
    xcrun(&["simctl", "bootstatus", udid, "-b"], &|_| {})?;
    match device_where(&list_devices()?, |device| device["udid"] == udid) {
        Some(device) if device.booted => Ok(()),
        _ => Err(Error::Message(
            "The simulator does not report Booted after its boot.".into(),
        )),
    }
}

/// Opens the selected Xcode's simulator app: DeviceHub from Xcode 27, Simulator.app before. Best effort.
pub(crate) fn open_app() {
    let Ok(developer) = xcrun(&["--find", "xcodebuild"], &|_| {}) else {
        return;
    };
    // `<Xcode>/Contents/Developer/usr/bin/xcodebuild`
    let Some(developer) = Path::new(developer.trim()).ancestors().nth(3) else {
        return;
    };
    let candidates = [
        developer.join("../Applications/DeviceHub.app"),
        developer.join("Applications/Simulator.app"),
    ];
    if let Some(app) = candidates.iter().find(|app| app.exists()) {
        let _ = Command::new(OPEN).arg("-g").arg(app).status();
    }
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

pub(crate) fn install(udid: &str, app: &Path, register: &dyn Fn(u32)) -> Result<(), Error> {
    xcrun(
        &["simctl", "install", udid, &app.to_string_lossy()],
        register,
    )
    .map(drop)
}

/// Ends the app; no error when it does not run.
pub(crate) fn terminate(udid: &str, bundle_id: &str) {
    let _ = xcrun(&["simctl", "terminate", udid, bundle_id], &|_| {});
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
    let stream = login_shell(
        "xcrun",
        &[
            "simctl",
            "spawn",
            udid,
            "log",
            "stream",
            "--style",
            "ndjson",
            "--predicate",
            &predicate,
        ],
    );
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
    // --console-pty gives the app a terminal, so its print output is line-buffered, and simctl exits with the app's exit code.
    let command = login_shell(
        "xcrun",
        &[
            "simctl",
            "launch",
            "--console-pty",
            "--terminate-running-process",
            udid,
            bundle_id,
        ],
    );
    let (mut child, reader) = match process::merged(command, Some(stream.id())) {
        Ok(started) => started,
        Err(error) => {
            let ended = stream.try_wait();
            end_stream(&mut stream);
            return Err(match ended {
                Ok(Some(status)) => Error::Message(format!(
                    "The OSLog stream ended before the app launched: {status}"
                )),
                _ => error.into(),
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
        assert_eq!(
            name_of(Path::new("/tmp/Example")),
            "Chord Example euhuvy"
        );
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
              {"platform":"iOS","isAvailable":true,"version":"27.1","identifier":"ios-27-1","supportedDeviceTypes":[
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
            Some(("p18pro".into(), "ios-27-1".into()))
        );
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
