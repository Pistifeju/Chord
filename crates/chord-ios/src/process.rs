//! Spawning the tools a build runs, each in a process group so a stop can end it with its children, and reading their output in lines and batches.

use std::io::{self, BufRead, BufReader, PipeReader};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use chord_core::{Error, login_shell};

/// Output reaches the UI every 100 ms or 64 KB, whichever comes first: a cold build prints thousands of lines.
pub(crate) const BATCH_INTERVAL: Duration = Duration::from_millis(100);
pub(crate) const BATCH_BYTES: usize = 64 * 1024;

/// How long a signalled group may take to end before it is killed, and how long the build database may stay held after it.
pub(crate) const GROUP_GRACE: Duration = Duration::from_secs(5);

/// Runs `command` to its end; its process group's id goes to `register` first.
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

/// Runs `xcrun` with `args` in a process group of its own, whose id goes to `register` so a stop can end it; its output when it succeeds, else its error text.
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

/// Starts `command` with its output and errors on one pipe, in the process group `group`, else a new one; the caller reads the pipe to its end, then waits for the child and registers its group.
pub(crate) fn merged(mut command: Command, group: Option<u32>) -> io::Result<(Child, PipeReader)> {
    let (reader, writer) = io::pipe()?;
    command
        .stdin(Stdio::null())
        .stdout(writer.try_clone()?)
        .stderr(writer)
        .process_group(group.map_or(0, |id| id as i32));
    let child = command.spawn()?;
    // The command's copies of the pipe's write end must close, or the reader never sees the end.
    drop(command);
    Ok((child, reader))
}

/// The reader's lines, read on a thread of its own; the channel closes at the pipe's end.
pub(crate) fn read_lines(reader: PipeReader) -> Receiver<Vec<u8>> {
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
    // SAFETY: `kill` takes plain integers and has no memory effects.
    unsafe { libc::kill(-pid, signal) == 0 }
}

/// Asks the group to end, waits, then kills what is left, so no compiler keeps the build database locked.
pub(crate) fn end_group(pid: u32) {
    signal_group(pid, libc::SIGTERM);
    kill_after_grace(pid);
}

/// Waits for the group to end, and kills what is left after `GROUP_GRACE`.
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
