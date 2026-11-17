//! Runs git in a folder and returns its output.

use std::path::Path;
use std::process::Command;

use crate::Error;

pub fn run(dir: &Path, args: &[&str]) -> Result<String, Error> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| Error::start_failed("git", "xcode-select --install", error))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Message(stderr.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
