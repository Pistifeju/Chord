//! A harness's process: started through the user's login shell on pipes, its stdout read line by line on a thread of its own, JSON lines written to its stdin, killed when dropped.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::Error;

/// What the process's stdout gives the adapter.
pub enum Output<'a> {
    /// One line, with its LF.
    Line(&'a [u8]),
    /// Stdout closed and nothing follows. `exited` holds the exit code, missing when a signal ended it; it is none when the process was dropped, since its pane is already forgotten.
    Closed { exited: Option<Option<i32>> },
}

/// The process's stdin, for the reader thread to answer on.
#[derive(Clone)]
pub struct Stdin(Arc<Mutex<ChildStdin>>);

impl Stdin {
    pub fn write(&self, line: &Value) -> Result<(), Error> {
        let mut stdin = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        writeln!(stdin, "{line}")?;
        stdin.flush()?;
        Ok(())
    }
}

pub struct Process {
    child: Arc<Mutex<Child>>,
    stdin: Stdin,
    stopped: Arc<AtomicBool>,
    /// Set when the pane stays, so its chat hears the end as an exit.
    reports_end: bool,
}

impl Process {
    /// Starts `binary` with `args` in `dir`. `on_output` runs on the reader thread for each stdout line, then once with `Closed`.
    pub fn start(
        binary: &str,
        args: &[&str],
        dir: &Path,
        mut on_output: impl FnMut(Output<'_>, &Stdin) + Send + 'static,
    ) -> std::io::Result<Process> {
        let mut child = login_shell(binary, args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(std::io::Error::other("it started without its pipes"));
        };
        let child = Arc::new(Mutex::new(child));
        let reader_child = Arc::clone(&child);
        let stdin = Stdin(Arc::new(Mutex::new(stdin)));
        let reader_stdin = stdin.clone();
        let stopped = Arc::new(AtomicBool::new(false));
        let reader_stopped = Arc::clone(&stopped);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            // Split on LF only: a JSON string may hold U+2028.
            while matches!(reader.read_until(b'\n', &mut line), Ok(read) if read > 0) {
                on_output(Output::Line(&line), &reader_stdin);
                line.clear();
            }
            let exited = (!reader_stopped.load(Ordering::SeqCst)).then(|| exit_code(&reader_child));
            on_output(Output::Closed { exited }, &reader_stdin);
        });
        Ok(Process {
            child,
            stdin,
            stopped,
            reports_end: false,
        })
    }

    pub fn write(&self, line: &Value) -> Result<(), Error> {
        self.stdin.write(line)
    }

    /// Ends the process and reports its exit, for a pane that stays.
    pub fn end_reported(mut self) {
        self.reports_end = true;
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.reports_end {
            self.stopped.store(true, Ordering::SeqCst);
        }
        let mut child = self.child.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = child.kill();
        let _ = child.wait();
    }
}

// Stdout closed, so the process is ending; it gets a moment to report its code.
fn exit_code(child: &Mutex<Child>) -> Option<i32> {
    for _ in 0..50 {
        let waited = child
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .try_wait();
        match waited {
            Ok(Some(status)) => return status.code(),
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    None
}

/// The user's login shell finds the harness and its API keys the way their terminal does.
/// Without a SHELL, the binary runs directly and finds itself on the app's own PATH.
pub fn login_shell(binary: &str, args: &[&str]) -> Command {
    match std::env::var("SHELL") {
        Ok(shell) if !shell.is_empty() => {
            let mut command = Command::new(shell);
            command
                .args(["-l", "-c", r#"exec "$0" "$@""#, binary])
                .args(args);
            command
        }
        _ => {
            let mut command = Command::new(binary);
            command.args(args);
            command
        }
    }
}
