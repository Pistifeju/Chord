//! The worktrees' terminals: one login shell per worktree on a pseudo-terminal, its output handed out as text, its input and size taken in.

use crate::lock;
use std::collections::HashMap;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{io, thread};

use serde::Serialize;
use specta::Type;

use crate::Error;

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminalEvent {
    Output {
        text: String,
    },
    /// The shell ended, on its own or stopped; a shell a newer start replaced sends nothing.
    Exited,
}

struct Shell {
    id: u64,
    master: File,
    /// The shell's pid, which is also its process group's id.
    pid: i32,
    /// Set once the shell ended and was reaped.
    ended: Arc<AtomicBool>,
}

impl Shell {
    fn signal(&self, signal: i32) {
        if !self.ended.load(Ordering::SeqCst) {
            unsafe { libc::kill(-self.pid, signal) };
        }
    }
}

// The shells get the hangup a closed terminal window sends, so they pass it on to their jobs; a shell that ignores it is killed after a second.
fn end(shells: impl IntoIterator<Item = Shell>) {
    let shells: Vec<Shell> = shells.into_iter().collect();
    shells.iter().for_each(|shell| shell.signal(libc::SIGHUP));
    let deadline = Instant::now() + Duration::from_secs(1);
    for shell in &shells {
        while !shell.ended.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        shell.signal(libc::SIGKILL);
    }
}

#[derive(Default)]
pub struct Terminals {
    shells: Arc<Mutex<HashMap<PathBuf, Shell>>>,
    next_id: AtomicU64,
}

impl Terminals {
    /// Starts the user's `$SHELL` as a login shell in the worktree, replacing a shell it already had.
    pub fn start(
        &self,
        worktree: PathBuf,
        cols: u16,
        rows: u16,
        on_event: impl Fn(TerminalEvent) + Send + 'static,
    ) -> Result<(), Error> {
        let (master, slave) = open_pty(cols, rows)?;
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|shell| !shell.is_empty())
            .unwrap_or_else(|| "/bin/zsh".into());
        let mut command = Command::new(&shell);
        command
            .arg("-l")
            .current_dir(&worktree)
            .env("TERM", "xterm-256color")
            .env("COLORTERM", "truecolor");
        // An app started from the Dock has no locale, and a shell without one garbles UTF-8.
        if ["LANG", "LC_ALL", "LC_CTYPE"]
            .iter()
            .all(|name| std::env::var_os(name).is_none())
        {
            command.env("LC_CTYPE", "UTF-8");
        }
        let slave_fd = slave.as_raw_fd();
        unsafe {
            command.pre_exec(move || {
                if libc::login_tty(slave_fd) == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command
            .spawn()
            .map_err(|error| Error::start_failed(&shell, "chsh -s /bin/zsh", error))?;
        drop(slave);
        let mut reader = master.try_clone()?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let ended = Arc::new(AtomicBool::new(false));
        let replaced = lock(&self.shells).insert(
            worktree.clone(),
            Shell {
                id,
                master,
                pid: child.id() as i32,
                ended: Arc::clone(&ended),
            },
        );
        end(replaced);
        let reaped = Arc::clone(&ended);
        thread::spawn(move || {
            let _ = child.wait();
            reaped.store(true, Ordering::SeqCst);
        });
        let shells = Arc::clone(&self.shells);
        thread::spawn(move || {
            let mut buffer = vec![0; 64 * 1024];
            let mut pending = Vec::new();
            // A job the shell left running can hold the pty open forever, so reading stops once the shell ended and its output is drained.
            while has_output(&reader, &ended) {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        pending.extend_from_slice(&buffer[..read]);
                        let text = take_text(&mut pending);
                        if !text.is_empty() {
                            on_event(TerminalEvent::Output { text });
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    // The pty answers EIO once the shell and everything it started closed it.
                    Err(_) => break,
                }
            }
            let mut shells = lock(&shells);
            let replaced = shells.get(&worktree).is_some_and(|shell| shell.id != id);
            if !replaced {
                shells.remove(&worktree);
                drop(shells);
                on_event(TerminalEvent::Exited);
            }
        });
        Ok(())
    }

    pub fn write(&self, worktree: &Path, text: &str) -> Result<(), Error> {
        // A shell that stops reading must not hold the other worktrees' shells.
        let mut master = self.master(worktree)?;
        master.write_all(text.as_bytes())?;
        Ok(())
    }

    pub fn resize(&self, worktree: &Path, cols: u16, rows: u16) -> Result<(), Error> {
        let master = self.master(worktree)?;
        let size = window_size(cols, rows);
        if unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ, &size) } == -1 {
            return Err(io::Error::last_os_error().into());
        }
        Ok(())
    }

    /// Ends the worktree's shell; returns within a second.
    pub fn stop(&self, worktree: &Path) {
        let stopped = lock(&self.shells).remove(worktree);
        end(stopped);
    }

    /// Ends every shell; returns within a second.
    pub fn stop_all(&self) {
        let stopped = std::mem::take(&mut *lock(&self.shells));
        end(stopped.into_values());
    }

    fn master(&self, worktree: &Path) -> Result<File, Error> {
        let shells = lock(&self.shells);
        let shell = shells
            .get(worktree)
            .ok_or_else(|| Error::Message("The terminal has no shell".into()))?;
        Ok(shell.master.try_clone()?)
    }
}

/// Waits until the pty has output; false once the shell ended and nothing is left to read.
fn has_output(reader: &File, ended: &AtomicBool) -> bool {
    loop {
        let shell_ended = ended.load(Ordering::SeqCst);
        let mut poll = libc::pollfd {
            fd: reader.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let timeout = if shell_ended { 0 } else { 100 };
        match unsafe { libc::poll(&mut poll, 1, timeout) } {
            0 if shell_ended => return false,
            0 => {}
            -1 if io::Error::last_os_error().kind() == ErrorKind::Interrupted => {}
            // Readable, hung up or failed: the read tells which.
            _ => return true,
        }
    }
}

fn window_size(cols: u16, rows: u16) -> libc::winsize {
    libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    }
}

fn open_pty(cols: u16, rows: u16) -> Result<(File, OwnedFd), Error> {
    let (mut master, mut slave) = (0, 0);
    let mut size = window_size(cols, rows);
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if opened == -1 {
        return Err(io::Error::last_os_error().into());
    }
    let (master, slave) = unsafe { (File::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    // No other process Chord starts may hold the shell's terminal open.
    for fd in [master.as_raw_fd(), slave.as_raw_fd()] {
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    Ok((master, slave))
}

/// The text the bytes hold; a character split across two reads waits in `pending` for its rest.
fn take_text(pending: &mut Vec<u8>) -> String {
    // Valid text, the usual read, moves into the string without a second check or a copy.
    let error = match String::from_utf8(std::mem::take(pending)) {
        Ok(text) => return text,
        Err(error) => error,
    };
    let incomplete = match error.utf8_error().error_len() {
        None => error.as_bytes().len() - error.utf8_error().valid_up_to(),
        Some(_) => 0,
    };
    let mut bytes = error.into_bytes();
    *pending = bytes.split_off(bytes.len() - incomplete);
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_split_across_reads_waits_for_its_rest() {
        let mut pending = vec![b'a', 0xC3];
        assert_eq!(take_text(&mut pending), "a");
        assert_eq!(pending, [0xC3]);
        pending.push(0xA9);
        assert_eq!(take_text(&mut pending), "é");
        assert!(pending.is_empty());
    }
}
