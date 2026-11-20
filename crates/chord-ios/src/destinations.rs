//! The run destinations the menu lists: the connected devices, and the simulator models of each iOS version among the user's simulators.

use std::fs;
use std::sync::atomic::{AtomicU32, Ordering};

use chord_core::{Error, RunDestination};
use serde::Serialize;
use serde_json::Value;
use specta::Type;

use crate::process::xcrun;
use crate::simulator::{self, DeviceSet, simctl};

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct Destinations {
    pub devices: Vec<RunDestination>,
    /// Newest iOS version first.
    pub simulators: Vec<SimulatorModels>,
}

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct SimulatorModels {
    /// "iOS 27.0".
    pub os: String,
    /// iPhones before the rest, each by name.
    pub models: Vec<RunDestination>,
}

pub fn list() -> Result<Destinations, Error> {
    let simulators = simctl(
        DeviceSet::Run,
        &["list", "devicetypes", "devices", "available", "--json"],
        &|_| {},
    )?;
    Ok(Destinations {
        // Older Xcode has no devicectl: no devices.
        devices: connected_devices().unwrap_or_default(),
        simulators: simulators_in(&serde_json::from_str(&simulators)?),
    })
}

/// `devicectl` writes its JSON to a file only; its table goes to stdout.
fn connected_devices() -> Result<Vec<RunDestination>, Error> {
    // Listings run at once (launch and the open menu): each reads a file of its own.
    static LISTINGS: AtomicU32 = AtomicU32::new(0);
    let file = std::env::temp_dir().join(format!(
        "chord-devices-{}-{}.json",
        std::process::id(),
        LISTINGS.fetch_add(1, Ordering::Relaxed)
    ));
    let result = xcrun(
        &[
            "devicectl",
            "list",
            "devices",
            "--json-output",
            &file.to_string_lossy(),
        ],
        &|_| {},
    )
    .and_then(|_| Ok(fs::read_to_string(&file)?));
    let _ = fs::remove_file(&file);
    Ok(devices_in(&serde_json::from_str(&result?)?))
}

/// The physical devices that are not unavailable, by name. A wired device shows as disconnected until a tool uses it.
fn devices_in(listed: &Value) -> Vec<RunDestination> {
    let mut devices: Vec<RunDestination> = listed["result"]["devices"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|device| {
            device["hardwareProperties"]["reality"] == "physical"
                && device["connectionProperties"]["tunnelState"] != "unavailable"
        })
        .filter_map(|device| {
            Some(RunDestination::Device {
                id: device["hardwareProperties"]["udid"].as_str()?.into(),
                name: device["deviceProperties"]["name"].as_str()?.into(),
            })
        })
        .collect();
    devices.sort_by_key(|device| device.name().to_string());
    devices
}

/// `com.apple.CoreSimulator.SimRuntime.iOS-26-4` as ("iOS 26.4", [26, 4]); None for another platform.
fn ios_version(runtime: &str) -> Option<(String, Vec<u32>)> {
    let version = runtime.rsplit('.').next()?.strip_prefix("iOS-")?;
    let numbers = version
        .split('-')
        .map(|part| part.parse().ok())
        .collect::<Option<Vec<u32>>>()?;
    Some((format!("iOS {}", version.replace('-', ".")), numbers))
}

/// The models of the available simulators, once per iOS version, each under its device type's name; Chord's own per-worktree simulators do not count.
fn simulators_in(listed: &Value) -> Vec<SimulatorModels> {
    let type_name = |model: &str| {
        listed["devicetypes"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|kind| kind["identifier"] == model)
            .and_then(|kind| kind["name"].as_str())
    };
    let mut found: Vec<(Vec<u32>, SimulatorModels)> = Vec::new();
    for (runtime, devices) in listed["devices"].as_object().into_iter().flatten() {
        let Some((os, version)) = ios_version(runtime) else {
            continue;
        };
        let mut models: Vec<RunDestination> = Vec::new();
        for device in devices.as_array().into_iter().flatten() {
            let (Some(name), Some(model)) = (
                device["name"].as_str(),
                device["deviceTypeIdentifier"].as_str(),
            ) else {
                continue;
            };
            let destination = RunDestination::Simulator {
                model: model.into(),
                runtime: runtime.clone(),
                // Another tool's simulator ("Other App 8c5ca1") would list its model twice under two names.
                name: type_name(model).unwrap_or(name).into(),
                os: os.clone(),
            };
            if !simulator::is_chord_name(name) && !models.contains(&destination) {
                models.push(destination);
            }
        }
        if !models.is_empty() {
            models.sort_by_key(|model| {
                let name = model.name();
                (!name.starts_with("iPhone"), name.to_string())
            });
            found.push((version, SimulatorModels { os, models }));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, models)| models).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_models_by_ios_version_newest_first_iphones_first_without_chords_own() {
        let listed: Value = serde_json::from_str(
            r#"{"devicetypes":[{"identifier":"p18","name":"iPhone 18"}],"devices":{
              "com.apple.CoreSimulator.SimRuntime.iOS-26-4":[
                {"name":"iPad Pro","deviceTypeIdentifier":"ipad","isAvailable":true},
                {"name":"iPhone 17","deviceTypeIdentifier":"p17","isAvailable":true},
                {"name":"iPhone 17","deviceTypeIdentifier":"p17","isAvailable":true}],
              "com.apple.CoreSimulator.SimRuntime.watchOS-12-0":[
                {"name":"Watch","deviceTypeIdentifier":"w","isAvailable":true}],
              "com.apple.CoreSimulator.SimRuntime.iOS-27-0":[
                {"name":"Chord feature abcdef","deviceTypeIdentifier":"p18","isAvailable":true},
                {"name":"Other tool abcdef","deviceTypeIdentifier":"p18","isAvailable":true},
                {"name":"iPhone 18","deviceTypeIdentifier":"p18","isAvailable":true}]}}"#,
        )
        .unwrap();
        let names: Vec<(String, Vec<String>)> = simulators_in(&listed)
            .into_iter()
            .map(|group| {
                (
                    group.os,
                    group
                        .models
                        .iter()
                        .map(|model| model.name().to_string())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            names,
            vec![
                ("iOS 27.0".to_string(), vec!["iPhone 18".to_string()]),
                (
                    "iOS 26.4".to_string(),
                    vec!["iPhone 17".to_string(), "iPad Pro".to_string()]
                ),
            ]
        );
    }

    #[test]
    fn lists_physical_devices_that_are_not_unavailable() {
        let listed: Value = serde_json::from_str(
            r#"{"result":{"devices":[
              {"hardwareProperties":{"reality":"simulated","udid":"S"},"connectionProperties":{"tunnelState":"disconnected"},"deviceProperties":{"name":"Sim"}},
              {"hardwareProperties":{"reality":"physical","udid":"B"},"connectionProperties":{"tunnelState":"unavailable"},"deviceProperties":{"name":"Away"}},
              {"hardwareProperties":{"reality":"physical","udid":"A"},"connectionProperties":{"tunnelState":"disconnected"},"deviceProperties":{"name":"Phone"}}]}}"#,
        )
        .unwrap();
        assert_eq!(
            devices_in(&listed),
            vec![RunDestination::Device {
                id: "A".into(),
                name: "Phone".into()
            }]
        );
    }
}
