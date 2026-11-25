//! Tests for the OpenCode adapter: hand-written records fed to the reader, and the lines written to OpenCode.

use std::path::Path;

use serde_json::{Value, json};

use super::reader::Reader;
use crate::harness::relaunch::Reader as _;
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent, LineChanges, ToolOutcome};

// A reader past the handshake, on session "s1".
fn started() -> Reader {
    let mut reader = Reader::new(Path::new("/w"));
    reader.initialize();
    reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    reader.feed(br#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1"}}"#);
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
    let mut reader = Reader::new(Path::new("/w"));
    reader.initialize();
    reader.feed(br#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}"#);
    let answer = r#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"s1","configOptions":[{"id":"model","category":"model","currentValue":"opencode/big-pickle","options":[{"value":"opencode/big-pickle","name":"opencode/Big Pickle"}]}]}}"#;
    assert_eq!(
        reader.feed(answer.as_bytes()).events,
        [
            HarnessEvent::SessionInfo { id: "s1".into() },
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
            json!({"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"s1","prompt":[{"type":"text","text":"hi"}]}})
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
        r#"{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}"#.into(),
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
        r#"{"jsonrpc":"2.0","id":3,"result":{"stopReason":"cancelled"}}"#.into(),
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

// Steers 4 and 5 come back after the turn ended, before prompt 3's answer.
fn two_late_steers() -> Reader {
    let mut reader = started();
    reader.send("first", &[], &[]).unwrap();
    for (id, text) in [(4, "more"), (5, "and more")] {
        let steer = reader.send(text, &[], &[]).unwrap().lines;
        assert_eq!(
            steer,
            [
                json!({"jsonrpc":"2.0","id":id,"method":"_session/steering","params":{"sessionId":"s1","prompt":[{"type":"text","text":text}]}})
            ]
        );
    }
    for id in [4, 5] {
        let refused = json!({"jsonrpc":"2.0","id":id,"result":{"outcome":"promptRequired","reason":"noRunningTurn"}});
        reader.feed(refused.to_string().as_bytes());
    }
    reader
}

#[test]
fn steers_the_ended_turn_refused_go_out_as_the_next_prompt_and_its_steer() {
    let mut reader = two_late_steers();
    let answered = reader.feed(br#"{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}"#);
    assert_eq!(
        answered.lines,
        [
            json!({"jsonrpc":"2.0","id":6,"method":"session/prompt","params":{"sessionId":"s1","prompt":[{"type":"text","text":"more"}]}}),
            json!({"jsonrpc":"2.0","id":7,"method":"_session/steering","params":{"sessionId":"s1","prompt":[{"type":"text","text":"and more"}]}}),
        ]
    );
}

#[test]
fn a_stop_drops_the_steers_the_ended_turn_refused() {
    let mut reader = two_late_steers();
    reader.interrupt();
    let answered = reader.feed(br#"{"jsonrpc":"2.0","id":3,"result":{"stopReason":"cancelled"}}"#);
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
        r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32000,"message":"Authentication required"}}"#
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
