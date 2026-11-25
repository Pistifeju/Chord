use std::fs::File;
use std::io::{self, BufRead, BufReader, PipeReader, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use chord_core::{Error, login_shell};

pub(crate) const BATCH_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const BATCH_BYTES: usize = 64 * 1024;

pub(crate) const GROUP_GRACE: Duration = Duration::from_secs(5);

pub(crate) fn captured(mut command: Command, register: &dyn Fn(u32)) -> io::Result<Output> {
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    register(child.id());
    child.wait_with_output()
}

pub(crate) fn checked(mut command: Command, failure: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("{failure}: {error}"))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let errors = String::from_utf8_lossy(&output.stderr);
    let first = errors
        .lines()
        .find(|line| !line.ends_with(": replacing existing signature"))
        .unwrap_or("no error output");
    Err(format!("{failure}: {first}"))
}

pub(crate) fn xcrun(args: &[&str], register: &dyn Fn(u32)) -> Result<String, Error> {
    let output = captured(login_shell("xcrun", args), register)?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(Error::Message(if error.is_empty() {
        format!("xcrun {} failed", args.join(" "))
    } else {
        error
    }))
}

pub(crate) fn merged(mut command: Command, group: Option<u32>) -> io::Result<(Child, PipeReader)> {
    let (reader, writer) = io::pipe()?;
    command
        .stdin(Stdio::null())
        .stdout(writer.try_clone()?)
        .stderr(writer)
        .process_group(group.map_or(0, |id| id as i32));
    let child = command.spawn()?;
    drop(command);
    Ok((child, reader))
}

pub(crate) fn merged_on_terminal(
    mut command: Command,
    group: Option<u32>,
) -> Result<(Child, File), Error> {
    let (terminal, output) = chord_core::terminal::open_pty(200, 50)?;
    command
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .process_group(group.map_or(0, |id| id as i32));
    let child = command.spawn()?;
    drop(command);
    Ok((child, terminal))
}

pub(crate) fn read_lines(reader: impl Read + Send + 'static) -> Receiver<Vec<u8>> {
    let (lines, received) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        loop {
            let mut line = Vec::new();
            if !matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0)
                || lines.send(line).is_err()
            {
                return;
            }
        }
    });
    received
}

pub(crate) fn signal_group(pid: u32, signal: i32) -> bool {
    let Ok(pid) = i32::try_from(pid) else {
        return false;
    };
    unsafe { libc::kill(-pid, signal) == 0 }
}

pub(crate) fn end_group(pid: u32) {
    signal_group(pid, libc::SIGTERM);
    kill_after_grace(pid);
}

pub(crate) fn kill_after_grace(pid: u32) {
    let until = Instant::now() + GROUP_GRACE;
    while signal_group(pid, 0) {
        if Instant::now() > until {
            signal_group(pid, libc::SIGKILL);
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
}
