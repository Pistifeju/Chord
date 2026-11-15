//! Spawning the tools a build runs, each in a process group of its own so a stop can end it with its children.

use std::io::{self, PipeReader};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};

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

/// Starts `command` with its output and errors on one pipe; the caller reads the pipe to its end, then waits for the child and registers its group.
pub(crate) fn merged(mut command: Command) -> io::Result<(Child, PipeReader)> {
    let (reader, writer) = io::pipe()?;
    command
        .stdin(Stdio::null())
        .stdout(writer.try_clone()?)
        .stderr(writer)
        .process_group(0);
    let child = command.spawn()?;
    // The command's copies of the pipe's write end must close, or the reader never sees the end.
    drop(command);
    Ok((child, reader))
}
