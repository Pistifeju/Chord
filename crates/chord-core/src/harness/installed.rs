use std::process::{Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;
use specta::Type;

use super::{HARNESSES, Harness, login_shell};
use crate::{Error, lock};

const GIT_INSTALL: &str = "xcode-select --install";

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Install {
    Ready { version: String },
    Missing,
    SignedOut { version: String },
    Outdated { version: String, oldest: String },
}

impl Install {
    fn takes_pick(&self) -> bool {
        !matches!(self, Install::Missing)
    }
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HarnessCheck {
    pub name: String,
    pub title: String,
    pub starts: bool,
    pub install: String,
    pub sign_in: Option<String>,
    pub upgrade: String,
    pub install_state: Install,
    pub takes_pick: bool,
}

impl HarnessCheck {
    fn step(&self) -> Option<String> {
        let title = &self.title;
        match &self.install_state {
            Install::Ready { .. } => None,
            Install::Missing => Some(format!(
                "{title} is not installed. Install it with: {}",
                self.install
            )),
            Install::SignedOut { .. } => Some(format!(
                "{title} is not signed in. Sign in with: {}",
                self.sign_in.as_deref().unwrap_or_default()
            )),
            Install::Outdated { version, oldest } => Some(format!(
                "{title} {version} is older than the tested {oldest}. Upgrade it with: {}",
                self.upgrade
            )),
        }
    }
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Checks {
    pub harnesses: Vec<HarnessCheck>,
    pub git_install: Option<String>,
    pub default_harness: Option<String>,
}

pub fn checks(pick: Option<&str>) -> Checks {
    let (harnesses, git) = std::thread::scope(|scope| {
        let git = scope.spawn(git_runs);
        (check_each(HARNESSES.iter()), git.join().unwrap_or(false))
    });
    let default_harness = default_harness(pick, &harnesses).map(str::to_string);
    *lock(&CHECKED) = Some(harnesses.clone());
    Checks {
        harnesses,
        git_install: (!git).then(|| GIT_INSTALL.into()),
        default_harness,
    }
}

static CHECKED: Mutex<Option<Vec<HarnessCheck>>> = Mutex::new(None);

pub fn default_for_new_pane(pick: Option<&str>) -> Result<String, Error> {
    let checked = lock(&CHECKED);
    if let Some(name) = checked
        .as_deref()
        .and_then(|checks| default_harness(pick, checks))
    {
        return Ok(name.to_string());
    }
    drop(checked);
    let starting: Vec<&Harness> = HARNESSES.iter().filter(|harness| harness.starts).collect();
    let checks = check_each(starting.iter().copied());
    if let Some(name) = default_harness(pick, &checks) {
        let name = name.to_string();
        *lock(&CHECKED) = Some(checks);
        return Ok(name);
    }
    match (starting.as_slice(), checks.as_slice()) {
        (
            [harness],
            [
                HarnessCheck {
                    install_state: Install::Missing,
                    ..
                },
            ],
        ) => Err(Error::MissingTool {
            tool: harness.binary.into(),
            install: harness.install.into(),
        }),
        _ => Err(Error::Message(
            checks
                .iter()
                .filter_map(HarnessCheck::step)
                .collect::<Vec<_>>()
                .join("\n"),
        )),
    }
}

pub fn default_harness<'a>(pick: Option<&str>, checks: &'a [HarnessCheck]) -> Option<&'a str> {
    let starting = || checks.iter().filter(|check| check.starts);
    starting()
        .find(|check| Some(check.name.as_str()) == pick && check.takes_pick)
        .or_else(|| {
            starting().find(|check| {
                matches!(
                    check.install_state,
                    Install::Ready { .. } | Install::Outdated { .. }
                )
            })
        })
        .map(|check| check.name.as_str())
}

fn check_each<'a>(harnesses: impl Iterator<Item = &'a Harness>) -> Vec<HarnessCheck> {
    std::thread::scope(|scope| {
        let running: Vec<_> = harnesses
            .map(|harness| (harness, scope.spawn(|| install_state(harness))))
            .collect();
        running
            .into_iter()
            .map(|(harness, state)| {
                let install_state = state.join().unwrap_or(Install::Missing);
                HarnessCheck {
                    name: harness.name.into(),
                    title: harness.title.into(),
                    starts: harness.starts,
                    install: harness.install.into(),
                    sign_in: harness.sign_in.as_ref().map(|sign_in| sign_in.step.into()),
                    upgrade: harness.upgrade.into(),
                    takes_pick: install_state.takes_pick(),
                    install_state,
                }
            })
            .collect()
    })
}

fn install_state(harness: &Harness) -> Install {
    let Ok(found) = version(harness) else {
        return Install::Missing;
    };
    let shown = found.map_or_else(|| "unknown".into(), dotted);
    if found.is_some_and(|found| found < harness.oldest_tested) {
        return Install::Outdated {
            version: shown,
            oldest: dotted(harness.oldest_tested),
        };
    }
    let signed_in = harness.sign_in.as_ref().is_none_or(|sign_in| {
        quiet(&mut login_shell(harness.binary, sign_in.check))
            .status()
            .is_ok_and(|status| status.success())
    });
    if signed_in {
        Install::Ready { version: shown }
    } else {
        Install::SignedOut { version: shown }
    }
}

static PASSED: Mutex<Vec<&str>> = Mutex::new(Vec::new());

pub fn require(harness: &Harness) -> Result<Option<String>, Error> {
    if lock(&PASSED).contains(&harness.name) {
        return Ok(None);
    }
    match version(harness)? {
        Some(found) if found < harness.oldest_tested => Ok(Some(format!(
            "{} {} is older than the tested {}. Upgrade it with: {}",
            harness.title,
            dotted(found),
            dotted(harness.oldest_tested),
            harness.upgrade
        ))),
        _ => {
            lock(&PASSED).push(harness.name);
            Ok(None)
        }
    }
}

fn version(harness: &Harness) -> Result<Option<(u32, u32, u32)>, Error> {
    let output = quiet(&mut login_shell(harness.binary, &["--version"]))
        .stdout(Stdio::piped())
        .output()
        .map_err(|error| Error::start_failed(harness.binary, harness.install, error))?;
    if output.status.code() == Some(127) {
        return Err(Error::MissingTool {
            tool: harness.binary.into(),
            install: harness.install.into(),
        });
    }
    Ok(parse_version(&String::from_utf8_lossy(&output.stdout)))
}

fn git_runs() -> bool {
    quiet(Command::new("git").arg("--version"))
        .status()
        .is_ok_and(|status| status.success())
}

fn quiet(command: &mut Command) -> &mut Command {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
}

fn dotted((major, minor, patch): (u32, u32, u32)) -> String {
    format!("{major}.{minor}.{patch}")
}

fn parse_version(text: &str) -> Option<(u32, u32, u32)> {
    text.lines()
        .rev()
        .flat_map(str::split_whitespace)
        .find_map(|word| {
            let mut parts = word.strip_prefix('v').unwrap_or(word).split('.');
            let mut number = || {
                let digits: String = parts
                    .next()?
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                digits.parse().ok()
            };
            let version = (number()?, number()?, number()?);
            Some(version)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_harness_version_line_parses() {
        assert_eq!(parse_version("1.5.2\n"), Some((1, 5, 2)));
        assert_eq!(parse_version("codex-cli 0.158.0"), Some((0, 158, 0)));
        assert_eq!(parse_version("2.1.328 (Claude Code)"), Some((2, 1, 328)));
        assert_eq!(parse_version("opencode v2.4.1"), Some((2, 4, 1)));
        assert_eq!(parse_version("Shell 5.9.0 ready\n1.5.2\n"), Some((1, 5, 2)));
        assert_eq!(parse_version("welcome back"), None);
    }

    fn harness(name: &str, starts: bool, install_state: Install) -> HarnessCheck {
        HarnessCheck {
            name: name.into(),
            title: name.into(),
            starts,
            install: String::new(),
            sign_in: None,
            upgrade: String::new(),
            takes_pick: install_state.takes_pick(),
            install_state,
        }
    }

    #[test]
    fn the_default_is_the_pick_while_it_can_start_else_the_first_ready_one() {
        let ready = || Install::Ready {
            version: "1.0.0".into(),
        };
        let signed_out = Install::SignedOut {
            version: "1.0.0".into(),
        };
        let checks = [
            harness("pi", true, Install::Missing),
            harness("codex", false, ready()),
            harness("claude", true, signed_out),
            harness("other", true, ready()),
        ];
        assert_eq!(default_harness(Some("claude"), &checks), Some("claude"));
        assert_eq!(default_harness(Some("pi"), &checks), Some("other"));
        assert_eq!(default_harness(None, &checks[..3]), None);
        let outdated = Install::Outdated {
            version: "1.0.0".into(),
            oldest: "2.0.0".into(),
        };
        let checks = [
            harness("pi", true, outdated),
            harness("other", true, ready()),
        ];
        assert_eq!(default_harness(None, &checks), Some("pi"));
    }
}
