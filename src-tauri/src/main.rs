//! Binary entry: runs the app, or with `agent-tools <socket> <worktree>` serves the build and run tools over stdio MCP for Codex and Claude Code.

use chord_core::harness::agent_tools;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let [_, mode, socket, worktree] = args.as_slice()
        && mode == agent_tools::SERVER_MODE
    {
        if let Err(error) = agent_tools::serve(Path::new(socket), Path::new(worktree)) {
            eprintln!("chord agent-tools: {error}");
            std::process::exit(1);
        }
        return;
    }
    chord_lib::run()
}
