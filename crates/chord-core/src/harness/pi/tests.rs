//! Tests for the pi adapter: records fed to the reader, and the commands written to pi.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::commands::{
    dialog_response, fork_command, model_command, prompt_command, session_command, thinking_command,
};
use super::reader::Reader;
use super::records::{OpenCalls, collapse_skill};
use super::*;
use crate::harness::{
    DialogAnswer, DialogAsk, HarnessEvent, Image, SlashCommand, SlashKind, SubagentNode,
    SubagentState, ToolOutcome, UsageWindow,
};

fn events(record: &str) -> Vec<HarnessEvent> {
    Reader::default().feed(record.as_bytes()).events
}

#[test]
fn a_text_delta_becomes_a_chunk() {
    let record = r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"Hi"}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::MessageChunk { text: "Hi".into() }]
    );
}

#[test]
fn a_thinking_delta_becomes_a_thought() {
    let record = r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","contentIndex":0,"delta":"hm"}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::ThoughtChunk { text: "hm".into() }]
    );
}

#[test]
fn a_read_only_shell_command_is_titled_by_what_it_does() {
    let record = r#"{"type":"tool_execution_start","toolCallId":"c1","toolName":"bash","args":{"command":"grep -rn foo src"}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::ToolStart {
            id: "c1".into(),
            title: "Search".into(),
            detail: "grep -rn foo src".into()
        }]
    );
}

#[test]
fn an_unknown_tool_shows_its_name_and_argument_values() {
    let record = r#"{"type":"tool_execution_start","toolCallId":"c2","toolName":"mcp__github__list_issues","args":{"repo":"a/b"}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::ToolStart {
            id: "c2".into(),
            title: "Github list issues".into(),
            detail: "a/b".into()
        }]
    );
}

#[test]
fn a_settled_run_stops_the_calls_it_left_open() {
    let mut open = OpenCalls::default();
    let started = |id: &str| HarnessEvent::ToolStart {
        id: id.into(),
        title: "Run".into(),
        detail: "sleep 9".into(),
    };
    assert_eq!(open.track(vec![started("a"), started("b")]).len(), 2);
    let ended = HarnessEvent::ToolEnd {
        id: "a".into(),
        outcome: ToolOutcome::Done,
        images: vec![],
        ms: None,
    };
    open.track(vec![ended]);
    assert_eq!(
        open.track(vec![HarnessEvent::State { running: false }]),
        vec![
            HarnessEvent::ToolEnd {
                id: "b".into(),
                outcome: ToolOutcome::Stopped,
                images: vec![],
                ms: None,
            },
            HarnessEvent::State { running: false }
        ]
    );
}

#[test]
fn a_thinking_start_begins_a_thought() {
    let record = r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_start","contentIndex":0}}"#;
    assert_eq!(events(record), vec![HarnessEvent::ThoughtStart]);
}

#[test]
fn a_failed_tool_call_ends_failed() {
    let record = r#"{"type":"tool_execution_end","toolCallId":"c1","toolName":"bash","result":{},"isError":true,"durationMs":55400}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::ToolEnd {
            id: "c1".into(),
            outcome: ToolOutcome::Failed,
            images: vec![],
            ms: Some(55400),
        }]
    );
}

#[test]
fn a_tool_result_keeps_its_images() {
    let record = r#"{"type":"tool_execution_end","toolCallId":"c1","toolName":"read","result":{"content":[{"type":"text","text":"Read image"},{"type":"image","data":"x","mimeType":"image/png"}]},"isError":false}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::ToolEnd {
            id: "c1".into(),
            outcome: ToolOutcome::Done,
            images: vec![Image {
                data: "x".into(),
                mime_type: "image/png".into()
            }],
            ms: None,
        }]
    );
}

#[test]
fn a_user_message_joins_its_text_blocks_and_keeps_its_images() {
    let record = r#"{"type":"message_end","message":{"role":"user","content":[{"type":"text","text":"a"},{"type":"image","data":"x","mimeType":"image/png"},{"type":"text","text":"b"}]}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::UserMessageChunk {
            text: "a\nb".into(),
            images: vec![Image {
                data: "x".into(),
                mime_type: "image/png".into()
            }],
            at: None
        }]
    );
}

#[test]
fn an_expanded_skill_shows_as_the_command_the_user_typed() {
    let record = r#"{"type":"message_end","message":{"role":"user","content":[{"type":"text","text":"<skill name=\"deploy\" location=\"deploy/SKILL.md\">\nReferences are relative to deploy.\n\nSteps\n</skill>\n\nto staging"}]}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::UserMessageChunk {
            text: "/skill:deploy to staging".into(),
            images: vec![],
            at: None
        }]
    );
    assert_eq!(
        collapse_skill("<skill name=\"deploy\" location=\"x\">\nbody\n</skill>"),
        "/skill:deploy"
    );
    assert_eq!(collapse_skill("<skill is a tag"), "<skill is a tag");
}

#[test]
fn the_command_list_becomes_slash_commands() {
    let record = r#"{"type":"response","command":"get_commands","success":true,"data":{"commands":[{"name":"skill:deploy","description":"Ship it","source":"skill"},{"name":"compact","source":"extension"},{"name":"chord-resume","source":"extension"},{"name":"review","source":"prompt","sourceInfo":{"source":"npm:@acme/pi-tools@1.2.0","origin":"package"}}]}}"#;
    let HarnessEvent::Commands { commands } = &events(record)[0] else {
        panic!("no command list");
    };
    let from_pi: Vec<&SlashCommand> = commands
        .iter()
        .filter(|command| command.kind != SlashKind::Session)
        .collect();
    assert_eq!(
        from_pi,
        [
            &SlashCommand {
                name: "skill:deploy".into(),
                description: "Ship it".into(),
                kind: SlashKind::Skill,
                origin: None
            },
            &SlashCommand {
                name: "compact".into(),
                description: "".into(),
                kind: SlashKind::Extension,
                origin: None
            },
            &SlashCommand {
                name: "review".into(),
                description: "".into(),
                kind: SlashKind::Prompt,
                origin: Some("pi-tools".into())
            }
        ]
    );
}

#[test]
fn saved_messages_come_back_as_the_events_of_their_run() {
    let record = r#"{"type":"response","command":"get_messages","success":true,"data":{"messages":[{"role":"user","content":"List it"},{"role":"assistant","content":[{"type":"thinking","thinking":"hm"},{"type":"text","text":"Sure."},{"type":"toolCall","id":"c1","name":"ls","arguments":{"path":"src"}},{"type":"toolCall","id":"c2","name":"read","arguments":{"path":"a.rs"}}],"stopReason":"toolUse"},{"role":"toolResult","toolCallId":"c1","content":[],"isError":true},{"role":"assistant","content":[],"stopReason":"aborted"}]}}"#;
    assert_eq!(
        events(record),
        vec![
            HarnessEvent::HistoryStart,
            HarnessEvent::UserMessageChunk {
                text: "List it".into(),
                images: vec![],
                at: None
            },
            HarnessEvent::ThoughtStart,
            HarnessEvent::ThoughtChunk { text: "hm".into() },
            HarnessEvent::MessageChunk {
                text: "Sure.".into()
            },
            HarnessEvent::MessageEnd {
                failed: false,
                at: None
            },
            HarnessEvent::ToolStart {
                id: "c1".into(),
                title: "List".into(),
                detail: "src".into()
            },
            HarnessEvent::ToolStart {
                id: "c2".into(),
                title: "Read".into(),
                detail: "a.rs".into()
            },
            HarnessEvent::ToolEnd {
                id: "c1".into(),
                outcome: ToolOutcome::Failed,
                images: vec![],
                ms: None,
            },
            HarnessEvent::Aborted,
            HarnessEvent::ToolEnd {
                id: "c2".into(),
                outcome: ToolOutcome::Stopped,
                images: vec![],
                ms: None,
            },
            HarnessEvent::HistoryEnd,
        ]
    );
}

#[test]
fn the_state_tells_the_session_file() {
    let record = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"sessions/a.jsonl","sessionName":"Login fix"}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::SessionInfo {
            file: "sessions/a.jsonl".into()
        }]
    );
}

#[test]
fn a_session_command_runs_instead_of_being_sent() {
    assert_eq!(
        session_command("/new", &[]),
        Some(json!({ "type": "new_session" }))
    );
    assert_eq!(
        session_command("/tree", &[]),
        Some(json!({ "id": "chord-session", "type": "prompt", "message": "/chord-tree" }))
    );
    assert_eq!(session_command("/new feature", &[]), None);
}

#[test]
fn export_passes_the_path_it_was_given() {
    assert_eq!(
        session_command("/export", &[]),
        Some(json!({ "type": "export_html" }))
    );
    assert_eq!(
        session_command("/export  out/login.html", &[]),
        Some(json!({ "type": "export_html", "outputPath": "out/login.html" }))
    );
    assert_eq!(session_command("/exports", &[]), None);
}

#[test]
fn resume_leaves_out_the_sessions_other_panes_hold() {
    assert_eq!(
        session_command("/resume", &[PathBuf::from("sessions/b.jsonl")]),
        Some(
            json!({ "id": "chord-session", "type": "prompt", "message": r#"/chord-resume ["sessions/b.jsonl"]"# })
        )
    );
}

#[test]
fn a_reload_that_finds_the_shown_session_changes_nothing() {
    let mut shown = Reader::default();
    let mut read = |record: &str| shown.feed(record.as_bytes()).events;
    let state_a = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"a.jsonl"}}"#;
    let state_b = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"b.jsonl"}}"#;
    let messages =
        r#"{"type":"response","command":"get_messages","success":true,"data":{"messages":[]}}"#;
    let history = vec![HarnessEvent::HistoryStart, HarnessEvent::HistoryEnd];
    read(state_a);
    assert_eq!(read(messages), history);
    read(state_a);
    assert_eq!(read(messages), vec![]);
    read(state_b);
    assert_eq!(read(messages), history);
}

#[test]
fn a_changed_session_is_read_again_and_a_cancelled_change_is_not() {
    let read = |record: &str| Reader::default().feed(record.as_bytes()).requests;
    let again = vec![
        json!({ "type": "get_state" }),
        json!({ "type": "get_messages" }),
        json!({ "type": "get_session_stats" }),
    ];
    assert_eq!(
        read(r#"{"type":"response","command":"clone","success":true,"data":{"cancelled":false}}"#),
        again
    );
    assert_eq!(
        read(
            r#"{"id":"chord-session","type":"response","command":"prompt","success":true,"data":{"disposition":"handled"}}"#
        ),
        [again.clone(), vec![json!({ "type": "get_commands" })]].concat()
    );
    assert_eq!(
        read(
            r#"{"type":"response","command":"new_session","success":true,"data":{"cancelled":true}}"#
        ),
        Vec::<Value>::new()
    );
    assert_eq!(
        read(
            r#"{"type":"response","command":"prompt","success":true,"data":{"disposition":"started"}}"#
        ),
        Vec::<Value>::new()
    );
}

#[test]
fn the_model_list_becomes_a_dialog_and_the_pick_sets_the_model() {
    let record = r#"{"type":"response","command":"get_available_models","success":true,"data":{"models":[{"id":"sonnet-4","provider":"anthropic"},{"id":"openai/gpt-5","provider":"openrouter"}]}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::Dialog {
            id: "chord-model".into(),
            title: "Model".into(),
            ask: DialogAsk::Menu {
                options: vec![
                    "anthropic/sonnet-4".into(),
                    "openrouter/openai/gpt-5".into()
                ],
                current: None,
                filter: true
            },
        }]
    );
    let picked = DialogAnswer::Value {
        value: "openrouter/openai/gpt-5".into(),
    };
    assert_eq!(
        model_command(&picked),
        Some(json!({ "type": "set_model", "provider": "openrouter", "modelId": "openai/gpt-5" }))
    );
    assert_eq!(model_command(&DialogAnswer::Cancelled), None);
}

#[test]
fn the_thinking_levels_open_for_a_model_that_thinks_and_the_pick_sets_the_level() {
    let thinks = r#"{"type":"response","command":"get_available_thinking_levels","success":true,"data":{"levels":["off","low","high"]}}"#;
    assert_eq!(
        events(thinks),
        vec![HarnessEvent::Dialog {
            id: "chord-thinking".into(),
            title: "Thinking Level".into(),
            ask: DialogAsk::Menu {
                options: ["Off", "Low", "High"].map(String::from).into(),
                current: None,
                filter: false
            },
        }]
    );
    let plain = r#"{"type":"response","command":"get_available_thinking_levels","success":true,"data":{"levels":["off"]}}"#;
    assert_eq!(
        events(plain),
        vec![HarnessEvent::Notice {
            text: "The model does not think".into()
        }]
    );
    let picked = DialogAnswer::Value {
        value: "High".into(),
    };
    assert_eq!(
        thinking_command(&picked),
        Some(json!({ "type": "set_thinking_level", "level": "high" }))
    );
}

#[test]
fn the_state_tells_the_model_and_a_picker_marks_it() {
    let mut shown = Reader::default();
    let state = r#"{"type":"response","command":"get_state","success":true,"data":{"sessionFile":"a.jsonl","thinkingLevel":"medium","model":{"id":"sonnet-4","provider":"anthropic","name":"Sonnet 4","reasoning":true}}}"#;
    assert_eq!(
        shown.feed(state.as_bytes()).events,
        vec![
            HarnessEvent::SessionInfo {
                file: "a.jsonl".into()
            },
            HarnessEvent::Model {
                name: "Sonnet 4".into(),
                thinking: Some("medium".into())
            },
        ]
    );
    let plain = r#"{"type":"response","command":"get_state","success":true,"data":{"thinkingLevel":"off","model":{"id":"small","provider":"local","reasoning":false}}}"#;
    assert_eq!(
        events(plain),
        vec![HarnessEvent::Model {
            name: "small".into(),
            thinking: None
        }]
    );
    shown.feed(state.as_bytes());
    let models = r#"{"type":"response","command":"get_available_models","success":true,"data":{"models":[{"id":"sonnet-4","provider":"anthropic"}]}}"#;
    let levels = r#"{"type":"response","command":"get_available_thinking_levels","success":true,"data":{"levels":["off","medium"]}}"#;
    let mut current = |record: &str| match shown.feed(record.as_bytes()).events.pop() {
        Some(HarnessEvent::Dialog {
            ask: DialogAsk::Menu { current, .. },
            ..
        }) => current,
        other => panic!("not a menu: {other:?}"),
    };
    assert_eq!(current(models), Some("anthropic/sonnet-4".into()));
    assert_eq!(current(levels), Some("Medium".into()));
}

fn node(
    id: &str,
    depth: u32,
    name: &str,
    state: SubagentState,
    tool: Option<&str>,
    started_at: Option<f64>,
    ended_at: Option<f64>,
) -> SubagentNode {
    SubagentNode {
        id: id.into(),
        depth,
        name: name.into(),
        state,
        tool: tool.map(String::from),
        started_at,
        ended_at,
    }
}

#[test]
fn a_subagents_widget_becomes_the_lineage() {
    let widget = r#"{"type":"extension_ui_request","id":"w1","method":"setWidget","widgetKey":"jobs","widgetLines":["PI_SUBAGENT_ASYNC_JSON:{\"runs\":[{\"id\":\"r1\",\"kind\":\"workflow\",\"label\":\"chain\",\"state\":\"running\",\"startedAt\":1000,\"children\":[{\"id\":\"r1:0\",\"kind\":\"step\",\"label\":\"scout\",\"state\":\"complete\",\"startedAt\":1000,\"endedAt\":4000},{\"id\":\"r1:1\",\"kind\":\"step\",\"label\":\"build\",\"state\":\"running\",\"startedAt\":4000,\"activity\":{\"currentTool\":\"bash\"},\"children\":[{\"id\":\"r1:1:0\",\"kind\":\"step\",\"label\":\"lint\",\"state\":\"stopped\"}]}]},{\"id\":\"r2\",\"kind\":\"subagent\",\"label\":\"worker\",\"state\":\"running\",\"startedAt\":2000,\"activity\":{\"currentTool\":\"read\"},\"children\":[{\"id\":\"r2:0\",\"kind\":\"step\",\"label\":\"worker\",\"state\":\"running\",\"activity\":{\"currentTool\":\"read\"}}]},{\"id\":\"r3\",\"kind\":\"subagent\",\"label\":\"worker\",\"state\":\"running\",\"children\":[{\"id\":\"r3:0\",\"kind\":\"step\",\"label\":\"worker\",\"state\":\"running\",\"activity\":{\"currentTool\":\"bash\"}}]},{\"id\":\"r4\",\"kind\":\"subagent\",\"label\":\"worker\",\"state\":\"running\",\"children\":[{\"id\":\"r4:0\",\"kind\":\"step\",\"label\":\"worker\",\"state\":\"failed\"}]}]}"]}"#;
    let mut shown = Reader::default();
    assert_eq!(
        shown.feed(widget.as_bytes()).events,
        vec![HarnessEvent::Subagents {
            nodes: vec![
                node(
                    "r1",
                    0,
                    "chain",
                    SubagentState::Running,
                    None,
                    Some(1000.0),
                    None
                ),
                node(
                    "r1:0",
                    1,
                    "scout",
                    SubagentState::Done,
                    None,
                    Some(1000.0),
                    Some(4000.0)
                ),
                node(
                    "r1:1",
                    1,
                    "build",
                    SubagentState::Running,
                    Some("bash"),
                    Some(4000.0),
                    None
                ),
                node("r1:1:0", 2, "lint", SubagentState::Error, None, None, None),
                node(
                    "r2",
                    0,
                    "worker",
                    SubagentState::Running,
                    Some("read"),
                    Some(2000.0),
                    None
                ),
                node(
                    "r3",
                    0,
                    "worker",
                    SubagentState::Running,
                    Some("bash"),
                    None,
                    None
                ),
                node("r4", 0, "worker", SubagentState::Running, None, None, None),
            ]
        }]
    );
    let other =
        r#"{"type":"extension_ui_request","id":"w2","method":"setWidget","widgetKey":"other"}"#;
    assert_eq!(shown.feed(other.as_bytes()).events, vec![]);
    let cleared =
        r#"{"type":"extension_ui_request","id":"w3","method":"setWidget","widgetKey":"jobs"}"#;
    assert_eq!(
        shown.feed(cleared.as_bytes()).events,
        vec![HarnessEvent::Subagents { nodes: vec![] }]
    );
}

#[test]
fn a_clear_during_an_automatic_compaction_keeps_the_lineage() {
    let mut shown = Reader::default();
    let widget = r#"{"type":"extension_ui_request","id":"w1","method":"setWidget","widgetKey":"jobs","widgetLines":["PI_SUBAGENT_ASYNC_JSON:{\"runs\":[{\"id\":\"r1\",\"kind\":\"subagent\",\"label\":\"worker\",\"state\":\"running\"}]}"]}"#;
    assert_eq!(shown.feed(widget.as_bytes()).events.len(), 1);
    let cleared =
        r#"{"type":"extension_ui_request","id":"w2","method":"setWidget","widgetKey":"jobs"}"#;
    let start = r#"{"type":"compaction_start","reason":"threshold"}"#;
    shown.feed(start.as_bytes());
    assert_eq!(shown.feed(cleared.as_bytes()).events, vec![]);
    let end = r#"{"type":"compaction_end","reason":"threshold","aborted":false,"willRetry":false}"#;
    shown.feed(end.as_bytes());
    assert_eq!(
        shown.feed(cleared.as_bytes()).events,
        vec![HarnessEvent::Subagents { nodes: vec![] }]
    );
}

#[test]
fn one_agent_repeated_down_a_chain_is_one_subagent_row() {
    let widget = r#"{"type":"extension_ui_request","id":"w1","method":"setWidget","widgetKey":"jobs","widgetLines":["PI_SUBAGENT_ASYNC_JSON:{\"runs\":[{\"id\":\"a\",\"kind\":\"subagent\",\"label\":\"worker\",\"state\":\"complete\",\"startedAt\":1000,\"endedAt\":18000,\"children\":[{\"id\":\"a:0\",\"kind\":\"step\",\"label\":\"worker\",\"state\":\"complete\",\"startedAt\":1200,\"children\":[{\"id\":\"a:0:0\",\"kind\":\"subagent\",\"label\":\"worker\",\"state\":\"running\",\"startedAt\":1500,\"activity\":{\"currentTool\":\"bash\"}}]}]},{\"id\":\"b\",\"kind\":\"workflow\",\"label\":\"worker\",\"state\":\"running\",\"startedAt\":2000,\"children\":[{\"id\":\"b:0\",\"kind\":\"step\",\"label\":\"worker\",\"state\":\"running\",\"children\":[{\"id\":\"b:0:0\",\"kind\":\"step\",\"label\":\"scout\",\"state\":\"running\"},{\"id\":\"b:0:1\",\"kind\":\"step\",\"label\":\"lint\",\"state\":\"failed\"}]}]}]}"]}"#;
    let mut shown = Reader::default();
    assert_eq!(
        shown.feed(widget.as_bytes()).events,
        vec![HarnessEvent::Subagents {
            nodes: vec![
                node(
                    "a",
                    0,
                    "worker",
                    SubagentState::Running,
                    Some("bash"),
                    Some(1000.0),
                    None
                ),
                node(
                    "b",
                    0,
                    "worker",
                    SubagentState::Running,
                    None,
                    Some(2000.0),
                    None
                ),
                node(
                    "b:0:0",
                    1,
                    "scout",
                    SubagentState::Running,
                    None,
                    None,
                    None
                ),
                node("b:0:1", 1, "lint", SubagentState::Error, None, None, None),
            ]
        }]
    );
}

#[test]
fn a_usage_status_becomes_the_usage_windows() {
    let status = r#"{"type":"extension_ui_request","id":"u1","method":"setStatus","statusKey":"chord-usage","statusText":"[{\"label\":\"5h 42%\",\"resetsAt\":1700000000000},{\"label\":\"week 10%\",\"resetsAt\":null}]"}"#;
    assert_eq!(
        events(status),
        vec![HarnessEvent::Usage {
            windows: vec![
                UsageWindow {
                    label: "5h 42%".into(),
                    resets_at: Some(1700000000000.0)
                },
                UsageWindow {
                    label: "week 10%".into(),
                    resets_at: None
                }
            ]
        }]
    );
    let cleared = r#"{"type":"extension_ui_request","id":"u2","method":"setStatus","statusKey":"chord-usage"}"#;
    assert_eq!(
        events(cleared),
        vec![HarnessEvent::Usage { windows: vec![] }]
    );
    let other = r#"{"type":"extension_ui_request","id":"u3","method":"setStatus","statusKey":"other","statusText":"x"}"#;
    assert_eq!(
        events(other),
        vec![HarnessEvent::StatusText {
            key: "other".into(),
            text: Some("x".into())
        }]
    );
    let colored = r#"{"type":"extension_ui_request","id":"u5","method":"setStatus","statusKey":"other","statusText":"\u001b[32m●\u001b[39m\u001b[2m Turn 3\u001b[0m"}"#;
    assert_eq!(
        events(colored),
        vec![HarnessEvent::StatusText {
            key: "other".into(),
            text: Some("● Turn 3".into())
        }]
    );
    let other_cleared =
        r#"{"type":"extension_ui_request","id":"u4","method":"setStatus","statusKey":"other"}"#;
    assert_eq!(
        events(other_cleared),
        vec![HarnessEvent::StatusText {
            key: "other".into(),
            text: None
        }]
    );
}

#[test]
fn the_session_stats_tell_the_context_fill() {
    let stats = r#"{"type":"response","command":"get_session_stats","success":true,"data":{"contextUsage":{"tokens":60000,"contextWindow":200000,"percent":30}}}"#;
    assert_eq!(
        events(stats),
        vec![HarnessEvent::Context {
            tokens: Some(60000),
            window: Some(200000)
        }]
    );
    let no_window = r#"{"type":"response","command":"get_session_stats","success":true,"data":{"contextUsage":{"tokens":null,"contextWindow":null}}}"#;
    assert_eq!(
        events(no_window),
        vec![HarnessEvent::Context {
            tokens: None,
            window: None
        }]
    );
    let compacted = r#"{"type":"response","command":"get_session_stats","success":true,"data":{"contextUsage":{"tokens":null,"contextWindow":200000,"percent":null}}}"#;
    assert_eq!(
        events(compacted),
        vec![HarnessEvent::Context {
            tokens: None,
            window: Some(200000)
        }]
    );
}

#[test]
fn a_settled_run_reads_the_stats_again() {
    let settled = r#"{"type":"agent_settled","aborted":false}"#;
    assert_eq!(
        Reader::default().feed(settled.as_bytes()).requests,
        vec![json!({ "type": "get_session_stats" })]
    );
}

#[test]
fn a_pick_reads_the_state_again() {
    let set = r#"{"type":"response","command":"set_model","success":true,"data":{}}"#;
    let refused = r#"{"type":"response","command":"set_thinking_level","success":false}"#;
    assert_eq!(
        Reader::default().feed(set.as_bytes()).requests,
        vec![
            json!({ "type": "get_state" }),
            json!({ "type": "get_session_stats" })
        ]
    );
    assert!(
        Reader::default()
            .feed(refused.as_bytes())
            .requests
            .is_empty()
    );
}

#[test]
fn a_fork_asks_which_message_and_forks_from_the_picked_one() {
    let record = r#"{"type":"response","command":"get_fork_messages","success":true,"data":{"messages":[{"entryId":"a1","text":"Fix the login\nand the logout"},{"entryId":"b2","text":"Fix the login"}]}}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::Dialog {
            id: "chord-fork:a1,b2".into(),
            title: "Fork From".into(),
            ask: DialogAsk::Select {
                options: vec!["1. Fix the login".into(), "2. Fix the login".into()],
                current: None
            },
        }]
    );
    let picked = DialogAnswer::Value {
        value: "2. Fix the login".into(),
    };
    assert_eq!(
        fork_command("a1,b2", &picked),
        Some(json!({ "type": "fork", "entryId": "b2" }))
    );
    assert_eq!(fork_command("a1,b2", &DialogAnswer::Cancelled), None);
}

#[test]
fn a_queue_update_lists_the_waiting_steers() {
    let record = r#"{"type":"queue_update","steering":["a","b"],"followUp":[]}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::Queue {
            steering: vec!["a".into(), "b".into()]
        }]
    );
}

#[test]
fn a_prompt_carries_its_images() {
    let image = Image {
        data: "x".into(),
        mime_type: "image/png".into(),
    };
    assert_eq!(
        prompt_command("look", &[image]),
        json!({ "type": "prompt", "message": "look", "images": [{ "type": "image", "data": "x", "mimeType": "image/png" }], "streamingBehavior": "steer" })
    );
}

#[test]
fn a_failed_reply_ends_and_says_why() {
    let record = r#"{"type":"message_end","message":{"role":"assistant","stopReason":"error","errorMessage":"No API key"}}"#;
    assert_eq!(
        events(record),
        vec![
            HarnessEvent::MessageEnd {
                failed: true,
                at: None
            },
            HarnessEvent::Notice {
                text: "No API key".into()
            }
        ]
    );
}

#[test]
fn a_select_request_opens_a_dialog_and_a_notify_does_not() {
    let dialog = r#"{"type":"extension_ui_request","id":"u1","method":"select","title":"Pick","options":["a"]}"#;
    let notify = r#"{"type":"extension_ui_request","id":"u2","method":"notify","message":"hi"}"#;
    assert_eq!(
        events(dialog),
        vec![HarnessEvent::Dialog {
            id: "u1".into(),
            title: "Pick".into(),
            ask: DialogAsk::Select {
                options: vec!["a".into()],
                current: None
            },
        }]
    );
    assert_eq!(events(notify), vec![]);
}

#[test]
fn an_answer_goes_back_under_the_request_id() {
    assert_eq!(
        dialog_response(
            "u1",
            &DialogAnswer::Value {
                value: "Allow".into()
            }
        ),
        json!({"type": "extension_ui_response", "id": "u1", "value": "Allow"})
    );
    assert_eq!(
        dialog_response("u2", &DialogAnswer::Confirmed { confirmed: false }),
        json!({"type": "extension_ui_response", "id": "u2", "confirmed": false})
    );
    assert_eq!(
        dialog_response("u3", &DialogAnswer::Cancelled),
        json!({"type": "extension_ui_response", "id": "u3", "cancelled": true})
    );
}

#[test]
fn a_refused_command_becomes_a_notice() {
    let record = r#"{"type":"response","command":"prompt","success":false,"error":"busy"}"#;
    assert_eq!(
        events(record),
        vec![HarnessEvent::Notice {
            text: "busy".into()
        }]
    );
}

#[test]
fn the_run_starts_and_settles() {
    assert_eq!(
        events(r#"{"type":"agent_start"}"#),
        vec![HarnessEvent::State { running: true }]
    );
    assert_eq!(
        events(r#"{"type":"agent_settled","aborted":false}"#),
        vec![HarnessEvent::State { running: false }]
    );
    assert_eq!(
        events(r#"{"type":"agent_settled","aborted":true}"#),
        vec![
            HarnessEvent::State { running: false },
            HarnessEvent::Aborted
        ]
    );
}

#[test]
fn a_version_line_parses() {
    assert_eq!(parse_version("1.5.0"), Some((1, 5, 0)));
    assert_eq!(parse_version("welcome back"), None);
}
