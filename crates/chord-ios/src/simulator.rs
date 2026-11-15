//! The worktree's simulator: found or created by name, booted, the app installed and launched on it with its output and OSLog read, and the Simulator app opened.

use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;
use chord_core::{Error, login_shell};
use specta::Type;

use crate::derived_data;
use crate::process::{self, BATCH_BYTES, BATCH_INTERVAL, end_group};

pub(crate) const PLUTIL: &str = "/usr/bin/plutil";
pub(crate) const OPEN: &str = "/usr/bin/open";
/// How long the launch waits for the log stream to start; it takes about a second.
const STREAM_START: Duration = Duration::from_secs(3);

#[derive(Serialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleTone {
    /// The app's stdout and stderr, and OSLog messages below error.
    Output,
    /// An OSLog error or fault.
    Error,
    /// A line Chord writes, not the app.
    Own,
}

#[derive(Debug, PartialEq)]
pub(crate) struct Device {
    pub udid: String,
    pub booted: bool,
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

/// Runs `xcrun` with `args` in a process group of its own, whose id goes to `register` so a stop can end it; its output when it succeeds, else its error text.
fn xcrun(args: &[&str], register: &dyn Fn(u32)) -> Result<String, Error> {
    let output = process::captured(login_shell("xcrun", args), register)?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(Error::Message(if error.is_empty() {
        format!("xcrun {} failed", args.join(" "))
    } else {
        error
    }))
}

fn device_named(listed: &Value, name: &str) -> Option<Device> {
    listed["devices"]
        .as_object()?
        .values()
        .filter_map(Value::as_array)
        .flatten()
        .find(|device| device["name"] == name && device["isAvailable"] == true)
        .map(|device| Device {
            udid: device["udid"].as_str().unwrap_or_default().into(),
            booted: device["state"] == "Booted",
        })
}

/// The worktree's simulator, if it exists.
pub(crate) fn find(worktree: &Path) -> Result<Option<Device>, Error> {
    let listed = xcrun(&["simctl", "list", "devices", "--json"], &|_| {})?;
    Ok(device_named(
        &serde_json::from_str(&listed)?,
        &name_of(worktree),
    ))
}

/// The worktree's simulator, created on the first run: the newest iOS runtime and the model it lists that reads best.
pub(crate) fn find_or_create(worktree: &Path, register: &dyn Fn(u32)) -> Result<Device, Error> {
    if let Some(device) = find(worktree)? {
        return Ok(device);
    }
    let runtimes = xcrun(&["simctl", "list", "runtimes", "--json"], register)?;
    let (device_type, runtime) =
        newest_ios_model(&serde_json::from_str(&runtimes)?).ok_or_else(|| {
            Error::Message(
                "No iOS simulator runtime is installed. Add one in Xcode > Settings > Components."
                    .into(),
            )
        })?;
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
    Ok(Device {
        udid: udid.trim().into(),
        booted: false,
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
pub(crate) fn boot(worktree: &Path, udid: &str) -> Result<(), Error> {
    xcrun(&["simctl", "bootstatus", udid, "-b"], &|_| {})?;
    match find(worktree)? {
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
    let plist = app.join("Info.plist").to_string_lossy().into_owned();
    let mut command = Command::new(PLUTIL);
    command.args(["-extract", key, "raw", "-o", "-", &plist]);
    let output = process::captured(command, register)?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !value.is_empty() {
        Ok(value)
    } else {
        Err(Error::Message(format!(
            "The app has no {key}: {}",
            app.display()
        )))
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

/// Console text waiting for its batch, in arrival order, neighbours of one tone joined.
#[derive(Default)]
struct Console {
    parts: Vec<(String, ConsoleTone)>,
    size: usize,
}

impl Console {
    fn push(&mut self, text: &str, tone: ConsoleTone) {
        self.size += text.len();
        match self.parts.last_mut() {
            Some((last, last_tone)) if *last_tone == tone => last.push_str(text),
            _ => self.parts.push((text.into(), tone)),
        }
    }

    fn flush(&mut self, console: &dyn Fn(String, ConsoleTone)) {
        self.size = 0;
        for (text, tone) in self.parts.drain(..) {
            console(text, tone);
        }
    }
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
    let mut output = String::new();
    let mut up = false;
    let mut pending = Console::default();
    let mut flushed = Instant::now();
    loop {
        while let Ok((text, tone)) = received_records.try_recv() {
            pending.push(&text, tone);
        }
        match lines.recv_timeout(BATCH_INTERVAL) {
            Ok(bytes) => {
                // The pty ends lines with \r\n.
                let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
                if up {
                    pending.push(&text, ConsoleTone::Output);
                } else if text.starts_with(&format!("{bundle_id}: ")) {
                    up = true;
                    launched();
                } else {
                    output.push_str(&text);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if pending.size >= BATCH_BYTES || flushed.elapsed() >= BATCH_INTERVAL {
            pending.flush(console);
            flushed = Instant::now();
        }
    }
    end_stream(&mut stream);
    while let Ok((text, tone)) = received_records.try_recv() {
        pending.push(&text, tone);
    }
    pending.flush(console);
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
                        {"name":"Chord w abcdef","udid":"U3","isAvailable":true,"state":"Booted"}]}}"#,
        )
        .unwrap();
        assert_eq!(
            device_named(&listed, "Chord w abcdef"),
            Some(Device {
                udid: "U3".into(),
                booted: true
            })
        );
        assert_eq!(device_named(&listed, "Missing"), None);
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
