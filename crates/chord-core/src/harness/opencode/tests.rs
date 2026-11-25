//! Tests for the OpenCode adapter: hand-written records fed to the reader, and the lines written to OpenCode.

use std::path::Path;

use serde_json::{Value, json};

use super::reader::Reader;
use crate::harness::relaunch::Reader as _;
use crate::harness::subagents::Subagents;
use crate::harness::{
    DialogAnswer, DialogAsk, HarnessEvent, LineChanges, ModelOption, SubagentState, ToolOutcome,
};

// A reader past the handshake, on session "s1".
fn started() -> Reader {
    let mut reader = Reader::new(Path::new("/w"), None, None, None);
    reader.initialize();
    reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    reader.feed(br#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1"}}"#);
    // Request 3 opens the probe session, which these tests leave unanswered.
    reader
}

fn update(update: Value) -> String {
    json!({ "jsonrpc": "2.0", "method": "session/update", "params": { "sessionId": "s1", "update": update } }).to_string()
}

fn events(reader: &mut Reader, records: &[String]) -> Vec<HarnessEvent> {
    records
        .iter()
        .flat_map(|record| reader.feed(record.as_bytes()).events)
        .collect()
}

#[test]
fn a_new_session_names_its_model_and_ends_an_empty_history() {
    let mut reader = Reader::new(Path::new("/w"), None, None, None);
    reader.initialize();
    reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    let answer = r#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1","configOptions":[{"id":"model","category":"model","currentValue":"opencode/big-pickle","options":[{"value":"opencode/big-pickle","name":"opencode/Big Pickle"}]}]}}"#;
    let mut events = reader.feed(answer.as_bytes()).events;
    events.retain(|event| !matches!(event, HarnessEvent::Commands { .. }));
    assert_eq!(
        events,
        [
            HarnessEvent::SessionInfo { id: "s1".into() },
            HarnessEvent::Models {
                harness: "opencode",
                models: vec![ModelOption {
                    id: "opencode/big-pickle".into(),
                    name: "Big Pickle".into(),
                    provider: Some("opencode".into()),
                    levels: vec![],
                    default_level: None,
                }],
            },
            HarnessEvent::Model {
                id: Some("opencode/big-pickle".into()),
                name: "Big Pickle".into(),
                thinking: None
            },
            HarnessEvent::HistoryStart,
            HarnessEvent::HistoryEnd,
        ]
    );
}

#[test]
fn the_commands_list_compact_once_and_opencodes_own() {
    let mut reader = started();
    let commands = update(json!({
        "sessionUpdate": "available_commands_update",
        "availableCommands": [
            { "name": "init", "description": "Write AGENTS.md" },
            { "name": "compact", "description": "Compact the session" },
        ],
    }));
    let Some(HarnessEvent::Commands { commands }) = events(&mut reader, &[commands]).pop() else {
        panic!("no commands");
    };
    let count = |name: &str| {
        commands
            .iter()
            .filter(|command| command.name == name)
            .count()
    };
    assert_eq!((count("compact"), count("init")), (1, 1));
}

// Two models, `openai/luna` in effect at effort `default`.
fn options(current: &str) -> Value {
    let mut options = vec![json!({
        "id": "model", "category": "model", "currentValue": current,
        "options": [
            { "value": "openai/luna", "name": "openai/Luna" },
            { "value": "opencode/big-pickle", "name": "opencode/Big Pickle" },
        ],
    })];
    if current == "openai/luna" {
        options.push(json!({
            "id": "effort", "category": "thought_level", "currentValue": "default",
            "options": [{ "value": "low", "name": "Low" }, { "value": "default", "name": "Default" }],
        }));
    }
    json!(options)
}

#[test]
fn a_model_pick_with_a_level_sets_the_level_once_the_model_answers_and_keeps_its_levels() {
    let mut reader = started();
    let opened = update(
        json!({ "sessionUpdate": "config_option_update", "configOptions": options("opencode/big-pickle") }),
    );
    events(&mut reader, &[opened]);
    let picked = reader.pick_model("openai/luna", Some("low"));
    assert_eq!(
        picked.lines,
        [
            json!({"jsonrpc":"2.0","id":4,"method":"session/set_config_option","params":{"sessionId":"s1","configId":"model","value":"openai/luna"}})
        ]
    );
    let answer =
        json!({ "jsonrpc": "2.0", "id": 4, "result": { "configOptions": options("openai/luna") } });
    let read = reader.feed(answer.to_string().as_bytes());
    assert_eq!(
        read.lines,
        [
            json!({"jsonrpc":"2.0","id":5,"method":"session/set_config_option","params":{"sessionId":"s1","configId":"effort","value":"low"}})
        ]
    );
    // Back on a model without levels, Luna's stay in the list.
    let back = update(
        json!({ "sessionUpdate": "config_option_update", "configOptions": options("opencode/big-pickle") }),
    );
    let Some(HarnessEvent::Models { models, .. }) = events(&mut reader, &[back]).first().cloned()
    else {
        panic!("no models");
    };
    assert_eq!(models[0].levels, ["low", "default"]);
}

#[test]
fn a_probe_session_reads_each_models_levels_then_goes() {
    let mut reader = started();
    let opened = update(
        json!({ "sessionUpdate": "config_option_update", "configOptions": options("opencode/big-pickle") }),
    );
    events(&mut reader, &[opened]);
    let pick = |id: u64, model: &str| json!({"jsonrpc":"2.0","id":id,"method":"session/set_config_option","params":{"sessionId":"p1","configId":"model","value":model}});
    let answer = |id: u64, model: &str| {
        json!({ "jsonrpc": "2.0", "id": id, "result": { "configOptions": options(model) } })
            .to_string()
    };
    let probe = reader.feed(br#"{"jsonrpc":"2.0","id":3,"result":{"sessionId":"p1"}}"#);
    assert_eq!(probe.lines, [pick(4, "opencode/big-pickle")]);
    let next = reader.feed(answer(4, "opencode/big-pickle").as_bytes());
    assert_eq!(
        (next.events, next.lines),
        (vec![], vec![pick(5, "openai/luna")])
    );
    let read = reader.feed(answer(5, "openai/luna").as_bytes());
    let [HarnessEvent::Models { models, .. }] = read.events.as_slice() else {
        panic!("no models");
    };
    assert_eq!(models[0].levels, ["low", "default"]);
    assert_eq!(
        read.lines,
        [json!({"jsonrpc":"2.0","id":6,"method":"session/delete","params":{"sessionId":"p1"}})]
    );
}

#[test]
fn compact_shows_no_user_message() {
    let mut reader = started();
    let sent = reader.send("/compact", &[], &[]).unwrap();
    assert_eq!(sent.events, [HarnessEvent::State { running: true }]);
    assert_eq!(sent.lines[0]["params"]["prompt"][0]["text"], "/compact");
}

#[test]
fn clone_and_compact_refuse_mid_turn_and_new_relaunches() {
    let mut reader = started();
    reader.send("hi", &[], &[]).unwrap();
    assert!(reader.send("/clone", &[], &[]).is_err());
    assert!(reader.send("/compact", &[], &[]).is_err());
    assert!(reader.send("/resume", &[], &[]).is_err());
    assert_eq!(reader.send("/new", &[], &[]).unwrap().relaunch, Some(None));
    assert_eq!(
        reader.send("/reload", &[], &[]).unwrap().relaunch,
        Some(Some("s1".into()))
    );
}

#[test]
fn a_turn_streams_its_thought_and_reply_and_settles_at_the_answer() {
    let mut reader = started();
    let sent = reader.send("hi", &[], &[]).unwrap();
    assert_eq!(
        sent.events,
        [
            HarnessEvent::UserMessageChunk {
                text: "hi".into(),
                images: vec![],
                at: None
            },
            HarnessEvent::State { running: true },
        ]
    );
    assert_eq!(
        sent.lines,
        [
            json!({"jsonrpc":"2.0","id":4,"method":"session/prompt","params":{"sessionId":"s1","prompt":[{"type":"text","text":"hi"}]}})
        ]
    );
    let records = [
        update(
            json!({"sessionUpdate":"agent_thought_chunk","messageId":"m1:reasoning:0","content":{"type":"text","text":"Hm"}}),
        ),
        update(
            json!({"sessionUpdate":"agent_message_chunk","messageId":"m2","content":{"type":"text","text":"Hel"}}),
        ),
        update(
            json!({"sessionUpdate":"agent_message_chunk","messageId":"m2","content":{"type":"text","text":"lo"}}),
        ),
        update(json!({"sessionUpdate":"usage_update","used":120,"size":2000})),
        r#"{"jsonrpc":"2.0","id":4,"result":{"stopReason":"end_turn"}}"#.into(),
    ];
    assert_eq!(
        events(&mut reader, &records),
        [
            HarnessEvent::ThoughtStart,
            HarnessEvent::ThoughtChunk { text: "Hm".into() },
            HarnessEvent::MessageChunk { text: "Hel".into() },
            HarnessEvent::MessageChunk { text: "lo".into() },
            HarnessEvent::MessageEnd {
                failed: false,
                at: None
            },
            HarnessEvent::Context {
                tokens: Some(120),
                window: Some(2000),
                estimated: false
            },
            HarnessEvent::State { running: false },
        ]
    );
}

#[test]
fn a_call_starts_once_at_its_input_and_a_write_counts_its_lines() {
    let mut reader = started();
    reader.send("write it", &[], &[]).unwrap();
    let progress = update(
        json!({"sessionUpdate":"tool_call_update","toolCallId":"c1","status":"in_progress","locations":[{"path":"/w/src/a.txt"}],"rawInput":{"path":"src/a.txt","content":"one\ntwo\n"}}),
    );
    let records = [
        update(
            json!({"sessionUpdate":"tool_call","toolCallId":"c1","name":"write","kind":"edit","status":"pending","rawInput":{}}),
        ),
        progress.clone(),
        progress,
        update(
            json!({"sessionUpdate":"tool_call_update","toolCallId":"c1","status":"completed","rawOutput":{"metadata":{}}}),
        ),
    ];
    assert_eq!(
        events(&mut reader, &records),
        [
            HarnessEvent::ToolStart {
                id: "c1".into(),
                title: "Write".into(),
                detail: "src/a.txt".into(),
                command: false,
                changes: Some(LineChanges {
                    added: 2,
                    removed: 0
                }),
            },
            HarnessEvent::ToolEnd {
                id: "c1".into(),
                outcome: ToolOutcome::Done,
                images: vec![],
                ms: None,
                changes: None,
            },
        ]
    );
}

#[test]
fn a_cancelled_turn_stops_its_open_calls_once_and_aborts() {
    let mut reader = started();
    reader.send("run it", &[], &[]).unwrap();
    let records = [
        update(
            json!({"sessionUpdate":"tool_call","toolCallId":"c1","name":"shell","kind":"execute","status":"in_progress","rawInput":{"command":"sleep 20"}}),
        ),
        update(
            json!({"sessionUpdate":"tool_call_update","toolCallId":"c1","status":"failed","rawOutput":{"metadata":{"interrupted":true}}}),
        ),
        update(
            json!({"sessionUpdate":"tool_call","toolCallId":"c2","name":"read","kind":"read","status":"in_progress","locations":[{"path":"/w/a.txt"}]}),
        ),
        r#"{"jsonrpc":"2.0","id":4,"result":{"stopReason":"cancelled"}}"#.into(),
        update(json!({"sessionUpdate":"tool_call_update","toolCallId":"c2","status":"completed"})),
    ];
    assert_eq!(
        reader.interrupt().lines,
        [json!({"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"s1"}})]
    );
    assert_eq!(
        events(&mut reader, &records),
        [
            HarnessEvent::ToolStart {
                id: "c1".into(),
                title: "Run".into(),
                detail: "sleep 20".into(),
                command: true,
                changes: None,
            },
            HarnessEvent::ToolEnd {
                id: "c1".into(),
                outcome: ToolOutcome::Stopped,
                images: vec![],
                ms: None,
                changes: None,
            },
            HarnessEvent::ToolStart {
                id: "c2".into(),
                title: "Read".into(),
                detail: "a.txt".into(),
                command: false,
                changes: None,
            },
            HarnessEvent::ToolEnd {
                id: "c2".into(),
                outcome: ToolOutcome::Stopped,
                images: vec![],
                ms: None,
                changes: None,
            },
            HarnessEvent::State { running: false },
            HarnessEvent::Aborted,
        ]
    );
    assert_eq!(reader.interrupt().lines, Vec::<Value>::new());
}

// Steers 5 and 6 come back after the turn ended, before prompt 4's answer.
fn two_late_steers() -> Reader {
    let mut reader = started();
    reader.send("first", &[], &[]).unwrap();
    for (id, text) in [(5, "more"), (6, "and more")] {
        let steer = reader.send(text, &[], &[]).unwrap().lines;
        assert_eq!(
            steer,
            [
                json!({"jsonrpc":"2.0","id":id,"method":"_session/steering","params":{"sessionId":"s1","prompt":[{"type":"text","text":text}]}})
            ]
        );
    }
    for id in [5, 6] {
        let refused = json!({"jsonrpc":"2.0","id":id,"result":{"outcome":"promptRequired","reason":"noRunningTurn"}});
        reader.feed(refused.to_string().as_bytes());
    }
    reader
}

#[test]
fn steers_the_ended_turn_refused_go_out_as_the_next_prompt_and_its_steer() {
    let mut reader = two_late_steers();
    let answered = reader.feed(br#"{"jsonrpc":"2.0","id":4,"result":{"stopReason":"end_turn"}}"#);
    assert_eq!(
        answered.lines,
        [
            json!({"jsonrpc":"2.0","id":7,"method":"session/prompt","params":{"sessionId":"s1","prompt":[{"type":"text","text":"more"}]}}),
            json!({"jsonrpc":"2.0","id":8,"method":"_session/steering","params":{"sessionId":"s1","prompt":[{"type":"text","text":"and more"}]}}),
        ]
    );
}

#[test]
fn a_stop_drops_the_steers_the_ended_turn_refused() {
    let mut reader = two_late_steers();
    reader.interrupt();
    let answered = reader.feed(br#"{"jsonrpc":"2.0","id":4,"result":{"stopReason":"cancelled"}}"#);
    assert_eq!(answered.lines, Vec::<Value>::new());
}

#[test]
fn a_failed_prompt_fails_its_reply_and_says_why() {
    let mut reader = started();
    reader.send("hi", &[], &[]).unwrap();
    let records = [
        update(
            json!({"sessionUpdate":"agent_message_chunk","messageId":"m1","content":{"type":"text","text":"Hi"}}),
        ),
        r#"{"jsonrpc":"2.0","id":4,"error":{"code":-32000,"message":"Authentication required"}}"#
            .into(),
    ];
    assert_eq!(
        events(&mut reader, &records),
        [
            HarnessEvent::MessageChunk { text: "Hi".into() },
            HarnessEvent::MessageEnd {
                failed: true,
                at: None
            },
            HarnessEvent::Notice {
                text: "Authentication required. Sign in with: opencode auth login".into()
            },
            HarnessEvent::State { running: false },
        ]
    );
}

#[test]
fn an_update_of_another_session_gives_nothing() {
    let mut reader = started();
    let record = r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s2","update":{"sessionUpdate":"agent_message_chunk","messageId":"m1","content":{"type":"text","text":"Hi"}}}}"#;
    assert_eq!(reader.feed(record.as_bytes()).events, []);
}

#[test]
fn a_request_chord_does_not_handle_is_refused() {
    let mut reader = started();
    let read = reader.feed(br#"{"jsonrpc":"2.0","id":0,"method":"fs/read_text_file","params":{}}"#);
    assert_eq!(
        read.lines,
        [
            json!({"jsonrpc":"2.0","id":0,"error":{"code":-32601,"message":"Chord does not handle fs/read_text_file"}})
        ]
    );
}

const PERMISSION: &str = r#"{"jsonrpc":"2.0","id":7,"method":"session/request_permission","params":{"sessionId":"s1","toolCall":{"toolCallId":"c1","kind":"execute","rawInput":{"command":"ls"}},"options":[{"optionId":"once","kind":"allow_once","name":"Allow once"},{"optionId":"always","kind":"allow_always","name":"Always allow"},{"optionId":"reject","kind":"reject_once","name":"Reject"}]}}"#;

#[test]
fn a_permission_request_opens_a_select_and_esc_picks_its_reject() {
    let mut reader = started();
    reader.send("run it", &[], &[]).unwrap();
    assert_eq!(
        reader.feed(PERMISSION.as_bytes()).events,
        [HarnessEvent::Dialog {
            id: "1".into(),
            title: "Run This Command?".into(),
            ask: DialogAsk::Select {
                options: vec!["Allow once".into(), "Always allow".into(), "Reject".into()],
                current: None,
                detail: Some("ls".into()),
                descriptions: vec![],
                multi: false,
            },
        }]
    );
    let answered = reader.answer("1", &DialogAnswer::Cancelled).unwrap();
    assert_eq!(answered.events, [HarnessEvent::DialogClosed]);
    assert_eq!(
        answered.lines,
        [
            json!({"jsonrpc":"2.0","id":7,"result":{"outcome":{"outcome":"selected","optionId":"reject"}}})
        ]
    );
}

#[test]
fn a_stop_answers_an_open_permission_request_cancelled() {
    let mut reader = started();
    reader.send("run it", &[], &[]).unwrap();
    reader.feed(PERMISSION.as_bytes());
    let stopped = reader.interrupt();
    assert_eq!(stopped.events, [HarnessEvent::DialogClosed]);
    assert_eq!(
        stopped.lines,
        [
            json!({"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"s1"}}),
            json!({"jsonrpc":"2.0","id":7,"result":{"outcome":{"outcome":"cancelled"}}}),
        ]
    );
}

#[test]
fn a_question_offers_other_and_sends_the_typed_answer_as_its_custom_field() {
    let mut reader = started();
    reader.send("ask me", &[], &[]).unwrap();
    let question = r#"{"jsonrpc":"2.0","id":8,"method":"elicitation/create","params":{"sessionId":"s1","mode":"form","message":"","requestedSchema":{"type":"object","properties":{"q0":{"type":"string","title":"Color","description":"Which color?","oneOf":[{"const":"red","title":"Red"},{"const":"blue","title":"Blue"}]},"q0_custom":{"type":"string"}},"required":[]}}}"#;
    assert_eq!(
        reader.feed(question.as_bytes()).events,
        [HarnessEvent::Dialog {
            id: "1".into(),
            title: "Color".into(),
            ask: DialogAsk::Select {
                options: vec!["Red".into(), "Blue".into(), "Other".into()],
                current: None,
                detail: Some("Which color?".into()),
                descriptions: vec![],
                multi: false,
            },
        }]
    );
    let other = DialogAnswer::Value {
        value: "Other".into(),
    };
    assert!(reader.answer("1", &other).unwrap().lines.is_empty());
    let typed = DialogAnswer::Value {
        value: "green".into(),
    };
    assert_eq!(
        reader.answer("2", &typed).unwrap().lines,
        [
            json!({"jsonrpc":"2.0","id":8,"result":{"action":"accept","content":{"q0_custom":"green"}}})
        ]
    );
}

#[test]
fn a_saved_session_replays_inside_its_history_and_a_refused_one_opens_a_new_session() {
    let mut reader = Reader::new(Path::new("/w"), Some("s1".into()), None, None);
    reader.initialize();
    let load = reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    assert_eq!(load.events, [HarnessEvent::HistoryStart]);
    assert_eq!(load.lines[0]["method"], "session/load");
    let records = [
        update(
            json!({"sessionUpdate":"user_message_chunk","messageId":"u1","content":{"type":"text","text":"hi"}}),
        ),
        update(
            json!({"sessionUpdate":"agent_message_chunk","messageId":"m1","content":{"type":"text","text":"Hello"}}),
        ),
        r#"{"jsonrpc":"2.0","id":2,"result":{}}"#.into(),
    ];
    let mut replay = events(&mut reader, &records);
    replay.retain(|event| !matches!(event, HarnessEvent::Commands { .. }));
    assert_eq!(
        replay,
        [
            HarnessEvent::UserMessageChunk {
                text: "hi".into(),
                images: vec![],
                at: None
            },
            HarnessEvent::MessageChunk {
                text: "Hello".into()
            },
            HarnessEvent::MessageEnd {
                failed: false,
                at: None
            },
            HarnessEvent::SessionInfo { id: "s1".into() },
            HarnessEvent::HistoryEnd,
        ]
    );

    let mut gone = Reader::new(Path::new("/w"), Some("s0".into()), None, None);
    gone.initialize();
    gone.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    let refused = gone
        .feed(br#"{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"session not found"}}"#);
    assert_eq!(refused.lines[0]["method"], "session/new");
}

#[test]
fn a_subagent_shows_its_running_tool_and_ends_done() {
    let mut reader = started();
    let child = |fields: Value| {
        let mut params = json!({"rootSessionId":"s1","childSessionId":"c1","parentSessionId":"s1","depth":1,"title":"List files"});
        params
            .as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        json!({"jsonrpc":"2.0","method":"opencode/session/child_update","params":params})
            .to_string()
    };
    let created = events(
        &mut reader,
        &[child(json!({"type":"status","status":"created"}))],
    );
    assert_eq!(
        Subagents::lineage(&created),
        [(0, "List files".into(), SubagentState::Running, None)]
    );
    let tool = child(
        json!({"type":"update","update":{"sessionUpdate":"tool_call_update","status":"in_progress","title":"List files: ls -a"}}),
    );
    assert_eq!(
        Subagents::lineage(&events(&mut reader, &[tool])),
        [(
            0,
            "List files".into(),
            SubagentState::Running,
            Some("ls -a".into())
        )]
    );
    let done = events(
        &mut reader,
        &[child(json!({"type":"status","status":"completed"}))],
    );
    assert_eq!(Subagents::lineage(&done)[0].2, SubagentState::Done);
}

#[test]
fn the_first_prompt_of_a_session_carries_the_tools_guidelines_once() {
    let mut reader = Reader::new(
        Path::new("/w"),
        None,
        Some(("chord".into(), vec![])),
        Some("Use build".into()),
    );
    reader.initialize();
    let new = reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    assert_eq!(
        new.lines[0]["params"]["mcpServers"],
        json!([{"name":"chord","command":"chord","args":[],"env":[]}])
    );
    reader.feed(br#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1"}}"#);
    let first = reader.send("hi", &[], &[]).unwrap();
    assert_eq!(
        first.lines[0]["params"]["prompt"][1],
        json!({"type":"text","text":"Use build","annotations":{"audience":["assistant"]}})
    );
    reader.feed(br#"{"jsonrpc":"2.0","id":4,"result":{"stopReason":"end_turn"}}"#);
    let second = reader.send("again", &[], &[]).unwrap();
    assert_eq!(
        second.lines[0]["params"]["prompt"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn a_loading_session_takes_no_message_and_its_first_prompt_carries_no_guidelines() {
    let mut reader = Reader::new(
        Path::new("/w"),
        Some("s1".into()),
        Some(("chord".into(), vec![])),
        Some("Use build".into()),
    );
    reader.initialize();
    reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    assert!(reader.send("hi", &[], &[]).is_err());
    reader.feed(br#"{"jsonrpc":"2.0","id":2,"result":{}}"#);
    let first = reader.send("hi", &[], &[]).unwrap();
    assert_eq!(
        first.lines[0]["params"]["prompt"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn opencodes_own_turn_runs_from_its_first_update_until_the_quiet_and_takes_a_steer() {
    let mut reader = started();
    let reply = update(
        json!({"sessionUpdate":"agent_message_chunk","messageId":"m1","content":{"type":"text","text":"Found 4 files"}}),
    );
    assert_eq!(
        events(&mut reader, &[reply]),
        [
            HarnessEvent::State { running: true },
            HarnessEvent::MessageChunk {
                text: "Found 4 files".into()
            },
        ]
    );
    assert_eq!(
        reader.quiet_after(),
        Some(std::time::Duration::from_secs(20))
    );
    assert_eq!(
        reader.send("thanks", &[], &[]).unwrap().lines[0]["method"],
        "_session/steering"
    );
    assert_eq!(
        reader.quiet(),
        [
            HarnessEvent::MessageEnd {
                failed: false,
                at: None
            },
            HarnessEvent::State { running: false },
        ]
    );
    assert_eq!(reader.quiet_after(), None);
}

#[test]
fn a_model_pick_before_the_session_opens_goes_out_when_it_opens() {
    let mut reader = Reader::new(Path::new("/w"), None, None, None);
    reader.initialize();
    reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    assert!(reader.pick_model("openai/luna", None).lines.is_empty());
    let answer = r#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1","configOptions":[{"id":"model","category":"model","currentValue":"opencode/big-pickle","options":[{"value":"opencode/big-pickle","name":"opencode/Big Pickle"},{"value":"openai/luna","name":"openai/Luna"}]}]}}"#;
    let read = reader.feed(answer.as_bytes());
    assert!(read.lines.contains(&json!({"jsonrpc":"2.0","id":3,"method":"session/set_config_option","params":{"sessionId":"s1","configId":"model","value":"openai/luna"}})));
}
