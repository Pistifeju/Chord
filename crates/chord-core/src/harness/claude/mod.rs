//! The Claude Code adapter: runs `claude -p` with stream-json in and out as a process and hands its stdout to the reader. The only module that knows Claude Code's records.

mod asks;
mod blocks;
mod commands;
mod permissions;
mod reader;
mod requests;
pub(super) mod spend;
#[cfg(test)]
mod tests;
mod transcript;

use std::path::Path;

use serde_json::json;

use super::models::{self, Step, listed};
use super::relaunch::Relaunchable;
use super::{
    Agent, Capability, DialogAnswer, Harness, HarnessEvent, Image, ModelOption, SignIn, agent_tools,
};
use crate::{Error, lock};
use commands::Settings;
use reader::{Open, Reader};

/// The harness name a pane saves.
pub const NAME: &str = "claude";

pub const HARNESS: Harness = Harness {
    name: NAME,
    title: "Claude Code",
    binary: "claude",
    install: "curl -fsSL https://claude.ai/install.sh | bash",
    sign_in: Some(SignIn {
        check: &["auth", "status"],
        step: "claude auth login",
    }),
    upgrade: "claude update",
    oldest_tested: (2, 1, 328),
    starts: true,
};

// `--replay-user-messages` echoes each message once a turn takes it in: the chat's user message, and the moment a steer leaves the queue.
const ARGS: [&str; 10] = [
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--include-partial-messages",
    "--replay-user-messages",
    "--permission-prompt-tool",
    "stdio",
];

/// The request id of the handshake that lists the models.
const MODELS_REQUEST: &str = "chord-models";

/// Claude Code's models, from the handshake of a process that saves no session and gets no message.
pub(crate) fn models() -> Result<Vec<ModelOption>, Error> {
    let mut settings = Settings::default();
    let args = &ARGS[..7];
    models::ask(
        &HARNESS,
        &[args, &["--no-session-persistence"]].concat(),
        &[requests::control(MODELS_REQUEST, "initialize", json!({}))],
        move |record| {
            let response = &record["response"];
            if record["type"] != "control_response" || response["request_id"] != MODELS_REQUEST {
                return Step::Wait;
            }
            let events = settings.initialized(&response["response"]);
            let models = events
                .into_iter()
                .find(|event| matches!(event, HarnessEvent::Models { .. }));
            models.map_or(Step::Wait, listed)
        },
    )
}

/// The pane's Claude Code; a session command that opens another session, or the same one again, swaps its process.
pub struct Session(Relaunchable<Reader>);

impl Session {
    /// Starts Claude Code in `dir` on the saved session `session` when its file exists, else on a new session. With `tools`, the socket Chord's tools listen on, the agent gets the build and run tools.
    pub fn start(
        dir: &Path,
        session: Option<&str>,
        tools: Option<&Path>,
        on_event: impl Fn(HarnessEvent) + Send + 'static,
    ) -> Result<Session, Error> {
        let open = Open {
            session: session.map(String::from),
            ..Open::default()
        };
        let (cwd, tools) = (dir.to_path_buf(), tools.map(Path::to_path_buf));
        let folder = transcript::folder(dir);
        let launch = move |open| launch(&cwd, folder.as_deref(), open, tools.as_deref());
        Relaunchable::start(&HARNESS, dir, open, launch, on_event).map(Session)
    }
}

// A saved session without its file starts a new one: `--resume` on an unknown id prints an error and exits.
fn launch(
    dir: &Path,
    folder: Option<&Path>,
    open: Open,
    tools: Option<&Path>,
) -> (Vec<String>, Reader) {
    let found = |id: &String| folder.is_some_and(|folder| transcript::file(folder, id).is_file());
    let open = match open.session {
        Some(id) if found(&id) => Open {
            session: Some(id),
            ..open
        },
        _ => Open {
            picks: open.picks,
            ..Open::default()
        },
    };
    let mut args: Vec<String> = ARGS.map(String::from).to_vec();
    if let Some(id) = &open.session {
        args.extend(["--resume".into(), id.clone()]);
    }
    if open.fork {
        args.push("--fork-session".into());
    }
    if let Some(at) = &open.at {
        args.extend(["--resume-session-at".into(), at.clone()]);
    }
    let picks = [
        ("--model", &open.picks.model),
        ("--effort", &open.picks.effort),
        ("--permission-mode", &open.picks.mode),
    ];
    for (flag, pick) in picks {
        args.extend(pick.iter().flat_map(|pick| [flag.into(), pick.clone()]));
    }
    // The tools are Chord's own, so they ask no permission, as in pi.
    if let Some((command, server_args)) = tools.and_then(|socket| agent_tools::command(socket, dir))
    {
        let config =
            json!({ "mcpServers": { "chord": { "command": command, "args": server_args } } });
        args.extend([
            "--mcp-config".into(),
            config.to_string(),
            "--allowedTools".into(),
            "mcp__chord__build,mcp__chord__run".into(),
        ]);
    }
    let reader = Reader::new(dir, folder.map(Path::to_path_buf), open);
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

    /// Sends a message, or runs a session command; `/resume` leaves out `held_elsewhere`, the sessions other panes hold.
    fn send(&self, text: &str, images: &[Image], held_elsewhere: &[String]) -> Result<bool, Error> {
        let took = commands::session_command(text).is_none();
        let images = if took { images } else { &[] };
        let read = lock(&self.0.reader()).send(text, images, held_elsewhere)?;
        self.0.deliver(read).map(|()| took)
    }

    fn abort(&self) -> Result<(), Error> {
        let interrupt = lock(&self.0.reader()).interrupt();
        interrupt.map_or(Ok(()), |line| self.0.write(&line))
    }

    fn answer_dialog(&self, id: &str, answer: &DialogAnswer) -> Result<(), Error> {
        let read = lock(&self.0.reader()).answer(id, answer);
        self.0.deliver(read)
    }

    fn pick_model(&self, id: &str, level: Option<&str>) -> Result<(), Error> {
        let read = lock(&self.0.reader()).pick_model(id, level);
        self.0.deliver(read)
    }

    fn end_reported(self: Box<Self>) {
        self.0.end_reported();
    }
}
