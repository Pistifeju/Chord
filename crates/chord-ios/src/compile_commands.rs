//! Each Swift module's compile commands for the preview hot reload, without a build setting: the driver line a Chord build of the worktree printed for an arm64 simulator, kept in the worktree's hot reload folder, and the one `swift-frontend` command per file the driver lists for it with `-###`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, PoisonError};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::preview;
use crate::process;

const DRIVER: &str = "builtin-SwiftDriver -- ";
const DRIVERS: &str = "drivers.json";

/// A module's `swiftc` line and the folder the build ran it in.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash)]
pub(crate) struct Driver {
    cwd: String,
    args: Vec<String>,
}

impl Driver {
    pub(crate) fn module(&self) -> Option<&str> {
        value_of(&self.args, "-module-name")
    }

    /// The module's `.swift` files from the file list its last build wrote, symbolic links resolved.
    pub(crate) fn files(&self) -> Vec<String> {
        self.file_lists()
            .lines()
            .map(|line| real(line.trim().trim_matches('"')))
            .filter(|path| path.ends_with(".swift"))
            .collect()
    }

    fn file_lists(&self) -> String {
        self.args
            .iter()
            .filter_map(|arg| arg.strip_prefix('@'))
            .filter_map(|list| fs::read_to_string(list).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The driver lines of a build's output, each with the folder of the `cd` line before it.
#[derive(Default)]
pub(crate) struct DriverLines {
    cwd: String,
    found: Vec<Driver>,
}

impl DriverLines {
    pub(crate) fn push(&mut self, line: &str) {
        let line = line.trim();
        if let Some(folder) = line.strip_prefix("cd ") {
            self.cwd = split(folder).concat();
        } else if let Some(command) = line.strip_prefix(DRIVER) {
            let args = split(command);
            let simulator = value_of(&args, "-target").is_some_and(|target| {
                target.starts_with("arm64-") && target.ends_with("-simulator")
            });
            if simulator {
                self.found.push(Driver {
                    cwd: self.cwd.clone(),
                    args,
                });
            }
        }
    }

    /// Keeps the lines in the worktree's hot reload folder over the ones of earlier builds: a build prints only the modules it compiled.
    pub(crate) fn record(self, worktree: &Path) {
        let Some(folder) = preview::hot_reload_folder(worktree) else {
            return;
        };
        if self.found.is_empty() {
            return;
        }
        // Two builds of a worktree can end at once; a reload reads the file meanwhile.
        static WRITING: Mutex<()> = Mutex::new(());
        let _writing = WRITING.lock().unwrap_or_else(PoisonError::into_inner);
        let mut drivers: BTreeMap<String, Driver> = read(&folder.join(DRIVERS)).unwrap_or_default();
        for driver in self.found {
            if let Some(module) = driver.module() {
                drivers.insert(module.to_string(), driver);
            }
        }
        let _ = fs::create_dir_all(&folder);
        let partial = folder.join("drivers.partial");
        if let Ok(json) = serde_json::to_vec(&drivers)
            && fs::write(&partial, json).is_ok()
        {
            let _ = fs::rename(&partial, folder.join(DRIVERS));
        }
    }
}

/// The modules the worktree's builds compiled for a simulator.
pub(crate) fn drivers(worktree: &Path) -> Vec<Driver> {
    preview::hot_reload_folder(worktree)
        .and_then(|folder| read::<BTreeMap<String, Driver>>(&folder.join(DRIVERS)))
        .map(|drivers| drivers.into_values().collect())
        .unwrap_or_default()
}

/// One file's `swift-frontend` command, and the object the build wrote for it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct Job {
    pub args: Vec<String>,
    pub object: String,
}

#[derive(Serialize, Deserialize)]
struct Jobs {
    key: u64,
    files: HashMap<String, Job>,
}

/// The module's command per source file, by its real path: asked of the driver once per driver line and file list, then read back from the folder.
pub(crate) fn jobs(worktree: &Path, driver: &Driver) -> Result<HashMap<String, Job>, String> {
    let folder = preview::hot_reload_folder(worktree).ok_or("Chord has no cache folder")?;
    let module = driver.module().ok_or("the driver line names no module")?;
    let mut hasher = DefaultHasher::new();
    driver.hash(&mut hasher);
    driver.file_lists().hash(&mut hasher);
    let key = hasher.finish();
    let saved = folder.join(format!("jobs-{module}.json"));
    if let Some(jobs) = read::<Jobs>(&saved).filter(|jobs| jobs.key == key) {
        return Ok(jobs.files);
    }
    let (swiftc, args) = driver
        .args
        .split_first()
        .ok_or("the driver line is empty")?;
    // The driver's temporary files, the module map of the explicit module build among them, stay as long as its commands and go with the folder.
    let temporary = folder.join(format!("driver-{module}"));
    let _ = fs::remove_dir_all(&temporary);
    fs::create_dir_all(&temporary).map_err(|error| error.to_string())?;
    let mut command = Command::new(swiftc);
    command
        .args(args.iter().filter(|arg| *arg != "-incremental"))
        .args([
            "-disable-batch-mode",
            "-driver-filelist-threshold",
            "1000000",
            "-###",
        ])
        .current_dir(&driver.cwd)
        .env("TMPDIR", &temporary);
    let output = process::captured(command, &|_| {}).map_err(|error| format!("swiftc: {error}"))?;
    if !output.status.success() {
        let errors = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "the driver listed no commands: {}",
            errors.lines().next().unwrap_or_default()
        ));
    }
    let files = parse_jobs(&String::from_utf8_lossy(&output.stdout));
    if let Ok(json) = serde_json::to_vec(&Jobs {
        key,
        files: files.clone(),
    }) {
        let _ = fs::write(saved, json);
    }
    Ok(files)
}

fn parse_jobs(text: &str) -> HashMap<String, Job> {
    text.lines()
        .filter(|line| line.contains(" -frontend -c "))
        .filter_map(|line| {
            let args = split(line);
            let primary = real(value_of(&args, "-primary-file")?);
            let object = value_of(&args, "-o")?.to_string();
            Some((primary, Job { args, object }))
        })
        .collect()
}

fn read<T: DeserializeOwned>(file: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(file).ok()?).ok()
}

/// The path with symbolic links resolved, as file events name it; the path itself when it is gone.
pub(crate) fn real(path: &str) -> String {
    fs::canonicalize(path).map_or_else(
        |_| path.to_string(),
        |real: PathBuf| real.to_string_lossy().into_owned(),
    )
}

pub(crate) fn value_of<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

/// A command line split as the shell would: the build log escapes with backslashes (`-DDEBUG\=1`), `-###` quotes.
pub(crate) fn split(line: &str) -> Vec<String> {
    let (mut out, mut current, mut quote, mut any) = (Vec::new(), String::new(), None, false);
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (None | Some('"'), '\\') => {
                current.extend(chars.next());
                any = true;
            }
            (None, '\'' | '"') => (quote, any) = (Some(c), true),
            (Some(open), c) if c == open => quote = None,
            (None, c) if c.is_whitespace() => {
                if any {
                    out.push(std::mem::take(&mut current));
                    any = false;
                }
            }
            (_, c) => {
                current.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_like_the_shell() {
        assert_eq!(
            split(r#"a -DDEBUG\=1 b\ c 'd e' "f\"g"  "#),
            ["a", "-DDEBUG=1", "b c", "d e", "f\"g"]
        );
        assert_eq!(split("x '' y"), ["x", "", "y"]);
    }

    #[test]
    fn keeps_arm64_simulator_driver_lines_with_their_folder() {
        let mut lines = DriverLines::default();
        for line in [
            "SwiftDriver Lib normal arm64 com.apple.xcode.tools.swift.compiler (in target 'Lib' from project 'Lib')\n",
            "    cd /W/My\\ App\n",
            "    builtin-SwiftDriver -- /X/swiftc -module-name Lib @/DD/Lib.SwiftFileList -DDEBUG\\=1 -target arm64-apple-ios17.0-simulator -c\n",
            "    builtin-SwiftDriver -- /X/swiftc -module-name Lib -target x86_64-apple-ios17.0-simulator -c\n",
            "    builtin-SwiftDriver -- /X/swiftc -module-name Lib -target arm64-apple-ios17.0 -c\n",
        ] {
            lines.push(line);
        }
        assert_eq!(
            lines.found,
            [Driver {
                cwd: "/W/My App".into(),
                args: [
                    "/X/swiftc",
                    "-module-name",
                    "Lib",
                    "@/DD/Lib.SwiftFileList",
                    "-DDEBUG=1",
                    "-target",
                    "arm64-apple-ios17.0-simulator",
                    "-c"
                ]
                .map(String::from)
                .into(),
            }]
        );
    }

    #[test]
    fn reads_one_job_per_primary_file() {
        let jobs = parse_jobs(
            "/X/swift-frontend -frontend -emit-module /W/A.swift /W/B.swift -o /DD/Lib.swiftmodule\n\
             /X/swift-frontend -frontend -c -primary-file /W/A.swift /W/B.swift '-DX=1' -o /DD/A.o\n\
             /X/swift-frontend -frontend -c /W/A.swift -primary-file /W/B.swift -o /DD/B.o\n",
        );
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs["/W/B.swift"].object, "/DD/B.o");
        assert_eq!(
            jobs["/W/A.swift"].args,
            [
                "/X/swift-frontend",
                "-frontend",
                "-c",
                "-primary-file",
                "/W/A.swift",
                "/W/B.swift",
                "-DX=1",
                "-o",
                "/DD/A.o"
            ]
        );
    }
}
