use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chord_core::Error;

use crate::plist::PLUTIL;
use crate::process;
use crate::service_groups::{GROUPS, ServiceGroup, ServiceRow, Subgroup, SubgroupRow};
use crate::simulator::{DeviceSet, simctl};

const FIRST_SLIM_RUNTIME: (u32, u32) = (18, 5);

fn kept(group: &ServiceGroup, subgroup: &Subgroup, switches: &BTreeMap<String, bool>) -> bool {
    switches
        .get(subgroup.id)
        .or_else(|| switches.get(group.id))
        .copied()
        .unwrap_or(group.kept_by_default)
}

pub fn service_rows(switches: &BTreeMap<String, bool>) -> Vec<ServiceRow> {
    GROUPS
        .iter()
        .map(|group| {
            let subgroups: Vec<SubgroupRow> = group
                .subgroups
                .iter()
                .map(|subgroup| SubgroupRow {
                    id: subgroup.id,
                    name: subgroup.name,
                    effect: subgroup.effect,
                    memory_mb: subgroup.memory_mb,
                    kept: kept(group, subgroup, switches),
                })
                .collect();
            let all = subgroups.iter().all(|row| row.kept);
            let none = subgroups.iter().all(|row| !row.kept);
            ServiceRow {
                id: group.id,
                name: group.name,
                effect: group.effect,
                memory_mb: subgroups.iter().map(|row| row.memory_mb).sum(),
                kept: (all || none).then_some(all),
                subgroups: if subgroups.len() > 1 {
                    subgroups
                } else {
                    Vec::new()
                },
            }
        })
        .collect()
}

fn subgroups_kept(
    switches: &BTreeMap<String, bool>,
) -> impl Iterator<Item = (&'static Subgroup, bool)> + '_ {
    GROUPS.iter().flat_map(move |group| {
        group
            .subgroups
            .iter()
            .map(move |subgroup| (subgroup, kept(group, subgroup, switches)))
    })
}

fn labels_to_disable(switches: &BTreeMap<String, bool>) -> BTreeSet<&'static str> {
    let needed: BTreeSet<&str> = subgroups_kept(switches)
        .filter(|(_, kept)| *kept)
        .flat_map(|(subgroup, _)| subgroup.labels.iter().copied())
        .collect();
    subgroups_kept(switches)
        .filter(|(_, kept)| !*kept)
        .flat_map(|(subgroup, _)| subgroup.labels.iter().copied())
        .filter(|label| !needed.contains(label))
        .collect()
}

pub(crate) fn runtime_slims(runtime: &str) -> bool {
    let name = runtime.rsplit('.').next().unwrap_or_default();
    let mut parts = name.split('-');
    let ios = parts.next() == Some("iOS");
    let mut number = || parts.next().and_then(|part| part.parse::<u32>().ok());
    match (ios, number(), number()) {
        (true, Some(major), Some(minor)) => (major, minor) >= FIRST_SLIM_RUNTIME,
        _ => false,
    }
}

fn store_folder(udid: &str) -> PathBuf {
    PathBuf::from(format!(
        "/private/var/tmp/com.apple.CoreSimulator.SimDevice.{udid}"
    ))
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn plist_of(entries: &BTreeMap<String, bool>) -> String {
    let mut text = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict>",
    );
    for (label, disabled) in entries {
        let value = if *disabled { "true" } else { "false" };
        text.push_str(&format!("<key>{}</key><{value}/>", escaped(label)));
    }
    text.push_str("</dict></plist>\n");
    text
}

fn stored_entries(file: &Path) -> BTreeMap<String, bool> {
    let mut command = Command::new(PLUTIL);
    command.args(["-convert", "json", "-o", "-"]).arg(file);
    let Ok(output) = process::captured(command, &|_| {}) else {
        return BTreeMap::new();
    };
    serde_json::from_slice::<BTreeMap<String, serde_json::Value>>(&output.stdout)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(label, value)| Some((label, value.as_bool()?)))
        .collect()
}

fn disabled_labels(output: &str) -> BTreeSet<&str> {
    output
        .lines()
        .filter_map(|line| {
            let (label, state) = line.trim().split_once(" => ")?;
            (state == "disabled").then_some(label.trim_matches('"'))
        })
        .collect()
}

pub(crate) fn write_overrides(
    udid: &str,
    switches: &BTreeMap<String, bool>,
) -> Result<BTreeSet<&'static str>, Error> {
    let off = labels_to_disable(switches);
    let folder = store_folder(udid);
    let file = folder.join("disabled.plist");
    let mut entries = stored_entries(&file);
    for subgroup in GROUPS.iter().flat_map(|group| group.subgroups) {
        for label in subgroup.labels {
            if off.contains(label) {
                entries.insert((*label).into(), true);
            } else {
                entries.remove(*label);
            }
        }
    }
    fs::create_dir_all(&folder)?;
    let staged = folder.join("disabled.plist.chord");
    fs::write(&staged, plist_of(&entries))?;
    fs::rename(&staged, &file)?;
    Ok(off)
}

pub(crate) fn holds(udid: &str, off: &BTreeSet<&str>) -> Option<bool> {
    let Ok(listed) = simctl(
        DeviceSet::Run,
        &["spawn", udid, "launchctl", "print-disabled", "system"],
        &|_| {},
    ) else {
        return None;
    };
    let disabled = disabled_labels(&listed);
    Some(off.iter().all(|label| disabled.contains(label)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slims_from_ios_18_5_only() {
        let runtime = |name: &str| format!("com.apple.CoreSimulator.SimRuntime.{name}");
        assert!(runtime_slims(&runtime("iOS-18-5")));
        assert!(runtime_slims(&runtime("iOS-26-0")));
        assert!(!runtime_slims(&runtime("iOS-18-4")));
        assert!(!runtime_slims(&runtime("iOS-17-5")));
        assert!(!runtime_slims(&runtime("watchOS-26-4")));
    }

    #[test]
    fn keeps_app_store_and_safari_services_on_by_default() {
        let off = labels_to_disable(&BTreeMap::new());
        assert!(off.contains("com.apple.siriinferenced"));
        assert!(!off.contains("com.apple.storekitd"));
        assert!(!off.contains("com.apple.swcd"));
        assert!(!off.contains("com.apple.amsaccountsd"));
    }

    #[test]
    fn a_switch_turns_a_group_on_or_off() {
        let switches = BTreeMap::from([("siri".to_string(), true), ("store".to_string(), false)]);
        let off = labels_to_disable(&switches);
        assert!(!off.contains("com.apple.siriinferenced"));
        assert!(off.contains("com.apple.storekitd"));
    }

    #[test]
    fn a_kept_subgroup_keeps_its_shared_labels_while_the_rest_of_its_group_is_off() {
        let switches = BTreeMap::from([
            ("connectivity".to_string(), false),
            ("connectivity.watch".to_string(), true),
        ]);
        let off = labels_to_disable(&switches);
        assert!(!off.contains("com.apple.wcd"));
        assert!(off.contains("com.apple.rapportd"));
        let rows = service_rows(&switches);
        let row = rows.iter().find(|row| row.id == "connectivity").unwrap();
        assert_eq!(row.kept, None);
    }

    #[test]
    fn reads_the_disabled_labels_of_print_disabled() {
        let output = "disabled services = {\n\t\"com.apple.a\" => disabled\n\t\"com.apple.b\" => enabled\n}\n";
        assert_eq!(disabled_labels(output), BTreeSet::from(["com.apple.a"]));
    }

    #[test]
    fn writes_the_store_as_a_plist_of_booleans() {
        let entries = BTreeMap::from([("a&b".to_string(), true), ("c".to_string(), false)]);
        assert!(plist_of(&entries).contains("<key>a&amp;b</key><true/><key>c</key><false/>"));
    }
}
