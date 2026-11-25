mod asks;
mod commands;
mod reader;
mod requests;
#[cfg(test)]
mod tests;
mod updates;

use std::path::Path;

use super::relaunch::Relaunchable;
use super::{Agent, Harness, HarnessEvent, agent_tools};
use crate::Error;
use reader::Reader;

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

pub fn start(
    dir: &Path,
    session: Option<&str>,
    tools: Option<&Path>,
    on_event: impl Fn(HarnessEvent) + Send + 'static,
) -> Result<Box<dyn Agent>, Error> {
    let (cwd, tools) = (dir.to_path_buf(), tools.map(Path::to_path_buf));
    let launch = move |session: Option<String>| {
        let server = tools
            .as_deref()
            .and_then(|socket| agent_tools::command(socket, &cwd));
        let instructions = server.is_some().then(agent_tools::instructions);
        let reader = Reader::new(&cwd, session, server, instructions);
        (vec!["acp".to_string()], reader)
    };
    Relaunchable::start(&HARNESS, dir, session.map(String::from), launch, on_event)
        .map(|agent| Box::new(agent) as Box<dyn Agent>)
}
