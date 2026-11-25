use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use chord_core::Error;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;

use crate::simulator::{self, DeviceSet};

const LOCALE: &str = "/usr/bin/locale";

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceStyle {
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ContentSize {
    ExtraSmall,
    Small,
    Medium,
    Large,
    ExtraLarge,
    ExtraExtraLarge,
    ExtraExtraExtraLarge,
    AccessibilityMedium,
    AccessibilityLarge,
    AccessibilityExtraLarge,
    AccessibilityExtraExtraLarge,
    AccessibilityExtraExtraExtraLarge,
}

const CONTENT_SIZES: [ContentSize; 12] = [
    ContentSize::ExtraSmall,
    ContentSize::Small,
    ContentSize::Medium,
    ContentSize::Large,
    ContentSize::ExtraLarge,
    ContentSize::ExtraExtraLarge,
    ContentSize::ExtraExtraExtraLarge,
    ContentSize::AccessibilityMedium,
    ContentSize::AccessibilityLarge,
    ContentSize::AccessibilityExtraLarge,
    ContentSize::AccessibilityExtraExtraLarge,
    ContentSize::AccessibilityExtraExtraExtraLarge,
];

#[derive(Serialize, Type, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimulatorAppearance {
    pub style: Option<InterfaceStyle>,
    pub content_size: Option<ContentSize>,
    pub language: Option<String>,
    pub region: Option<String>,
    pub content_sizes: Vec<ContentSize>,
    pub languages: Vec<String>,
    pub regions: Vec<String>,
}

#[derive(Deserialize, Type, Clone, Debug, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SimulatorAppearanceChange {
    Style { style: InterfaceStyle },
    ContentSize { size: ContentSize },
    Language { language: String },
    Region { region: String },
}

fn word<T: Serialize>(setting: T) -> String {
    match serde_json::to_value(setting) {
        Ok(Value::String(word)) => word,
        _ => unreachable!("a setting serializes to a string"),
    }
}

fn setting<T: DeserializeOwned>(word: &str) -> Option<T> {
    serde_json::from_value(Value::String(word.into())).ok()
}

fn locale_codes(listing: &str) -> (Vec<String>, Vec<String>) {
    let mut languages = BTreeSet::new();
    let mut regions = BTreeSet::new();
    for line in listing.lines() {
        let name = line.split(['.', '@']).next().unwrap_or("");
        let Some((language, region)) = name.split_once('_') else {
            continue;
        };
        if is_language(language) && is_region(region) {
            languages.insert(match (language, region) {
                ("zh", "TW" | "HK" | "MO") => "zh-Hant".to_string(),
                ("zh", _) => "zh-Hans".to_string(),
                _ => language.to_string(),
            });
            regions.insert(region.to_string());
        }
    }
    (
        languages.into_iter().collect(),
        regions.into_iter().collect(),
    )
}

fn is_language(code: &str) -> bool {
    let (base, script) = code.split_once('-').unwrap_or((code, ""));
    (2..=3).contains(&base.len())
        && base.bytes().all(|byte| byte.is_ascii_lowercase())
        && (script.is_empty() || is_script(script))
}

fn is_script(code: &str) -> bool {
    code.len() == 4
        && code.as_bytes()[0].is_ascii_uppercase()
        && code.bytes().skip(1).all(|byte| byte.is_ascii_lowercase())
}

fn is_region(code: &str) -> bool {
    code.len() == 2 && code.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn language_of(identifier: &str) -> Option<String> {
    let mut parts = identifier.split(['-', '_', '@', '.']);
    let base = parts.next()?;
    let language = match parts.next() {
        Some(script) if is_script(script) => format!("{base}-{script}"),
        _ => base.to_string(),
    };
    is_language(&language).then_some(language)
}

fn first_language(text: &str) -> Option<String> {
    let first = text
        .trim_start_matches(|c: char| c == '(' || c == '"' || c.is_whitespace())
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .next()?;
    language_of(first)
}

fn region_of_locale(text: &str) -> Option<String> {
    let text = text.trim();
    let region = match text.split_once("@rg=") {
        Some((_, value)) => value.get(..2)?.to_ascii_uppercase(),
        None => text
            .split(['@', '.'])
            .next()?
            .split_once('_')?
            .1
            .to_string(),
    };
    is_region(&region).then_some(region)
}

struct Device {
    set: DeviceSet,
    udid: String,
}

impl Device {
    fn booted(worktree: &Path, preview: bool) -> Result<Device, Error> {
        let (set, udid) = simulator::booted(worktree, preview)?;
        Ok(Device { set, udid })
    }

    fn simctl(&self, args: &[&str]) -> Result<String, Error> {
        simulator::simctl(self.set, args, &|_| {})
    }

    fn ui<T: DeserializeOwned>(&self, name: &str) -> Option<T> {
        let value = self.simctl(&["ui", &self.udid, name]).ok()?;
        setting(value.trim())
    }

    fn set_ui(&self, name: &str, value: String) -> Result<(), Error> {
        self.simctl(&["ui", &self.udid, name, &value]).map(drop)
    }

    fn preference(&self, key: &str) -> Option<String> {
        self.simctl(&["spawn", &self.udid, "defaults", "read", "-g", key])
            .ok()
    }

    fn write_preference(&self, args: &[&str]) -> Result<(), Error> {
        let command = [&["spawn", &self.udid, "defaults", "write", "-g"], args].concat();
        self.simctl(&command).map(drop)
    }

    fn language(&self) -> Option<String> {
        self.preference("AppleLanguages")
            .and_then(|text| first_language(&text))
            .or_else(|| {
                self.preference("AppleLocale")
                    .and_then(|text| language_of(text.trim()))
            })
    }

    fn region(&self) -> Option<String> {
        self.preference("AppleLocale")
            .and_then(|text| region_of_locale(&text))
    }
}

fn locale_listing() -> Result<String, Error> {
    let output = Command::new(LOCALE)
        .arg("-a")
        .output()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => Error::MissingTool {
                tool: "locale".into(),
                install: "it ships with macOS in /usr/bin; restore it by reinstalling macOS".into(),
            },
            _ => Error::Message(format!("`locale -a` did not run: {error}")),
        })?;
    if !output.status.success() {
        return Err(Error::Message(format!(
            "`locale -a` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn read(worktree: &Path, preview: bool) -> Result<SimulatorAppearance, Error> {
    let device = Device::booted(worktree, preview)?;
    let (languages, regions) = locale_codes(&locale_listing()?);
    Ok(SimulatorAppearance {
        style: device.ui("appearance"),
        content_size: device.ui("content_size"),
        language: device.language(),
        region: device.region(),
        content_sizes: CONTENT_SIZES.to_vec(),
        languages,
        regions,
    })
}

pub fn set(
    worktree: &Path,
    preview: bool,
    change: &SimulatorAppearanceChange,
) -> Result<(), Error> {
    let device = Device::booted(worktree, preview)?;
    match change {
        SimulatorAppearanceChange::Style { style } => device.set_ui("appearance", word(style)),
        SimulatorAppearanceChange::ContentSize { size } => {
            device.set_ui("content_size", word(size))
        }
        SimulatorAppearanceChange::Language { language } if is_language(language) => {
            device.write_preference(&["AppleLanguages", "-array", language])?;
            match device.region() {
                Some(region) => {
                    device.write_preference(&["AppleLocale", &format!("{language}_{region}")])
                }
                None => Ok(()),
            }
        }
        SimulatorAppearanceChange::Region { region } if is_region(region) => {
            let language = device.language().ok_or_else(|| {
                Error::Message("The simulator's language is unknown. Pick a language first.".into())
            })?;
            device.write_preference(&["AppleLocale", &format!("{language}_{region}")])
        }
        _ => Err(Error::Message(
            "That appearance setting is not valid.".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_codes_split_languages_and_regions() {
        let listing = "C\nPOSIX\nde_AT.UTF-8\nde_DE.ISO8859-1\nen_US.UTF-8\nsr_RS.UTF-8@latin\nzh_CN.GBK\nzh_TW\nUTF-8\n";
        let (languages, regions) = locale_codes(listing);
        assert_eq!(languages, ["de", "en", "sr", "zh-Hans", "zh-Hant"]);
        assert_eq!(regions, ["AT", "CN", "DE", "RS", "TW", "US"]);
    }

    #[test]
    fn first_language_reads_the_defaults_array() {
        assert_eq!(
            first_language("(\n    \"de-AT\",\n    \"en\"\n)\n"),
            Some("de".into())
        );
        assert_eq!(first_language("(\n    en\n)"), Some("en".into()));
        assert_eq!(
            first_language("(\n    \"zh-Hans-CN\"\n)"),
            Some("zh-Hans".into())
        );
        assert_eq!(first_language(""), None);
    }

    #[test]
    fn region_of_locale_reads_the_region_after_the_language() {
        assert_eq!(region_of_locale("de_AT\n"), Some("AT".into()));
        assert_eq!(region_of_locale("en_US@rg=huzzzz\n"), Some("HU".into()));
        assert_eq!(region_of_locale("zh-Hans_CN"), Some("CN".into()));
        assert_eq!(region_of_locale("en"), None);
    }
}
