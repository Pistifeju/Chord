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
use super::{Agent, Harness, HarnessEvent, ModelOption, SignIn, agent_tools};
use crate::Error;
use commands::Settings;
use reader::{Open, Reader};

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

const MODELS_REQUEST: &str = "chord-models";

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

pub fn start(
    dir: &Path,
    session: Option<&str>,
    tools: Option<&Path>,
    on_event: impl Fn(HarnessEvent) + Send + 'static,
) -> Result<Box<dyn Agent>, Error> {
    let open = Open {
        session: session.map(String::from),
        ..Open::default()
    };
    let (cwd, tools) = (dir.to_path_buf(), tools.map(Path::to_path_buf));
    let folder = transcript::folder(dir);
    let launch = move |open| launch(&cwd, folder.as_deref(), open, tools.as_deref());
    Relaunchable::start(&HARNESS, dir, open, launch, on_event)
        .map(|agent| Box::new(agent) as Box<dyn Agent>)
}

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
