//! The console of a run: its tones, and the loop that reads an app's output into batches for the UI.

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Instant;

use serde::Serialize;
use specta::Type;

use crate::process::{BATCH_BYTES, BATCH_INTERVAL};

#[derive(Serialize, Type, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConsoleTone {
    /// The app's stdout and stderr, and OSLog messages below error.
    Output,
    /// An OSLog error or fault.
    Error,
    /// A line Chord writes, not the app.
    Own,
}

/// Console text waiting for its batch, in arrival order, neighbours of one tone joined.
#[derive(Default)]
struct Console {
    parts: Vec<(String, ConsoleTone)>,
    size: usize,
}

impl Console {
    fn push(&mut self, text: &str, tone: ConsoleTone) {
        self.size += text.len();
        match self.parts.last_mut() {
            Some((last, last_tone)) if *last_tone == tone => last.push_str(text),
            _ => self.parts.push((text.into(), tone)),
        }
    }

    fn flush(&mut self, console: &dyn Fn(String, ConsoleTone)) {
        self.size = 0;
        for (text, tone) in self.parts.drain(..) {
            console(text, tone);
        }
    }
}

pub(crate) struct Pumped {
    /// A line passed `is_launch_line`: the app's process ran.
    pub up: bool,
    /// What the launch tool printed before that.
    pub output: String,
}

/// Reads `lines` to their end. Lines before the first one `is_launch_line` accepts are kept in `output`; that line calls `launched`, and every line after it goes to `console` in batches, with the `records` (OSLog) in arrival order. `ended` runs once the lines end, before the last records are read.
pub(crate) fn pump(
    lines: &Receiver<Vec<u8>>,
    records: Option<&Receiver<(String, ConsoleTone)>>,
    is_launch_line: impl Fn(&str) -> bool,
    launched: &dyn Fn(),
    console: &dyn Fn(String, ConsoleTone),
    ended: impl FnOnce(),
) -> Pumped {
    let mut output = String::new();
    let mut up = false;
    let mut pending = Console::default();
    let mut flushed = Instant::now();
    let drain = |pending: &mut Console| {
        while let Some(Ok((text, tone))) = records.map(Receiver::try_recv) {
            pending.push(&text, tone);
        }
    };
    loop {
        drain(&mut pending);
        match lines.recv_timeout(BATCH_INTERVAL) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).replace("\r\n", "\n");
                if up {
                    pending.push(&text, ConsoleTone::Output);
                } else if is_launch_line(&text) {
                    up = true;
                    launched();
                } else {
                    output.push_str(&text);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if pending.size >= BATCH_BYTES || flushed.elapsed() >= BATCH_INTERVAL {
            pending.flush(console);
            flushed = Instant::now();
        }
    }
    ended();
    drain(&mut pending);
    pending.flush(console);
    Pumped { up, output }
}
