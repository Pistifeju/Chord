//! The worktree's run simulator's appearance: light or dark and the Dynamic Type size through `simctl ui`, which act at once, and the language and region in the simulator's global preferences, which an app reads when it next launches. The languages and regions offered are the ones the Mac's own locales list.

use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use chord_core::Error;
use specta::Type;

use crate::simulator::{self, DeviceSet, Simulator};
use crate::xcode;

const LOCALE: &str = "/usr/bin/locale";

#[derive(Serialize, Deserialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceStyle {
    Light,
    Dark,
}

/// `simctl ui content_size`'s categories.
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

/// Smallest first.
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

/// What the Appearance menu shows: the simulator's current settings, and what it can be set to.
#[derive(Serialize, Type, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimulatorAppearance {
    pub style: Option<InterfaceStyle>,
    pub content_size: Option<ContentSize>,
    /// A language code like "de" or "zh-Hans".
    pub language: Option<String>,
    /// A region code like "AT".
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

/// The `simctl ui` word for a setting, which is its serde name.
fn word<T: Serialize>(setting: T) -> String {
    match serde_json::to_value(setting) {
        Ok(Value::String(word)) => word,
        _ => unreachable!("a setting serializes to a string"),
    }
}

fn setting<T: DeserializeOwned>(word: &str) -> Option<T> {
    serde_json::from_value(Value::String(word.into())).ok()
}

/// The language codes and the region codes of `locale -a`'s lines ("de_AT.UTF-8" is "de" and "AT"), sorted. Chinese takes the script iOS names it by: "zh_TW" is "zh-Hant".
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

/// "de", or a language with its script: "zh-Hans".
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

/// The language of a locale identifier, with its script if it has one: "de-AT" and "de_AT" are "de", "zh-Hans-CN" is "zh-Hans".
fn language_of(identifier: &str) -> Option<String> {
    let mut parts = identifier.split(['-', '_', '@', '.']);
    let base = parts.next()?;
    let language = match parts.next() {
        Some(script) if is_script(script) => format!("{base}-{script}"),
        _ => base.to_string(),
    };
    is_language(&language).then_some(language)
}

/// The first entry of `defaults read -g AppleLanguages` ("(\n    \"de-AT\"\n)").
fn first_language(text: &str) -> Option<String> {
    let first = text
        .trim_start_matches(|c: char| c == '(' || c == '"' || c.is_whitespace())
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .next()?;
    language_of(first)
}

/// The region of an `AppleLocale` like "de_AT"; a region override wins: "en_US@rg=huzzzz" is "HU".
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

fn booted(worktree: &Path) -> Result<Simulator, Error> {
    xcode::require_tools()?;
    simulator::find(DeviceSet::Run, worktree)?
        .filter(|device| device.booted)
        .ok_or_else(|| Error::Message("The simulator is off. Run boots it.".into()))
}

fn ui<T: DeserializeOwned>(udid: &str, name: &str) -> Option<T> {
    let value = simulator::simctl(DeviceSet::Run, &["ui", udid, name], &|_| {}).ok()?;
    setting(value.trim())
}

fn preference(udid: &str, key: &str) -> Option<String> {
    simulator::simctl(
        DeviceSet::Run,
        &["spawn", udid, "defaults", "read", "-g", key],
        &|_| {},
    )
    .ok()
}

fn write_preference(udid: &str, args: &[&str]) -> Result<(), Error> {
    let command = [&["spawn", udid, "defaults", "write", "-g"], args].concat();
    simulator::simctl(DeviceSet::Run, &command, &|_| {}).map(drop)
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

/// The simulator's first language; a device without `AppleLanguages` takes its `AppleLocale`'s.
fn current_language(udid: &str) -> Option<String> {
    preference(udid, "AppleLanguages")
        .and_then(|text| first_language(&text))
        .or_else(|| preference(udid, "AppleLocale").and_then(|text| language_of(text.trim())))
}

fn current_region(udid: &str) -> Option<String> {
    preference(udid, "AppleLocale").and_then(|text| region_of_locale(&text))
}

/// The worktree's booted run simulator's settings, with the languages and regions the Mac offers.
pub fn read(worktree: &Path) -> Result<SimulatorAppearance, Error> {
    let device = booted(worktree)?;
    let (languages, regions) = locale_codes(&locale_listing()?);
    Ok(SimulatorAppearance {
        style: ui(&device.udid, "appearance"),
        content_size: ui(&device.udid, "content_size"),
        language: current_language(&device.udid),
        region: current_region(&device.udid),
        content_sizes: CONTENT_SIZES.to_vec(),
        languages,
        regions,
    })
}

/// Applies `change` to the worktree's booted run simulator. A language or region reaches an app the next time it launches.
pub fn set(worktree: &Path, change: &SimulatorAppearanceChange) -> Result<(), Error> {
    let device = booted(worktree)?;
    let udid = device.udid.as_str();
    let ui = |name: &str, value: String| {
        simulator::simctl(DeviceSet::Run, &["ui", udid, name, &value], &|_| {}).map(drop)
    };
    match change {
        SimulatorAppearanceChange::Style { style } => ui("appearance", word(style)),
        SimulatorAppearanceChange::ContentSize { size } => ui("content_size", word(size)),
        // The locale carries the language too, so it follows, as iOS's Settings does it.
        SimulatorAppearanceChange::Language { language } if is_language(language) => {
            write_preference(udid, &["AppleLanguages", "-array", language])?;
            match current_region(udid) {
                Some(region) => {
                    write_preference(udid, &["AppleLocale", &format!("{language}_{region}")])
                }
                None => Ok(()),
            }
        }
        SimulatorAppearanceChange::Region { region } if is_region(region) => {
            let language = current_language(udid).ok_or_else(|| {
                Error::Message("The simulator's language is unknown. Pick a language first.".into())
            })?;
            write_preference(udid, &["AppleLocale", &format!("{language}_{region}")])
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
