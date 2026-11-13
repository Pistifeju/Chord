//! What pi's records mean: each record the reader parses, turned into harness events.

use serde_json::Value;

use super::{EXTENSION_PREFIX, FORK_DIALOG, Picker, SESSION_COMMANDS, USAGE_STATUS};
use crate::harness::shell::read_only_title;
use crate::harness::{
    DialogAsk, HarnessEvent, Image, SlashCommand, SlashKind, SubagentNode, SubagentState,
    ToolOutcome, UsageWindow,
};

// The model label shows pi's name for the model; the thinking level shows only for a model that reasons.
fn count(value: &Value) -> Option<u32> {
    value.as_u64().and_then(|number| u32::try_from(number).ok())
}

fn chosen_model(state: &Value) -> Option<HarnessEvent> {
    let model = &state["model"];
    let name = model["name"].as_str().or_else(|| model["id"].as_str())?;
    let thinking = if model["reasoning"] == true {
        state["thinkingLevel"].as_str().map(String::from)
    } else {
        None
    };
    Some(HarnessEvent::Model {
        name: name.into(),
        thinking,
    })
}

fn model_dialog(models: &Value) -> HarnessEvent {
    let options = models
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| {
            Some(format!(
                "{}/{}",
                model["provider"].as_str()?,
                model["id"].as_str()?
            ))
        })
        .collect();
    picker_dialog(
        Picker::Model,
        options,
        "No model is available. Log in to a provider in pi",
    )
}

// pi lists only the levels the current model supports, and just "off" for one that does not think.
fn thinking_dialog(levels: &Value) -> HarnessEvent {
    let options = levels
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|level| level.as_str().map(capitalized))
        .collect();
    picker_dialog(Picker::Thinking, options, "The model does not think")
}

// A list with nothing to pick (no option, or only "Off") is a notice instead.
fn picker_dialog(picker: Picker, options: Vec<String>, nothing_to_pick: &str) -> HarnessEvent {
    if options.iter().all(|option| option == "Off") {
        return HarnessEvent::Notice {
            text: nothing_to_pick.into(),
        };
    }
    HarnessEvent::Dialog {
        id: picker.id().into(),
        title: match picker {
            Picker::Model => "Model",
            Picker::Thinking => "Thinking Level",
        }
        .into(),
        ask: DialogAsk::Menu {
            options,
            current: None,
            filter: matches!(picker, Picker::Model),
        },
    }
}

pub(super) fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

// A run that settles or dies leaves calls pi never closed; they end as stopped so none shows as running.
#[derive(Default)]
pub(super) struct OpenCalls(Vec<String>);

impl OpenCalls {
    pub(super) fn track(&mut self, events: Vec<HarnessEvent>) -> Vec<HarnessEvent> {
        let mut out = Vec::new();
        for event in events {
            match &event {
                HarnessEvent::ToolStart { id, .. } => self.0.push(id.clone()),
                HarnessEvent::ToolEnd { id, .. } => self.0.retain(|open| open != id),
                HarnessEvent::State { running: false } => out.extend(self.stop()),
                _ => {}
            }
            out.push(event);
        }
        out
    }

    pub(super) fn stop(&mut self) -> Vec<HarnessEvent> {
        self.0
            .drain(..)
            .map(|id| HarnessEvent::ToolEnd {
                id,
                outcome: ToolOutcome::Stopped,
                images: vec![],
                ms: None,
            })
            .collect()
    }
}

pub(super) fn events_from(record: &Value) -> Vec<HarnessEvent> {
    match record["type"].as_str() {
        Some("agent_start") => vec![HarnessEvent::State { running: true }],
        Some("agent_settled") if record["aborted"] == true => {
            vec![
                HarnessEvent::State { running: false },
                HarnessEvent::Aborted,
            ]
        }
        Some("agent_settled") => vec![HarnessEvent::State { running: false }],
        Some("message_update") => message_update(&record["assistantMessageEvent"]),
        Some("tool_execution_start") => {
            let (title, detail) = describe_tool(
                record["toolName"].as_str().unwrap_or("tool"),
                &record["args"],
            );
            vec![HarnessEvent::ToolStart {
                id: record["toolCallId"].as_str().unwrap_or_default().into(),
                title,
                detail,
            }]
        }
        Some("tool_execution_end") => vec![HarnessEvent::ToolEnd {
            id: record["toolCallId"].as_str().unwrap_or_default().into(),
            outcome: if record["isError"] == true {
                ToolOutcome::Failed
            } else {
                ToolOutcome::Done
            },
            images: images_of(&record["result"]["content"]),
            ms: record["durationMs"].as_f64().map(|ms| ms.round() as u32),
        }],
        Some("message_end") => message_end(&record["message"]),
        Some("queue_update") => vec![HarnessEvent::Queue {
            steering: record["steering"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|text| text.as_str().map(collapse_skill))
                .collect(),
        }],
        Some("extension_ui_request") => ui_request(record).into_iter().collect(),
        // A failure in one of Chord's own commands is the answer to what the user asked for.
        Some("extension_error")
            if record["extensionPath"]
                .as_str()
                .is_some_and(|path| path.starts_with(&format!("command:{EXTENSION_PREFIX}"))) =>
        {
            vec![HarnessEvent::Notice {
                text: record["error"]
                    .as_str()
                    .unwrap_or("The session command failed")
                    .into(),
            }]
        }
        Some("response") if record["success"] == false => vec![HarnessEvent::Notice {
            text: record["error"]
                .as_str()
                .unwrap_or("pi refused the command")
                .into(),
        }],
        Some("response") => response(record["command"].as_str(), &record["data"]),
        _ => vec![],
    }
}

fn response(command: Option<&str>, data: &Value) -> Vec<HarnessEvent> {
    match command {
        Some("get_commands") => {
            vec![HarnessEvent::Commands {
                commands: data["commands"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|command| {
                        Some(SlashCommand {
                            name: command["name"].as_str()?.into(),
                            description: command["description"].as_str().unwrap_or_default().into(),
                            kind: match command["source"].as_str() {
                                Some("skill") => SlashKind::Skill,
                                Some("prompt") => SlashKind::Prompt,
                                _ => SlashKind::Extension,
                            },
                            origin: package_name(&command["sourceInfo"]),
                        })
                    })
                    .filter(|command| !command.name.starts_with(EXTENSION_PREFIX))
                    .chain(SESSION_COMMANDS.map(|(name, description)| SlashCommand {
                        name: name.into(),
                        description: description.into(),
                        kind: SlashKind::Session,
                        origin: None,
                    }))
                    .collect(),
            }]
        }
        Some("get_state") => data["sessionFile"]
            .as_str()
            .map(|file| HarnessEvent::SessionInfo { file: file.into() })
            .into_iter()
            .chain(chosen_model(data))
            .collect(),
        Some("get_messages") => history(&data["messages"]),
        Some("get_session_stats") => vec![HarnessEvent::Context {
            tokens: count(&data["contextUsage"]["tokens"]),
            window: count(&data["contextUsage"]["contextWindow"]),
        }],
        Some("get_available_models") => vec![model_dialog(&data["models"])],
        Some("get_available_thinking_levels") => vec![thinking_dialog(&data["levels"])],
        Some("get_fork_messages") => vec![fork_dialog(&data["messages"])],
        Some("fork") => data["text"]
            .as_str()
            .map(|text| HarnessEvent::Draft {
                text: collapse_skill(text),
            })
            .into_iter()
            .collect(),
        Some("export_html") => data["path"]
            .as_str()
            .map(|path| HarnessEvent::Exported { path: path.into() })
            .into_iter()
            .collect(),
        _ => vec![],
    }
}

// The session's saved messages as the events their live run produced, between the marks that start the transcript over and end the replay.
fn history(messages: &Value) -> Vec<HarnessEvent> {
    let mut events = vec![HarnessEvent::HistoryStart];
    let mut open_calls = OpenCalls::default();
    for message in messages.as_array().into_iter().flatten() {
        events.extend(open_calls.track(saved_message(message)));
    }
    events.extend(open_calls.stop());
    events.push(HarnessEvent::HistoryEnd);
    events
}

fn saved_message(message: &Value) -> Vec<HarnessEvent> {
    match message["role"].as_str() {
        Some("user") => message_end(message),
        Some("assistant") => {
            let blocks = message["content"].as_array().into_iter().flatten();
            let mut events: Vec<HarnessEvent> = blocks.flat_map(saved_block).collect();
            if message["stopReason"] == "aborted" {
                events.push(HarnessEvent::Aborted);
            }
            events
        }
        Some("toolResult") => vec![HarnessEvent::ToolEnd {
            id: message["toolCallId"].as_str().unwrap_or_default().into(),
            outcome: if message["isError"] == true {
                ToolOutcome::Failed
            } else {
                ToolOutcome::Done
            },
            images: images_of(&message["content"]),
            ms: None,
        }],
        _ => vec![],
    }
}

fn saved_block(block: &Value) -> Vec<HarnessEvent> {
    match block["type"].as_str() {
        Some("text") => match block["text"].as_str() {
            Some(text) if !text.is_empty() => vec![
                HarnessEvent::MessageChunk { text: text.into() },
                HarnessEvent::MessageEnd { failed: false },
            ],
            _ => vec![],
        },
        Some("thinking") => vec![
            HarnessEvent::ThoughtStart,
            HarnessEvent::ThoughtChunk {
                text: block["thinking"].as_str().unwrap_or_default().into(),
            },
        ],
        Some("toolCall") => {
            let (title, detail) = describe_tool(
                block["name"].as_str().unwrap_or("tool"),
                &block["arguments"],
            );
            vec![HarnessEvent::ToolStart {
                id: block["id"].as_str().unwrap_or_default().into(),
                title,
                detail,
            }]
        }
        _ => vec![],
    }
}

// One numbered option per user message of the session, oldest first; the number finds the entry again in the answer.
fn fork_dialog(messages: &Value) -> HarnessEvent {
    let messages: Vec<(&str, String)> = messages
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|message| {
            let text = collapse_skill(message["text"].as_str()?);
            Some((message["entryId"].as_str()?, text))
        })
        .collect();
    if messages.is_empty() {
        return HarnessEvent::Notice {
            text: "The session has no message to fork from".into(),
        };
    }
    let entries: Vec<&str> = messages.iter().map(|(entry, _)| *entry).collect();
    HarnessEvent::Dialog {
        id: format!("{FORK_DIALOG}{}", entries.join(",")),
        title: "Fork From".into(),
        ask: DialogAsk::Select {
            options: messages
                .iter()
                .enumerate()
                .map(|(i, (_, text))| format!("{}. {}", i + 1, first_line(text)))
                .collect(),
            current: None,
        },
    }
}

fn first_line(text: &str) -> String {
    let line = text.lines().find(|line| !line.trim().is_empty());
    line.unwrap_or_default().trim().chars().take(80).collect()
}

// The package a command comes from, from a source like `npm:@scope/name@1.2.0` or `https://host/user/name.git`; none for a local file.
fn package_name(source_info: &Value) -> Option<String> {
    if source_info["origin"] != "package" {
        return None;
    }
    let source = source_info["source"].as_str()?;
    let last = source.trim_end_matches('/').rsplit('/').next()?;
    let name = last.strip_suffix(".git").unwrap_or(last);
    let name = name.rsplit(':').next()?;
    let name = match name.rfind('@') {
        Some(at) if at > 0 => &name[..at],
        _ => name,
    };
    (!name.is_empty()).then(|| name.to_string())
}

// Extensions colour their status with the terminal theme; the bar shows plain text.
fn without_ansi(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            plain.push(c);
        } else if chars.next_if_eq(&'[').is_some() {
            while chars.next().is_some_and(|c| !('@'..='~').contains(&c)) {}
        }
    }
    plain
}

// Select, confirm and input wait for an answer; the editor text goes in the composer; the usage status is read; the other status texts go to the composer bar; notify is not.
fn ui_request(record: &Value) -> Option<HarnessEvent> {
    let text = |key: &str| record[key].as_str().unwrap_or_default().to_string();
    let ask = match record["method"].as_str()? {
        "setStatus" if record["statusKey"] == USAGE_STATUS => {
            return Some(HarnessEvent::Usage {
                windows: usage_windows(record["statusText"].as_str().unwrap_or("[]")),
            });
        }
        "setStatus" => {
            return Some(HarnessEvent::StatusText {
                key: text("statusKey"),
                text: record["statusText"].as_str().map(without_ansi),
            });
        }
        "set_editor_text" => return Some(HarnessEvent::Draft { text: text("text") }),
        "select" => DialogAsk::Select {
            options: record["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|option| option.as_str().map(String::from))
                .collect(),
            current: None,
        },
        "confirm" => DialogAsk::Confirm {
            message: text("message"),
        },
        "input" => DialogAsk::Input {
            placeholder: text("placeholder"),
        },
        _ => return None,
    };
    Some(HarnessEvent::Dialog {
        id: text("id"),
        title: text("title"),
        ask,
    })
}

// The extension writes the windows as JSON; text that does not parse shows no usage.
fn usage_windows(text: &str) -> Vec<UsageWindow> {
    let windows: Vec<Value> = serde_json::from_str(text).unwrap_or_default();
    windows
        .iter()
        .filter_map(|window| {
            Some(UsageWindow {
                label: window["label"].as_str()?.into(),
                resets_at: window["resetsAt"].as_f64(),
            })
        })
        .collect()
}

// A snapshot that does not parse holds no runs.
pub(super) fn subagent_nodes(snapshot: &str) -> Vec<SubagentNode> {
    fn state_of(state: &str) -> SubagentState {
        match state {
            "complete" => SubagentState::Done,
            "failed" | "partial" | "stopped" | "rejected" => SubagentState::Error,
            _ => SubagentState::Running,
        }
    }
    fn row(node: &Value, depth: u32) -> Option<SubagentNode> {
        Some(SubagentNode {
            id: node["id"].as_str()?.into(),
            depth,
            name: node["label"].as_str()?.into(),
            state: state_of(node["state"].as_str()?),
            tool: node["activity"]["currentTool"].as_str().map(String::from),
            started_at: node["startedAt"].as_f64(),
            ended_at: node["endedAt"].as_f64(),
        })
    }
    // A run, its step and a nested summary of one agent repeat its name down a chain of single children; they are one row, in one state: running while any of them runs, else failed if any failed, else done.
    fn walk(node: &Value, depth: u32, nodes: &mut Vec<SubagentNode>) {
        let Some(mut merged) = row(node, depth) else {
            return;
        };
        let mut last = node;
        while let [only] = last["children"].as_array().map_or(&[][..], Vec::as_slice) {
            let Some(next) = row(only, depth).filter(|next| next.name == merged.name) else {
                break;
            };
            merged.state = match (next.state, merged.state) {
                (SubagentState::Running, _) | (_, SubagentState::Running) => SubagentState::Running,
                (SubagentState::Error, _) | (_, SubagentState::Error) => SubagentState::Error,
                _ => SubagentState::Done,
            };
            merged.tool = next.tool.or(merged.tool);
            merged.started_at = match (merged.started_at, next.started_at) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            };
            merged.ended_at = next.ended_at.or(merged.ended_at);
            last = only;
        }
        if merged.state == SubagentState::Running {
            merged.ended_at = None;
        }
        nodes.push(merged);
        for child in last["children"].as_array().into_iter().flatten() {
            walk(child, depth + 1, nodes);
        }
    }
    let snapshot: Value = serde_json::from_str(snapshot).unwrap_or_default();
    let mut nodes = vec![];
    for run in snapshot["runs"].as_array().into_iter().flatten() {
        walk(run, 0, &mut nodes);
    }
    nodes
}

pub(super) fn message_update(update: &Value) -> Vec<HarnessEvent> {
    match (update["type"].as_str(), update["delta"].as_str()) {
        (Some("text_delta"), Some(text)) => {
            vec![HarnessEvent::MessageChunk { text: text.into() }]
        }
        (Some("thinking_start"), _) => vec![HarnessEvent::ThoughtStart],
        (Some("thinking_delta"), Some(text)) => {
            vec![HarnessEvent::ThoughtChunk { text: text.into() }]
        }
        _ => vec![],
    }
}

// A shell command that only reads shows as Read, Search or List; an unknown tool shows its name and argument values.
fn describe_tool(name: &str, args: &Value) -> (String, String) {
    let text = |key: &str| args[key].as_str().unwrap_or_default().to_string();
    match name {
        "read" => ("Read".into(), text("path")),
        "grep" | "find" => ("Search".into(), text("pattern")),
        "ls" => ("List".into(), text("path")),
        "edit" => ("Edit".into(), text("path")),
        "write" => ("Write".into(), text("path")),
        "bash" => {
            let command = text("command");
            let title = read_only_title(&command).unwrap_or("Run");
            (title.into(), command)
        }
        _ => (readable_name(name), argument_values(args)),
    }
}

// "mcp__github__list_issues" reads as "Github list issues".
fn readable_name(name: &str) -> String {
    let words = name
        .trim_start_matches("mcp__")
        .split(['_', '-'])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Tool".into(),
    }
}

fn argument_values(args: &Value) -> String {
    let Some(map) = args.as_object() else {
        return String::new();
    };
    map.values()
        .map(|value| match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn message_end(message: &Value) -> Vec<HarnessEvent> {
    match message["role"].as_str() {
        Some("user") => vec![HarnessEvent::UserMessageChunk {
            text: collapse_skill(&text_of(&message["content"])),
            images: images_of(&message["content"]),
        }],
        Some("assistant") => {
            let failed = message["stopReason"] == "error";
            let mut events = vec![HarnessEvent::MessageEnd { failed }];
            if failed {
                events.push(HarnessEvent::Notice {
                    text: message["errorMessage"]
                        .as_str()
                        .unwrap_or("The reply failed")
                        .into(),
                });
            }
            events
        }
        _ => vec![],
    }
}

// pi expands `/skill:name args` into `<skill name="name" location="...">\n...\n</skill>\n\nargs`; the chat shows what the user typed.
pub(super) fn collapse_skill(text: &str) -> String {
    let collapsed = text
        .strip_prefix("<skill name=\"")
        .and_then(|rest| rest.split_once("\" location=\""))
        .and_then(|(name, rest)| {
            let (_, after) = rest.split_once("\">\n")?;
            let (_, args) = after.split_once("\n</skill>")?;
            Some((name, args.trim_start_matches("\n\n")))
        });
    match collapsed {
        Some((name, "")) => format!("/skill:{name}"),
        Some((name, args)) => format!("/skill:{name} {args}"),
        None => text.into(),
    }
}

fn text_of(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn images_of(content: &Value) -> Vec<Image> {
    let Some(blocks) = content.as_array() else {
        return vec![];
    };
    blocks
        .iter()
        .filter(|block| block["type"] == "image")
        .filter_map(|block| {
            Some(Image {
                data: block["data"].as_str()?.into(),
                mime_type: block["mimeType"].as_str()?.into(),
            })
        })
        .collect()
}
