//! The worktree's simulator: found or created by name, booted, the app installed and launched on it, and the Simulator app opened.

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use chord_core::{Error, login_shell};

use crate::derived_data;
use crate::process;

pub(crate) const PLUTIL: &str = "/usr/bin/plutil";
pub(crate) const OPEN: &str = "/usr/bin/open";

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

pub(crate) fn bundle_id(app: &Path, register: &dyn Fn(u32)) -> Result<String, Error> {
    let plist = app.join("Info.plist").to_string_lossy().into_owned();
    let mut command = Command::new(PLUTIL);
    command.args(["-extract", "CFBundleIdentifier", "raw", "-o", "-", &plist]);
    let output = process::captured(command, register)?;
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !id.is_empty() {
        Ok(id)
    } else {
        Err(Error::Message(format!(
            "The app has no bundle identifier: {}",
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

/// Launches the app, replacing a running copy, and returns when it ends. `launched` is called once simctl reports the app's process; its output after that is dropped. An app that never launches fails with simctl's message.
pub(crate) fn launch(
    udid: &str,
    bundle_id: &str,
    register: &dyn Fn(u32),
    launched: &dyn Fn(),
) -> Result<(), Error> {
    let command = login_shell(
        "xcrun",
        &[
            "simctl",
            "launch",
            "--console",
            "--terminate-running-process",
            udid,
            bundle_id,
        ],
    );
    let (mut child, reader) = process::merged(command)?;
    register(child.id());
    let mut output = String::new();
    let mut up = false;
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
        if !up {
            let text = String::from_utf8_lossy(&line);
            if text.starts_with(&format!("{bundle_id}: ")) {
                up = true;
                launched();
            } else {
                output.push_str(&text);
            }
        }
        line.clear();
    }
    let status = child.wait()?;
    if up {
        return Ok(());
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
}
