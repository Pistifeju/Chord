//! The Codex adapter: runs `codex app-server` as a process and hands its stdout to the reader. The only module that knows Codex's records.

mod asks;
mod commands;
mod items;
mod permissions;
mod reader;
mod requests;
pub(super) mod spend;
#[cfg(test)]
mod tests;
mod usage;

use std::path::Path;

use serde_json::json;

use super::models::{self, Step, listed};
use super::relaunch::Relaunchable;
use super::{
    Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, ModelOption, SignIn, agent_tools,
};
use crate::{Error, lock};
use commands::Settings;
use reader::{Reader, Relaunch};

/// The harness name a pane saves.
pub const NAME: &str = "codex";

pub const HARNESS: Harness = Harness {
    name: NAME,
    title: "Codex",
    binary: "codex",
    install: "npm install -g @openai/codex",
    sign_in: Some(SignIn {
        check: &["login", "status"],
        step: "codex login",
    }),
    upgrade: "codex update",
    oldest_tested: (0, 158, 0),
    starts: true,
};

/// Codex's 60 s tool timeout would end a cold build's call.
const TOOL_TIMEOUT_SEC: u32 = 3600;

/// Codex's models, read from an app server that opens no thread.
pub(crate) fn models() -> Result<Vec<ModelOption>, Error> {
    let mut settings = Settings::default();
    models::ask(
        &HARNESS,
        &["app-server"],
        &[requests::initialize(1)],
        move |record| match record["id"].as_u64() {
            Some(1) => Step::Write(vec![requests::initialized(), requests::model_list(2)]),
            Some(2) => listed(settings.read_models(&record["result"])),
            _ => Step::Wait,
        },
    )
}

/// The pane's Codex; `/new` and `/reload` swap its process for another.
pub struct Session(Relaunchable<Reader>);

impl Session {
    /// Starts Codex in `dir` on the saved thread `session`, else on a new thread. With `tools`, the socket Chord's tools listen on, the agent gets the build and run tools.
    pub fn start(
        dir: &Path,
        session: Option<&str>,
        tools: Option<&Path>,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        let open = Relaunch {
            thread: session.map(String::from),
            ..Relaunch::default()
        };
        let (cwd, tools) = (dir.to_path_buf(), tools.map(Path::to_path_buf));
        let launch = move |open| launch(&cwd, open, tools.as_deref());
        Relaunchable::start(&HARNESS, dir, open, launch, on_event).map(Session)
    }
}

fn launch(dir: &Path, open: Relaunch, tools: Option<&Path>) -> (Vec<String>, Reader) {
    let reader = Reader::new(dir, open.thread.as_deref(), open.picks);
    let mut args = vec!["app-server".to_string()];
    if let Some((command, server_args)) = tools.and_then(|socket| agent_tools::command(socket, dir))
    {
        let overrides = [
            ("command", json!(command).to_string()),
            ("args", json!(server_args).to_string()),
            ("tool_timeout_sec", TOOL_TIMEOUT_SEC.to_string()),
            ("default_tools_approval_mode", r#""approve""#.into()),
        ];
        for (key, value) in overrides {
            args.extend(["-c".into(), format!("mcp_servers.chord.{key}={value}")]);
        }
    }
    (args, reader)
}

impl Agent for Session {
    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability::Chat,
            Capability::Attachments,
            Capability::Steer,
            Capability::Models,
            Capability::Context,
            Capability::Subagents,
        ]
    }

    /// Starts a turn, steers the running one, or runs a session command.
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<bool, Error> {
        let reader = self.0.reader();
        let took = lock(&reader).takes_images(text);
        let images = if took { images } else { &[] };
        let relaunch = lock(&reader).relaunch(text);
        if let Some(open) = relaunch {
            return self.0.relaunch(open).map(|()| false);
        }
        let read = lock(&reader).send(text, images, held_elsewhere)?;
        self.0.deliver(read.into()).map(|()| took)
    }

    fn abort(&self) -> Result<(), Error> {
        let interrupt = lock(&self.0.reader()).interrupt();
        interrupt.map_or(Ok(()), |line| self.0.write(&line))
    }

    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        let read = lock(&self.0.reader()).answer(id, answer)?;
        self.0.deliver(read.into())
    }

    fn pick_model(&self, id: &str, level: Option<&str>) -> Result<(), Error> {
        let read = lock(&self.0.reader()).pick_model(id, level);
        self.0.deliver(read.into())
    }

    fn end_reported(self: Box<Self>) {
        self.0.end_reported();
    }
}
