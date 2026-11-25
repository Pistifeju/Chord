use chord_core::lock;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::agent_reply::{AgentReply, agent_reply};
use crate::build::BuildEvent;
use crate::issues::BuildIssue;

const START_WAIT: Duration = Duration::from_secs(10);

#[derive(Serialize, Type, Clone, Debug)]
pub struct AgentBuild {
    pub request: u32,
    pub worktree: PathBuf,
    pub run: bool,
}

#[derive(Deserialize)]
struct Call {
    tool: String,
    worktree: PathBuf,
}

#[derive(Clone, Default)]
pub struct AgentTools(Arc<Waiting>);

#[derive(Default)]
struct Waiting {
    calls: Mutex<HashMap<u32, Sender<BuildEvent>>>,
    next: AtomicU32,
}

impl AgentTools {
    pub fn listen(
        &self,
        socket: &Path,
        start: impl Fn(AgentBuild) + Send + Sync + 'static,
    ) -> std::io::Result<()> {
        let _ = std::fs::remove_file(socket);
        let listener = UnixListener::bind(socket)?;
        std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))?;
        let tools = self.clone();
        let start = Arc::new(start);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (tools, start) = (tools.clone(), Arc::clone(&start));
                thread::spawn(move || tools.answer(stream, &*start));
            }
        });
        Ok(())
    }

    pub fn take(&self, request: u32) -> Option<Sender<BuildEvent>> {
        lock(&self.0.calls).remove(&request)
    }

    fn answer(&self, mut stream: UnixStream, start: &dyn Fn(AgentBuild)) {
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() {
            return;
        }
        let reply = match serde_json::from_str::<Call>(&line) {
            Ok(call) => self.build(call, start),
            Err(error) => AgentReply::failed(format!("Chord cannot read the call: {error}")),
        };
        if let Ok(text) = serde_json::to_string(&reply) {
            let _ = writeln!(stream, "{text}");
        }
    }

    fn build(&self, call: Call, start: &dyn Fn(AgentBuild)) -> AgentReply {
        let run = match call.tool.as_str() {
            "build" => false,
            "run" => true,
            other => return AgentReply::failed(format!("Chord has no tool \"{other}\"")),
        };
        let request = self.0.next.fetch_add(1, Ordering::Relaxed);
        let (sender, events) = mpsc::channel();
        lock(&self.0.calls).insert(request, sender);
        start(AgentBuild {
            request,
            worktree: call.worktree,
            run,
        });
        let tools = self.clone();
        thread::spawn(move || {
            thread::sleep(START_WAIT);
            if let Some(sender) = tools.take(request) {
                let _ = sender.send(BuildEvent::Failed {
                    issues: vec![BuildIssue {
                        file: None,
                        line: None,
                        message: "Chord's window did not start the build".into(),
                    }],
                    more: 0,
                });
            }
        });
        agent_reply(events, run)
    }
}
