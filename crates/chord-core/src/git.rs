use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::Error;

fn command(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(test)]
    command
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    command.arg("-C").arg(dir).args(args);
    command
}

fn started(result: std::io::Result<Output>) -> Result<String, Error> {
    let output =
        result.map_err(|error| Error::start_failed("git", "xcode-select --install", error))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = match stderr.trim() {
            "" => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            stderr => stderr.to_string(),
        };
        return Err(Error::Message(reason));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn in_repository(dir: &Path) -> bool {
    dir.ancestors().any(|folder| folder.join(".git").exists())
}

pub fn run(dir: &Path, args: &[&str]) -> Result<String, Error> {
    started(command(dir, args).output())
}

pub fn read(dir: &Path, args: &[&str]) -> Result<String, Error> {
    run(
        dir,
        &[&["--no-optional-locks", "--literal-pathspecs"], args].concat(),
    )
}

pub fn run_with_input(dir: &Path, args: &[&str], input: &[u8]) -> Result<String, Error> {
    started((|| {
        let mut child = command(dir, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let _ = child.stdin.take().expect("piped stdin").write_all(input);
        child.wait_with_output()
    })())
}

pub fn optional(result: Result<String, Error>) -> Result<Option<String>, Error> {
    match result {
        Ok(output) => Ok(Some(output.trim().to_string())),
        Err(error @ Error::MissingTool { .. }) => Err(error),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_reads_a_git_failure_as_nothing_but_keeps_a_missing_git() {
        assert!(matches!(optional(Ok(" main\n".into())), Ok(Some(text)) if text == "main"));
        assert!(matches!(
            optional(Err(Error::Message("no upstream".into()))),
            Ok(None)
        ));
        let missing = Error::MissingTool {
            tool: "git".into(),
            install: "xcode-select --install".into(),
        };
        assert!(matches!(
            optional(Err(missing)),
            Err(Error::MissingTool { .. })
        ));
    }
}
