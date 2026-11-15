//! The worktree's Xcode project or workspace, its schemes, and the check that `xcodebuild` and `xcrun` exist.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::Value;
use chord_core::{Error, login_shell, xcode_bundles};

use crate::derived_data::LSOF;
use crate::process;
use crate::simulator::{OPEN, PLUTIL};

pub(crate) struct Project {
    pub path: PathBuf,
    /// `-workspace` or `-project`.
    pub flag: &'static str,
}

/// The tools a build needs, how each is checked and how to get it.
const TOOLS: [(&str, &str, &str); 2] = [
    (
        "xcodebuild",
        "-version",
        "Xcode from the App Store, then sudo xcode-select -s /Applications/Xcode.app/Contents/Developer",
    ),
    ("xcrun", "--version", "xcode-select --install"),
];

/// Fails with `MissingTool` and the install step when `xcodebuild`, `xcrun`, `lsof`, `plutil` or `open` does not run: the login shell finds nothing, or the command line tools' stand-in has no Xcode behind it.
pub(crate) fn require_tools() -> Result<(), Error> {
    for path in [LSOF, PLUTIL, OPEN] {
        if !Path::new(path).exists() {
            return Err(Error::MissingTool {
                tool: path.into(),
                install: "reinstall macOS: it is part of it".into(),
            });
        }
    }
    for (tool, check, install) in TOOLS {
        let runs = login_shell(tool, &[check])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !runs {
            return Err(Error::MissingTool {
                tool: tool.into(),
                install: install.into(),
            });
        }
    }
    Ok(())
}

/// The workspace in the worktree's folder, else its project.
pub(crate) fn project_in(worktree: &Path) -> Result<Project, Error> {
    let found = xcode_bundles(worktree);
    let of = |extension: &str| {
        found
            .iter()
            .find(|path| path.extension().is_some_and(|e| e == extension))
    };
    if let Some(path) = of("xcworkspace") {
        return Ok(Project {
            path: path.clone(),
            flag: "-workspace",
        });
    }
    match of("xcodeproj") {
        Some(path) => Ok(Project {
            path: path.clone(),
            flag: "-project",
        }),
        None => Err(Error::Message("No Xcode project in this worktree".into())),
    }
}

/// The scheme named like the project, else one named like a target, else the first. Each `xcodebuild -list` runs in its own process group, whose id goes to `register` so a stop can end it.
pub(crate) fn scheme_of(
    project: &Project,
    worktree: &Path,
    register: &dyn Fn(u32),
) -> Result<String, Error> {
    let listed = list(project.flag, &project.path, worktree, register)?;
    pick_scheme(&listed, || targets(project, &listed, worktree, register))
}

/// The destination of a build: the simulator or device by id; any simulator while it has none.
pub(crate) fn destination(target: Option<&str>) -> String {
    match target {
        Some(udid) => format!("id={udid}"),
        None => "generic/platform=iOS Simulator".into(),
    }
}

/// The `.app` the scheme builds for the destination, from `xcodebuild -showBuildSettings -json` with the arguments of the build. Runs in its own process group, whose id goes to `register`.
pub(crate) fn built_app(
    project: &Project,
    scheme: &str,
    target: Option<&str>,
    worktree: &Path,
    register: &dyn Fn(u32),
) -> Result<PathBuf, Error> {
    let path = project.path.to_string_lossy();
    let mut command = login_shell(
        "xcodebuild",
        &[
            project.flag,
            &path,
            "-scheme",
            scheme,
            "-destination",
            &destination(target),
            "-showBuildSettings",
            "-json",
        ],
    );
    command.current_dir(worktree);
    let output = process::captured(command, register)?;
    if !output.status.success() {
        return Err(Error::Message(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let app = app_in_settings(&String::from_utf8_lossy(&output.stdout))?;
    if app.exists() {
        Ok(app)
    } else {
        Err(Error::Message(format!(
            "The build made no app at {}",
            app.display()
        )))
    }
}

/// The first target of the scheme that is an app, as `TARGET_BUILD_DIR/FULL_PRODUCT_NAME`. Extensions, watch apps and test bundles have other product types.
fn app_in_settings(text: &str) -> Result<PathBuf, Error> {
    // xcodebuild may print notes before the JSON array.
    let listed: Value = serde_json::from_str(&text[text.find('[').unwrap_or(0)..])?;
    listed
        .as_array()
        .into_iter()
        .flatten()
        .map(|target| &target["buildSettings"])
        .find(|settings| settings["PRODUCT_TYPE"] == "com.apple.product-type.application")
        .and_then(|settings| {
            let dir = settings["TARGET_BUILD_DIR"].as_str()?;
            let name = settings["FULL_PRODUCT_NAME"].as_str()?;
            Some(Path::new(dir).join(name))
        })
        .ok_or_else(|| Error::Message("The scheme builds no app for the destination.".into()))
}

fn list(flag: &str, path: &Path, dir: &Path, register: &dyn Fn(u32)) -> Result<Value, Error> {
    let path = path.to_string_lossy();
    let mut command = login_shell("xcodebuild", &["-list", "-json", flag, &path]);
    command.current_dir(dir);
    let output = process::captured(command, register)?;
    if !output.status.success() {
        return Err(Error::Message(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    parse_list(&String::from_utf8_lossy(&output.stdout))
}

// xcodebuild may print notes before the JSON.
fn parse_list(text: &str) -> Result<Value, Error> {
    Ok(serde_json::from_str(&text[text.find('{').unwrap_or(0)..])?)
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

// Package and Pods schemes match no target, and `-list` sorts them in among the app's.
fn pick_scheme(listed: &Value, targets: impl FnOnce() -> Vec<String>) -> Result<String, Error> {
    let info = listed
        .get("workspace")
        .or_else(|| listed.get("project"))
        .ok_or_else(|| Error::Message("xcodebuild -list returned no project".into()))?;
    let schemes = strings(&info["schemes"]);
    let name = info["name"].as_str().unwrap_or_default();
    if schemes.iter().any(|scheme| scheme == name) {
        return Ok(name.into());
    }
    let targets = targets();
    schemes
        .iter()
        .find(|scheme| targets.contains(scheme))
        .or(schemes.first())
        .cloned()
        .ok_or_else(|| {
            Error::Message(
                "The project has no scheme. Create one in Xcode (Product > Scheme > New Scheme)."
                    .into(),
            )
        })
}

// A workspace has no targets of its own: its sibling projects' targets count, and Pods.xcodeproj sits in Pods/, so its stay out.
fn targets(
    project: &Project,
    listed: &Value,
    worktree: &Path,
    register: &dyn Fn(u32),
) -> Vec<String> {
    if project.flag == "-project" {
        return strings(&listed["project"]["targets"]);
    }
    xcode_bundles(worktree)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|e| e == "xcodeproj"))
        .filter_map(|path| list("-project", &path, worktree, register).ok())
        .flat_map(|sibling| strings(&sibling["project"]["targets"]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(json: &str, targets: &[&str]) -> Result<String, Error> {
        let listed = parse_list(json).map_err(|e| Error::Message(e.to_string()))?;
        pick_scheme(&listed, || targets.iter().map(|t| t.to_string()).collect())
    }

    #[test]
    fn finds_the_app_target_among_the_schemes_targets() {
        let text = r#"note
[{"target":"Widget","buildSettings":{"PRODUCT_TYPE":"com.apple.product-type.app-extension","TARGET_BUILD_DIR":"/b/Debug-iphonesimulator","FULL_PRODUCT_NAME":"Widget.appex"}},
          {"target":"App","buildSettings":{"PRODUCT_TYPE":"com.apple.product-type.application","TARGET_BUILD_DIR":"/b/Debug-iphonesimulator","FULL_PRODUCT_NAME":"My App.app"}}]"#;
        assert_eq!(
            app_in_settings(text).unwrap(),
            PathBuf::from("/b/Debug-iphonesimulator/My App.app")
        );
    }

    #[test]
    fn picks_the_scheme_named_like_the_project_then_like_a_target_then_the_first() {
        assert!(
            matches!(pick(r#"{"project":{"name":"App","schemes":["AppKit","App"]}}"#, &[]), Ok(s) if s == "App")
        );
        assert!(matches!(
            pick(r#"note: x {"workspace":{"name":"W","schemes":["Alamofire","Main","Pods-W"]}}"#, &["Main", "MainTests"]),
            Ok(s) if s == "Main"
        ));
        assert!(
            matches!(pick(r#"{"project":{"name":"App","schemes":["Alamofire","Dev"]}}"#, &[]), Ok(s) if s == "Alamofire")
        );
        assert!(pick(r#"{"project":{"name":"App","schemes":[]}}"#, &[]).is_err());
    }
}
