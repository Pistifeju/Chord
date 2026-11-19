//! Runs git in a folder, with or without input on its stdin, and returns its output, or None for a failure that only means the thing asked for is not there.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::Error;

fn command(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    // The app has no terminal to ask in: a push without credentials fails with git's reason.
    command.env("GIT_TERMINAL_PROMPT", "0");
    command.arg("-C").arg(dir).args(args);
    command
}

fn started(result: std::io::Result<Output>) -> Result<String, Error> {
    let output =
        result.map_err(|error| Error::start_failed("git", "xcode-select --install", error))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // A stash that conflicts reports on stdout alone.
        let reason = match stderr.trim() {
            "" => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr => stderr.to_string(),
        };
        return Err(Error::Message(reason));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn run(dir: &Path, args: &[&str]) -> Result<String, Error> {
    started(command(dir, args).output())
}

pub fn run_with_input(dir: &Path, args: &[&str], input: &[u8]) -> Result<String, Error> {
    started((|| {
        let mut child = command(dir, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        // git reads all of its input before it writes, so writing first cannot block on a full stdout. A failed write means git quit, and its stderr says why.
        let _ = child.stdin.take().expect("piped stdin").write_all(input);
        child.wait_with_output()
    })())
}

/// Other git failures read as None, so a detached HEAD, a branch without an upstream or a stash without untracked files reads as nothing there.
pub fn optional(result: Result<String, Error>) -> Result<Option<String>, Error> {
    match result {
        Ok(output) => Ok(Some(output.trim().to_string())),
        Err(error @ Error::MissingTool { .. }) => Err(error),
        Err(_) => Ok(None),
    }
}
