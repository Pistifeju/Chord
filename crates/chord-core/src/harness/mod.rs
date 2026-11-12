//! What a harness tells the chat: the events every adapter emits.

pub mod pi;
pub mod sessions;
mod shell;

use serde::Serialize;
use specta::Type;

/// What a harness can do; the adapter returns its list when a session starts, and the UI hides a control whose capability is missing.
#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Streaming replies, a composer, steering and stopping a run.
    Chat,
}

#[derive(Serialize, Type, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToolOutcome {
    Done,
    Failed,
    Stopped,
}

#[derive(Serialize, Type, Debug, Clone, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HarnessEvent {
    /// A user message the agent took in, whether it started the run or steered it.
    UserMessageChunk { text: String },
    /// Text of the reply that is streaming now.
    MessageChunk { text: String },
    /// The agent began a new block of reasoning.
    ThoughtStart,
    /// Text of the reasoning block that is streaming now.
    ThoughtChunk { text: String },
    /// A tool call began; `title` is what it does (Read, Search, List, Run, Edit, Write, a tool's name) and `detail` its path, command or arguments.
    ToolStart {
        id: String,
        title: String,
        detail: String,
    },
    /// A tool call ended; `Stopped` means the run ended or was aborted before it reported.
    ToolEnd { id: String, outcome: ToolOutcome },
    /// The streaming reply is complete.
    MessageEnd,
    /// The agent started or finished work.
    State { running: bool },
    /// Something the user should read: a refused command or a failed reply.
    Notice { text: String },
    /// The harness process ended.
    Exited,
}
