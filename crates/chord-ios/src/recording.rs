//! A screen recording of the worktree's simulator: `simctl io recordVideo` from a start to a stop or the simulator's shutdown, into the folder of Settings or the Desktop, and a Quick Look thumbnail of it as the preview.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use chord_core::Error;

use crate::process;
use crate::screenshot::{self, Screenshot};
use crate::simulator;

const QLMANAGE: &str = "/usr/bin/qlmanage";
/// How long `simctl` may take to say it records.
const WAIT: Duration = Duration::from_secs(10);
/// Leads the recording's process group and interrupts it when its stdin closes: at a stop, and when Chord quits or crashes, so `simctl` always finishes the file.
const WATCHDOG: &str = "read _; kill -INT 0";

struct Recording {
    worktree: PathBuf,
    preview: bool,
    group: u32,
    /// Dropping it stops the recording.
    _watchdog: ChildStdin,
}

static RECORDINGS: Mutex<Vec<Recording>> = Mutex::new(Vec::new());

fn take(found: impl Fn(&Recording) -> bool) -> Option<Recording> {
    let mut recordings = RECORDINGS
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let at = recordings.iter().position(found)?;
    Some(recordings.remove(at))
}

/// Records the worktree's booted simulator, or with `preview` its preview simulator, into `folder`, else the Desktop. Calls `started` once the first frame is in; returns the video and its preview when the recording ends, by `stop_recording` or by itself.
pub fn record(
    worktree: &Path,
    preview: bool,
    folder: Option<&Path>,
    started: &dyn Fn(),
) -> Result<Screenshot, Error> {
    stop_recording(worktree, preview);
    let (set, udid) = simulator::booted(worktree, preview)?;
    let file = screenshot::free_file(&screenshot::save_folder(folder)?, "Recording", "mov");
    let command = simulator::simctl_command(
        set,
        &[
            "io",
            &udid,
            "recordVideo",
            "--codec=h264",
            "--force",
            &file.to_string_lossy(),
        ],
    )?;
    let mut watchdog = Command::new("/bin/sh")
        .args(["-c", WATCHDOG])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    let group = watchdog.id();
    let Some(stdin) = watchdog.stdin.take() else {
        process::signal_group(group, libc::SIGKILL);
        let _ = watchdog.wait();
        return Err(Error::Message("The recording did not start.".into()));
    };
    let (mut child, reader) = match process::merged(command, Some(group)) {
        Ok(spawned) => spawned,
        Err(error) => {
            drop(stdin);
            let _ = watchdog.wait();
            return Err(error.into());
        }
    };
    let output = process::read_lines(reader);
    let mut said = String::new();
    let until = Instant::now() + WAIT;
    let recording = loop {
        match output.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(line) => {
                let line = String::from_utf8_lossy(&line);
                if line.contains("Recording started") {
                    break true;
                }
                said.push_str(&line);
            }
            Err(RecvTimeoutError::Timeout) => {
                said.clear();
                break false;
            }
            Err(RecvTimeoutError::Disconnected) => break false,
        }
    };
    if !recording {
        drop(stdin);
        process::kill_after_grace(group);
        let _ = child.wait();
        let _ = watchdog.wait();
        let _ = std::fs::remove_file(&file);
        let said = said.trim();
        return Err(Error::Message(if said.is_empty() {
            "The recording did not start.".into()
        } else {
            said.to_string()
        }));
    }
    RECORDINGS
        .lock()
        .unwrap_or_else(|poison| poison.into_inner())
        .push(Recording {
            worktree: worktree.to_path_buf(),
            preview,
            group,
            _watchdog: stdin,
        });
    started();
    // `simctl` writes to the pipe while it finishes the file, so the pipe is read to its end.
    output.iter().for_each(drop);
    let _ = child.wait();
    // Ended by itself, as at a shutdown: its watchdog goes too.
    drop(take(|recording| recording.group == group));
    let _ = watchdog.wait();
    if !file.is_file() {
        return Err(Error::Message("The recording was not saved.".into()));
    }
    let temporary = screenshot::previews_folder()?;
    let thumbnail = Command::new(QLMANAGE)
        .args(["-t", "-s", "160", "-o"])
        .arg(&temporary)
        .arg(&file)
        .output()?;
    let image = temporary.join(format!(
        "{}.png",
        file.file_name().unwrap_or_default().to_string_lossy()
    ));
    if !thumbnail.status.success() || !image.is_file() {
        return Err(Error::Message(format!(
            "The recording is saved as {}, but its preview could not be made.",
            file.display()
        )));
    }
    let shot = screenshot::preview_of(file, &image);
    let _ = std::fs::remove_file(&image);
    shot
}

/// Stops the recording of the worktree's simulator, or with `preview` its preview simulator, if one runs; `record` returns its video.
pub fn stop_recording(worktree: &Path, preview: bool) {
    if let Some(recording) =
        take(|recording| recording.worktree == worktree && recording.preview == preview)
    {
        let group = recording.group;
        drop(recording);
        process::kill_after_grace(group);
    }
}
