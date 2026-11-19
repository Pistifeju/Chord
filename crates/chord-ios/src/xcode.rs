//! The worktree's Xcode project or workspace, its schemes with the configuration each one runs in, its build configurations, and the check that `xcodebuild` and `xcrun` exist.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::SystemTime;

use chord_core::{Error, lock, login_shell, xcode_bundles};
use serde::Serialize;
use serde_json::Value;
use specta::Type;

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

/// What the Run menu offers a project.
#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct Schemes {
    pub schemes: Vec<Scheme>,
    pub configurations: Vec<String>,
    /// The scheme a build without a pick uses.
    pub default: String,
}

#[derive(Serialize, Type, Clone, Debug, PartialEq)]
pub struct Scheme {
    pub name: String,
    /// The configuration a build without a pick uses: the one the scheme's Run action names.
    pub configuration: String,
}

/// The schemes and build configurations of the worktree's project; a workspace's configurations are its sibling projects'.
pub fn list_schemes(worktree: &Path) -> Result<Schemes, Error> {
    require_tools()?;
    let project = project_in(worktree)?;
    let register = |_| {};
    let listed = list(project.flag, &project.path, worktree, &register)?;
    let projects = if project.flag == "-project" {
        vec![listed.clone()]
    } else {
        siblings(worktree, &register)
    };
    let default = pick_scheme(&listed, || {
        projects
            .iter()
            .flat_map(|sibling| strings(&sibling["project"]["targets"]))
            .collect()
    })?;
    let mut configurations: Vec<String> = Vec::new();
    for configuration in projects
        .iter()
        .flat_map(|sibling| strings(&sibling["project"]["configurations"]))
    {
        if !configurations.contains(&configuration) {
            configurations.push(configuration);
        }
    }
    Ok(Schemes {
        schemes: strings(&info(&listed)?["schemes"])
            .into_iter()
            .map(|name| Scheme {
                configuration: run_configuration(worktree, &name),
                name,
            })
            .collect(),
        configurations,
        default,
    })
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

/// The configuration the scheme's Run action names, from its `.xcscheme` in a project or workspace of the worktree, shared or the user's own; Debug for a scheme Xcode creates on the fly, which has no file.
pub(crate) fn run_configuration(worktree: &Path, scheme: &str) -> String {
    let file = format!("{scheme}.xcscheme");
    xcode_bundles(worktree)
        .into_iter()
        .flat_map(|bundle| {
            let users = fs::read_dir(bundle.join("xcuserdata"))
                .into_iter()
                .flatten()
                .flatten()
                .map(|user| user.path().join("xcschemes"));
            std::iter::once(bundle.join("xcshareddata/xcschemes"))
                .chain(users)
                .collect::<Vec<_>>()
        })
        .find_map(|folder| launch_configuration(&fs::read_to_string(folder.join(&file)).ok()?))
        .unwrap_or_else(|| "Debug".into())
}

/// The `buildConfiguration` of the scheme's `<LaunchAction>` tag.
fn launch_configuration(scheme: &str) -> Option<String> {
    let tag = &scheme[scheme.find("<LaunchAction")?..];
    let tag = &tag[..tag.find('>')?];
    let value = &tag[tag.find("buildConfiguration")?..];
    let value = &value[value.find('"')? + 1..];
    Some(value[..value.find('"')?].to_string())
}

/// What xcodebuild builds: the project, the scheme in the configuration, and the destination's id, any simulator while it has none. No `-derivedDataPath` and no build setting, so Xcode's own folder for the project is used.
pub(crate) fn build_args(
    project: &Project,
    scheme: &str,
    configuration: &str,
    target: Option<&str>,
) -> Vec<String> {
    [
        project.flag,
        &project.path.to_string_lossy(),
        "-scheme",
        scheme,
        "-configuration",
        configuration,
        "-destination",
        &destination(target),
    ]
    .map(String::from)
    .into()
}

/// The modification times of the files that decide which app a scheme builds where: each project's `project.pbxproj`, each workspace's contents and the shared schemes in the worktree's folder.
// shortcut: a product name set in an `.xcconfig`, a user scheme or a project below the folder is not watched; add their times if a stale app path shows.
fn project_stamp(worktree: &Path) -> Vec<Option<SystemTime>> {
    xcode_bundles(worktree)
        .into_iter()
        .flat_map(|bundle| {
            let schemes = fs::read_dir(bundle.join("xcshareddata/xcschemes"))
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path());
            [
                bundle.join("project.pbxproj"),
                bundle.join("contents.xcworkspacedata"),
            ]
            .into_iter()
            .chain(schemes)
            .collect::<Vec<_>>()
        })
        .map(|file| file.metadata().and_then(|meta| meta.modified()).ok())
        .collect()
}

/// The `.app` the scheme builds for the destination, from `xcodebuild -showBuildSettings -json` with the arguments of the build, kept until a project or shared scheme changes. Runs in its own process group, whose id goes to `register`.
pub(crate) fn built_app(
    project: &Project,
    scheme: &str,
    configuration: &str,
    target: Option<&str>,
    worktree: &Path,
    register: &dyn Fn(u32),
) -> Result<PathBuf, Error> {
    type Found = HashMap<Vec<String>, (Vec<Option<SystemTime>>, PathBuf)>;
    static FOUND: Mutex<Option<Found>> = Mutex::new(None);
    let mut args = build_args(project, scheme, configuration, target);
    let stamp = project_stamp(worktree);
    let known = lock(&FOUND)
        .as_ref()
        .and_then(|found| found.get(&args))
        .filter(|(known, app)| *known == stamp && app.exists())
        .map(|(_, app)| app.clone());
    if let Some(app) = known {
        return Ok(app);
    }
    let key = args.clone();
    args.extend(["-showBuildSettings".into(), "-json".into()]);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut command = login_shell("xcodebuild", &arg_refs);
    command.current_dir(worktree);
    let output = process::captured(command, register)?;
    if !output.status.success() {
        return Err(Error::Message(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let app = app_in_settings(&String::from_utf8_lossy(&output.stdout))?;
    if app.exists() {
        lock(&FOUND)
            .get_or_insert_default()
            .insert(key, (stamp, app.clone()));
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
    let info = info(listed)?;
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

fn info(listed: &Value) -> Result<&Value, Error> {
    listed
        .get("workspace")
        .or_else(|| listed.get("project"))
        .ok_or_else(|| Error::Message("xcodebuild -list returned no project".into()))
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
    siblings(worktree, register)
        .iter()
        .flat_map(|sibling| strings(&sibling["project"]["targets"]))
        .collect()
}

/// The `-list` of each project beside a workspace.
fn siblings(worktree: &Path, register: &dyn Fn(u32)) -> Vec<Value> {
    xcode_bundles(worktree)
        .into_iter()
        .filter(|path| path.extension().is_some_and(|e| e == "xcodeproj"))
        .filter_map(|path| list("-project", &path, worktree, register).ok())
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
    fn reads_the_run_actions_configuration() {
        let scheme = r#"<Scheme>
   <BuildAction buildConfiguration = "Release"></BuildAction>
   <LaunchAction
      buildConfiguration = "Staging"
      launchStyle = "0">
   </LaunchAction>
</Scheme>"#;
        assert_eq!(launch_configuration(scheme).as_deref(), Some("Staging"));
        assert_eq!(launch_configuration("<Scheme></Scheme>"), None);
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
