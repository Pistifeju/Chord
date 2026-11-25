use serde_json::Value;

use super::{
    EXTENSION_PREFIX, FORK_DIALOG, MODELS_REQUEST, NAME, Picker, SESSION_COMMANDS, USAGE_STATUS,
};
use crate::harness::calls::OpenCalls;
use crate::harness::session_commands::{self, NO_THINKING};
use crate::harness::shell::read_only_title;
use crate::harness::{
    DialogAsk, HarnessEvent, LineChanges, ModelOption, SlashCommand, SlashKind, SubagentNode,
    SubagentState, ToolOutcome, UsageWindow, capitalized, count, images, text,
};

fn chosen_model(state: &Value) -> Option<HarnessEvent> {
    let model = &state["model"];
    let name = model["name"].as_str().or_else(|| model["id"].as_str())?;
    let thinking = if model["reasoning"] == true {
        state["thinkingLevel"].as_str().map(String::from)
    } else {
        None
    };
    Some(HarnessEvent::Model {
        id: model_id(model),
        name: name.into(),
        thinking,
    })
}

fn model_id(model: &Value) -> Option<String> {
    Some(format!(
        "{}/{}",
        model["provider"].as_str()?,
        model["id"].as_str()?
    ))
}

const THINKING_LEVELS: [&str; 7] = ["off", "minimal", "low", "medium", "high", "xhigh", "max"];

fn thinking_levels(model: &Value) -> Vec<String> {
    if model["reasoning"] != true {
        return vec![];
    }
    let map = &model["thinkingLevelMap"];
    THINKING_LEVELS
        .into_iter()
        .filter(|level| match map.get(level) {
            Some(Value::Null) => false,
            mapped => !matches!(*level, "xhigh" | "max") || mapped.is_some(),
        })
        .map(String::from)
        .collect()
}

pub(super) fn models_event(models: &Value) -> HarnessEvent {
    HarnessEvent::Models {
        harness: NAME,
        models: models
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|model| {
                let id = model_id(model)?;
                Some(ModelOption {
                    name: text(if model["name"].is_string() {
                        &model["name"]
                    } else {
                        &model["id"]
                    }),
                    provider: model["provider"].as_str().map(String::from),
                    levels: thinking_levels(model),
                    default_level: None,
                    id,
                })
            })
            .collect(),
    }
}

fn model_dialog(models: &Value) -> HarnessEvent {
    let options = models
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(model_id)
        .collect();
    picker_dialog(
        Picker::Model,
        options,
        "No model is available. Log in to a provider in pi",
    )
}

pub(super) fn newest_first<T>(options: &mut [T], current: Option<&str>, id: impl Fn(&T) -> &str) {
    fn provider(option: &str) -> &str {
        option.split_once('/').map_or("", |(provider, _)| provider)
    }
    let mut providers: Vec<String> = current
        .map(|model| provider(model).into())
        .into_iter()
        .collect();
    for option in options.iter() {
        if !providers.iter().any(|p| p == provider(id(option))) {
            providers.push(provider(id(option)).into());
        }
    }
    options.sort_by_cached_key(|option| {
        let option = id(option);
        let group = providers.iter().position(|p| *p == provider(option));
        let version: Vec<u32> = option
            .split(|c: char| !c.is_ascii_digit())
            .filter(|run| (1..=4).contains(&run.len()))
            .filter_map(|run| run.parse().ok())
            .collect();
        (group, std::cmp::Reverse(version))
    });
}

fn thinking_dialog(levels: &Value) -> HarnessEvent {
    let options = levels
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|level| level.as_str().map(capitalized))
        .collect();
    picker_dialog(Picker::Thinking, options, NO_THINKING)
}

fn picker_dialog(picker: Picker, options: Vec<String>, nothing_to_pick: &str) -> HarnessEvent {
    if options.iter().all(|option| option == "Off") {
        return HarnessEvent::Notice {
            text: nothing_to_pick.into(),
        };
    }
    let menu = match picker {
        Picker::Model => session_commands::Picker::Model,
        Picker::Thinking => session_commands::Picker::Thinking,
    };
    session_commands::menu(menu, picker.id(), options, None)
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
        Some("compaction_start") if record["reason"] == "manual" => {
            vec![
                HarnessEvent::State { running: true },
                HarnessEvent::Compacting { active: true },
            ]
        }
        Some("compaction_end") if record["reason"] == "manual" => {
            vec![
                HarnessEvent::Compacting { active: false },
                HarnessEvent::State { running: false },
            ]
        }
        Some("compaction_start") => vec![HarnessEvent::Compacting { active: true }],
        Some("compaction_end") => vec![HarnessEvent::Compacting { active: false }],
        Some("tool_execution_start") => {
            vec![tool_start(
                record["toolCallId"].as_str(),
                record["toolName"].as_str(),
                &record["args"],
            )]
        }
        Some("tool_execution_end") => vec![HarnessEvent::ToolEnd {
            id: record["toolCallId"].as_str().unwrap_or_default().into(),
            outcome: if record["isError"] == true {
                ToolOutcome::Failed
            } else {
                ToolOutcome::Done
            },
            images: images(&record["result"]["content"]),
            ms: record["durationMs"].as_f64().map(|ms| ms.round() as u32),
            changes: edit_changes(record["toolName"].as_str(), &record["result"]["details"]),
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
        Some("response") if record["success"] == false => {
            let notice = HarnessEvent::Notice {
                text: record["error"]
                    .as_str()
                    .unwrap_or("pi refused the command")
                    .into(),
            };
            let ended = (record["command"] == "get_messages").then_some(HarnessEvent::HistoryEnd);
            std::iter::once(notice).chain(ended).collect()
        }
        Some("response") if record["id"] == MODELS_REQUEST => {
            vec![models_event(&record["data"]["models"])]
        }
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
                    .chain(session_commands::listed(&SESSION_COMMANDS))
                    .collect(),
            }]
        }
        Some("get_state") => data["sessionFile"]
            .as_str()
            .map(|file| HarnessEvent::SessionInfo { id: file.into() })
            .into_iter()
            .chain(chosen_model(data))
            .collect(),
        Some("get_messages") => history(&data["messages"]),
        Some("get_session_stats") => vec![HarnessEvent::Context {
            tokens: count(&data["contextUsage"]["tokens"]),
            window: count(&data["contextUsage"]["contextWindow"]),
            estimated: false,
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
            let mut events: Vec<HarnessEvent> = blocks
                .flat_map(|block| saved_block(block, message["timestamp"].as_f64()))
                .collect();
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
            images: images(&message["content"]),
            ms: None,
            changes: edit_changes(message["toolName"].as_str(), &message["details"]),
        }],
        Some("compactionSummary") => vec![HarnessEvent::Compaction {
            summary: message["summary"].as_str().unwrap_or_default().into(),
            tokens_before: count(&message["tokensBefore"]),
        }],
        _ => vec![],
    }
}

fn saved_block(block: &Value, at: Option<f64>) -> Vec<HarnessEvent> {
    match block["type"].as_str() {
        Some("text") => match block["text"].as_str() {
            Some(text) if !text.is_empty() => vec![
                HarnessEvent::MessageChunk { text: text.into() },
                HarnessEvent::MessageEnd { failed: false, at },
            ],
            _ => vec![],
        },
        Some("thinking") => vec![
            HarnessEvent::ThoughtStart,
            HarnessEvent::ThoughtChunk {
                text: block["thinking"].as_str().unwrap_or_default().into(),
            },
        ],
        Some("toolCall") => vec![tool_start(
            block["id"].as_str(),
            block["name"].as_str(),
            &block["arguments"],
        )],
        _ => vec![],
    }
}

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
            detail: None,
            descriptions: vec![],
            multi: false,
        },
    }
}

fn first_line(text: &str) -> String {
    let line = text.lines().find(|line| !line.trim().is_empty());
    line.unwrap_or_default().trim().chars().take(80).collect()
}

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

fn ui_request(record: &Value) -> Option<HarnessEvent> {
    let text = |key: &str| record[key].as_str().unwrap_or_default().to_string();
    let ask = match record["method"].as_str()? {
        "setStatus" if record["statusKey"] == USAGE_STATUS => {
            return Some(HarnessEvent::Usage {
                harness: super::NAME,
                windows: usage_windows(record["statusText"].as_str().unwrap_or("[]")),
            });
        }
        "setStatus" => return None,
        "set_editor_text" => return Some(HarnessEvent::Draft { text: text("text") }),
        "select" => DialogAsk::Select {
            options: record["options"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|option| option.as_str().map(String::from))
                .collect(),
            current: None,
            detail: None,
            descriptions: vec![],
            multi: false,
        },
        "confirm" => {
            let message = text("message");
            let (message, detail) = if is_command(&message) {
                (String::new(), Some(message))
            } else {
                (message, None)
            };
            DialogAsk::Confirm {
                message,
                detail,
                url: None,
            }
        }
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

pub(super) fn subagent_nodes(snapshot: &str) -> Vec<SubagentNode> {
    fn state_of(state: &str) -> SubagentState {
        match state {
            "complete" => SubagentState::Done,
            "failed" | "partial" | "stopped" | "rejected" => SubagentState::Error,
            _ => SubagentState::Running,
        }
    }
    fn row(node: &Value, depth: u32) -> Option<SubagentNode> {
        let id = node["id"].as_str()?;
        let label = node["label"].as_str()?;
        let name = match node["agent"].as_str() {
            Some(agent) if label == id => agent,
            _ => label,
        };
        Some(SubagentNode {
            id: id.into(),
            depth,
            name: name.into(),
            state: state_of(node["state"].as_str()?),
            tool: node["activity"]["currentTool"].as_str().map(String::from),
            started_at: node["startedAt"].as_f64(),
            ended_at: node["endedAt"].as_f64(),
        })
    }
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
        let children = last["children"].as_array().map_or(&[][..], Vec::as_slice);
        let agents: Vec<&str> = children
            .iter()
            .filter_map(|child| child["agent"].as_str().or(child["label"].as_str()))
            .collect();
        let shown = agents[..agents.len().min(3)].join(", ");
        let more = if agents.len() > 3 {
            format!(", +{} more", agents.len() - 3)
        } else {
            String::new()
        };
        let repeats_children = agents.len() > 1 && merged.name == format!("{shown}{more}");
        let child_depth = if repeats_children { depth } else { depth + 1 };
        if !repeats_children {
            nodes.push(merged);
        }
        for child in children {
            walk(child, child_depth, nodes);
        }
    }
    let snapshot: Value = serde_json::from_str(snapshot).unwrap_or_default();
    let mut nodes = vec![];
    for run in snapshot["runs"].as_array().into_iter().flatten() {
        walk(run, 0, &mut nodes);
    }
    nodes
}

pub(super) fn message_update(kind: Option<&str>, delta: Option<&str>) -> Vec<HarnessEvent> {
    match (kind, delta) {
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

fn tool_start(id: Option<&str>, name: Option<&str>, args: &Value) -> HarnessEvent {
    let name = name.unwrap_or("tool");
    let (title, detail) = describe_tool(name, args);
    HarnessEvent::ToolStart {
        id: id.unwrap_or_default().into(),
        title,
        detail,
        command: name == "bash",
        changes: match (name, args["content"].as_str()) {
            ("write", Some(content)) => Some(LineChanges {
                added: content.lines().count() as u32,
                removed: 0,
            }),
            _ => None,
        },
    }
}

fn edit_changes(name: Option<&str>, details: &Value) -> Option<LineChanges> {
    let diff = details["diff"].as_str().filter(|_| name == Some("edit"))?;
    let count = |sign: char| diff.lines().filter(|line| line.starts_with(sign)).count() as u32;
    Some(LineChanges {
        added: count('+'),
        removed: count('-'),
    })
}

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

fn is_command(message: &str) -> bool {
    const TOOLS: [&str; 10] = [
        "cd", "git", "npm", "npx", "rm", "mv", "cp", "cargo", "sh", "bash",
    ];
    ["&&", "||", "; ", " | "]
        .iter()
        .any(|chain| message.contains(chain))
        || message
            .split_once(char::is_whitespace)
            .is_some_and(|(first, _)| TOOLS.contains(&first))
}

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
    let at = message["timestamp"].as_f64();
    match message["role"].as_str() {
        Some("user") => vec![HarnessEvent::UserMessageChunk {
            text: collapse_skill(&text_of(&message["content"])),
            images: images(&message["content"]),
            at,
        }],
        Some("assistant") => {
            let failed = message["stopReason"] == "error";
            let mut events = vec![HarnessEvent::MessageEnd { failed, at }];
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
