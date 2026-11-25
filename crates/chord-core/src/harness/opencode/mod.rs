//! The OpenCode adapter: runs `opencode acp` as a process and hands its stdout to the reader. The only module that knows OpenCode's records.

mod asks;
mod commands;
mod reader;
mod requests;
#[cfg(test)]
mod tests;
mod updates;

use std::path::Path;

use super::relaunch::Relaunchable;
use super::{Agent, Harness, HarnessEvent};
use crate::Error;
use reader::Reader;

/// The harness name a pane saves.
pub const NAME: &str = "opencode";

pub const HARNESS: Harness = Harness {
    name: NAME,
    title: "OpenCode",
    binary: "opencode",
    install: "npm install -g @opencode/cli",
    sign_in: None,
    upgrade: "opencode upgrade",
    oldest_tested: (2, 4, 1),
    starts: true,
};

/// Starts the pane's OpenCode in `dir` on a new session; `/new` swaps its process for another.
pub fn start(
    dir: &Path,
    on_event: impl Fn(HarnessEvent) + Send + 'static,
) -> Result<Box<dyn Agent>, Error> {
    let cwd = dir.to_path_buf();
    let launch = move |()| (vec!["acp".to_string()], Reader::new(&cwd));
    Relaunchable::start(&HARNESS, dir, (), launch, on_event)
        .map(|agent| Box::new(agent) as Box<dyn Agent>)
}
