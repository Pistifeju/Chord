//! The agents' build and run tools over MCP, for Codex and Claude Code: the command that starts Chord's own binary as a stdio MCP server, and the server, which passes each call to the socket Chord's window takes the tools on.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Stdout, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::lock;

/// The first argument that starts Chord's binary as the server, followed by the tools' socket and the worktree.
pub const SERVER_MODE: &str = "agent-tools";

/// The tools' names, descriptions and guidelines; pi's extension gets the same texts in `CHORD_TOOL_TEXTS`.
pub(super) const TEXTS: &str = include_str!("agent_tools.json");

#[derive(Deserialize)]
struct Texts {
    instructions: Vec<String>,
    tools: Vec<Tool>,
}

#[derive(Deserialize)]
struct Tool {
    name: String,
    description: String,
}

/// What Chord's socket answers a call with.
#[derive(Deserialize)]
struct Reply {
    text: String,
    failed: bool,
}

/// The connections of the calls that run, by their JSON-RPC id.
type Open = Arc<Mutex<HashMap<String, UnixStream>>>;

/// The tools' guidelines in one line, as the server's `instructions` say them.
pub(super) fn instructions() -> String {
    serde_json::from_str::<Texts>(TEXTS)
        .map(|texts| texts.instructions.join(" "))
        .unwrap_or_default()
}

/// The command and arguments of the server for an agent in `worktree`; none when Chord cannot find its own binary, and the agent starts without the tools.
pub(super) fn command(socket: &Path, worktree: &Path) -> Option<(String, Vec<String>)> {
    let binary = std::env::current_exe().ok()?;
    let args =
        [Path::new(SERVER_MODE), socket, worktree].map(|arg| arg.to_string_lossy().into_owned());
    Some((binary.to_string_lossy().into_owned(), args.to_vec()))
}

/// Answers MCP on stdin until it closes, then waits for the calls that run. Each call runs on its own thread, so a running build blocks no other request.
pub fn serve(socket: &Path, worktree: &Path) -> std::io::Result<()> {
    let texts: Texts = serde_json::from_str(TEXTS).map_err(std::io::Error::other)?;
    let stdout = Arc::new(Mutex::new(std::io::stdout()));
    let open = Open::default();
    let mut calls: Vec<JoinHandle<()>> = Vec::new();
    for line in std::io::stdin().lock().lines() {
        let Ok(message) = serde_json::from_str::<Value>(&line?) else {
            continue;
        };
        let method = message["method"].as_str().unwrap_or_default();
        let params = &message["params"];
        let Some(id) = message.get("id").cloned() else {
            if method == "notifications/cancelled" {
                cancel(&open, &params["requestId"]);
            }
            continue;
        };
        if method != "tools/call" {
            send(&stdout, &id, answer(method, params, &texts));
            continue;
        }
        calls.retain(|call| !call.is_finished());
        let tool = params["name"].as_str().unwrap_or_default().to_string();
        let (socket, worktree) = (socket.to_path_buf(), worktree.to_path_buf());
        let (stdout, open) = (Arc::clone(&stdout), Arc::clone(&open));
        calls.push(thread::spawn(move || {
            if let Some(reply) = call(&socket, &tool, &worktree, &open, &id) {
                let result = json!({
                    "content": [{ "type": "text", "text": reply.text }],
                    "isError": reply.failed,
                });
                send(&stdout, &id, Ok(result));
            }
        }));
    }
    // A client that closes stdin after its call still gets the answer.
    for call in calls {
        let _ = call.join();
    }
    Ok(())
}

fn answer(method: &str, params: &Value, texts: &Texts) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "chord", "version": env!("CARGO_PKG_VERSION") },
            "instructions": texts.instructions.join(" "),
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({
            "tools": texts.tools.iter().map(|tool| json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": { "type": "object", "properties": {} },
            })).collect::<Vec<_>>(),
        })),
        other => Err(format!("Chord has no method \"{other}\"")),
    }
}

fn send(stdout: &Mutex<Stdout>, id: &Value, result: Result<Value, String>) {
    let reply = match result {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": error },
        }),
    };
    let mut stdout = lock(stdout);
    let _ = writeln!(stdout, "{reply}");
    let _ = stdout.flush();
}

// One call per connection: one JSON line each way. None when the client cancelled the call, since it wants no answer then.
fn call(socket: &Path, tool: &str, worktree: &Path, open: &Open, id: &Value) -> Option<Reply> {
    let connected =
        UnixStream::connect(socket).and_then(|stream| Ok((stream.try_clone()?, stream)));
    let (kept, mut stream) = match connected {
        Ok(pair) => pair,
        Err(error) => return Some(failed(format!("Chord cannot take the call: {error}"))),
    };
    lock(open).insert(id.to_string(), kept);
    let request = json!({ "tool": tool, "worktree": worktree });
    let mut answer = String::new();
    let read =
        writeln!(stream, "{request}").and_then(|()| BufReader::new(stream).read_line(&mut answer));
    lock(open).remove(&id.to_string())?;
    Some(
        read.ok()
            .and_then(|_| serde_json::from_str(&answer).ok())
            .unwrap_or_else(|| failed("Chord closed the call without an answer".into())),
    )
}

// Drops the call's connection, as pi's extension does on abort; the build goes on in Chord.
fn cancel(open: &Open, id: &Value) {
    if let Some(stream) = lock(open).remove(&id.to_string()) {
        let _ = stream.shutdown(Shutdown::Both);
    }
}

fn failed(text: String) -> Reply {
    Reply { text, failed: true }
}
