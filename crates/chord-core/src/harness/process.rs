use crate::lock;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::Error;

pub enum Output<'a> {
    Line(&'a [u8]),
    Closed { exited: Option<Option<i32>> },
}

#[derive(Clone)]
pub struct Stdin(Arc<Mutex<ChildStdin>>);

impl Stdin {
    pub fn write(&self, line: &Value) -> Result<(), Error> {
        let mut stdin = lock(&self.0);
        writeln!(stdin, "{line}")?;
        stdin.flush()?;
        Ok(())
    }
}

pub struct Process {
    child: Arc<Mutex<Child>>,
    stdin: Stdin,
    stopped: Arc<AtomicBool>,
    reports_end: bool,
}

impl Process {
    pub fn start(
        mut command: Command,
        mut on_output: impl FnMut(Output<'_>, &Stdin) + Send + 'static,
    ) -> std::io::Result<Process> {
        let mut child = command
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

    pub fn end_reported(mut self) {
        self.reports_end = true;
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.reports_end {
            self.stopped.store(true, Ordering::SeqCst);
        }
        let pid = lock(&self.child).id();
        if let Ok(pid) = i32::try_from(pid) {
            unsafe { libc::kill(pid, libc::SIGTERM) };
        }
        let deadline = Instant::now() + TERM_GRACE;
        while Instant::now() < deadline {
            let waited = lock(&self.child).try_wait();
            if !matches!(waited, Ok(None)) {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let mut child = lock(&self.child);
        let _ = child.kill();
        let _ = child.wait();
    }
}

const TERM_GRACE: Duration = Duration::from_secs(2);

fn exit_code(child: &Mutex<Child>) -> Option<i32> {
    for _ in 0..50 {
        let waited = lock(child).try_wait();
        match waited {
            Ok(Some(status)) => return status.code(),
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    None
}

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
