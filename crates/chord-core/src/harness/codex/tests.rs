//! Tests for the Codex adapter: hand-written records fed to the reader, and the lines written to Codex.

use std::path::Path;

use serde_json::json;

use super::permissions;
use super::reader::Reader;
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent};

// A reader past the handshake, on thread "t1".
fn started() -> Reader {
    let mut reader = Reader::new(Path::new("/w"), None, Default::default());
    reader.initialize();
    for answer in [
        r#"{"id":1,"result":{}}"#,
        r#"{"id":2,"result":{"data":[]}}"#,
        r#"{"id":3,"result":{"data":[{"id":"m1","displayName":"Model One"}]}}"#,
        r#"{"id":4,"result":{"thread":{"id":"t1"},"model":"m1","reasoningEffort":"high"}}"#,
    ] {
        reader.feed(answer.as_bytes());
    }
    reader
}

fn events(reader: &mut Reader, records: &[&str]) -> Vec<HarnessEvent> {
    records
        .iter()
        .flat_map(|record| reader.feed(record.as_bytes()).events)
        .collect()
}

#[test]
fn a_streamed_turn_runs_says_its_reply_and_settles() {
    let mut reader = started();
    let records = [
        r#"{"method":"turn/started","params":{"threadId":"t1","turn":{"id":"u1"}}}"#,
        r#"{"method":"item/started","params":{"threadId":"t1","item":{"type":"agentMessage","id":"a1","text":""}}}"#,
        r#"{"method":"item/agentMessage/delta","params":{"threadId":"t1","itemId":"a1","delta":"Hi"}}"#,
        r#"{"method":"item/completed","params":{"threadId":"t1","completedAtMs":5,"item":{"type":"agentMessage","id":"a1","text":"Hi"}}}"#,
        r#"{"method":"turn/completed","params":{"threadId":"t1","turn":{"id":"u1","status":"completed"}}}"#,
    ];
    assert_eq!(
        events(&mut reader, &records),
        [
            HarnessEvent::State { running: true },
            HarnessEvent::MessageChunk { text: "Hi".into() },
            HarnessEvent::MessageEnd {
                failed: false,
                at: Some(5.0)
            },
            HarnessEvent::State { running: false },
        ]
    );
}

#[test]
fn a_notification_of_another_thread_gives_nothing() {
    let mut reader = started();
    let record = r#"{"method":"item/agentMessage/delta","params":{"threadId":"t2","itemId":"a1","delta":"Hi"}}"#;
    assert_eq!(events(&mut reader, &[record]), []);
}

#[test]
fn a_steer_the_ended_turn_refused_starts_the_next_turn() {
    let mut reader = started();
    reader.feed(br#"{"method":"turn/started","params":{"threadId":"t1","turn":{"id":"u1"}}}"#);
    let steer = reader
        .send("more", &[], &[])
        .map(|read| read.lines)
        .unwrap_or_default();
    reader.feed(br#"{"method":"turn/completed","params":{"threadId":"t1","turn":{"id":"u1","status":"completed"}}}"#);
    let refused =
        reader.feed(br#"{"id":5,"error":{"code":-32600,"message":"no active turn to steer"}}"#);
    let input = json!([{ "type": "text", "text": "more", "text_elements": [] }]);
    assert_eq!(
        steer,
        [
            json!({ "id": 5, "method": "turn/steer", "params": { "threadId": "t1", "input": input, "expectedTurnId": "u1" } })
        ]
    );
    assert_eq!(
        refused.lines,
        [
            json!({ "id": 6, "method": "turn/start", "params": { "threadId": "t1", "input": input } })
        ]
    );
}

const COMMAND_ASK: &str = r#"{"id":"r1","method":"item/commandExecution/requestApproval","params":{"threadId":"t1","turnId":"u1","itemId":"c1","command":"/bin/zsh -lc 'rm -rf build'","reason":"Remove the build?"}}"#;

#[test]
fn allow_for_this_session_answers_with_its_decision() {
    let mut reader = started();
    let opened = events(&mut reader, &[COMMAND_ASK]);
    assert_eq!(
        opened,
        [HarnessEvent::Dialog {
            id: "1".into(),
            title: "Remove the build?".into(),
            ask: DialogAsk::Select {
                options: vec![
                    "Allow".into(),
                    "Allow for This Session".into(),
                    "Deny".into()
                ],
                current: None,
                detail: Some("rm -rf build".into()),
                descriptions: vec![],
                multi: false,
            },
        }]
    );
    let picked = DialogAnswer::Value {
        value: "Allow for This Session".into(),
    };
    let answered = reader.answer("1", &picked).map(|read| read.lines);
    assert_eq!(
        answered.unwrap_or_default(),
        [json!({ "id": "r1", "result": { "decision": "acceptForSession" } })]
    );
}

#[test]
fn a_second_ask_opens_after_the_first_one_closes() {
    let mut reader = started();
    let second = r#"{"id":"r2","method":"item/tool/requestUserInput","params":{"threadId":"t1","turnId":"u1","itemId":"q1","questions":[{"id":"name","header":"Name","question":"Your name?","isOther":false,"isSecret":false,"options":null}]}}"#;
    events(&mut reader, &[COMMAND_ASK]);
    assert_eq!(events(&mut reader, &[second]), []);
    let answered = reader
        .answer("1", &DialogAnswer::Cancelled)
        .map(|read| read.events)
        .unwrap_or_default();
    assert_eq!(
        answered,
        [
            HarnessEvent::DialogClosed,
            HarnessEvent::Dialog {
                id: "2".into(),
                title: "Your name?".into(),
                ask: DialogAsk::Input {
                    placeholder: "".into()
                },
            }
        ]
    );
}

fn reopened() -> Reader {
    let mut reader = Reader::new(Path::new("/w"), Some("t1"), Default::default());
    reader.initialize();
    reader.feed(br#"{"id":1,"result":{}}"#);
    reader.feed(br#"{"id":2,"result":{"data":[]}}"#);
    reader
}

#[test]
fn a_saved_thread_opens_again_with_its_turns() {
    let mut reader = reopened();
    let models = reader.feed(br#"{"id":3,"result":{"data":[]}}"#);
    assert_eq!(
        models.lines,
        [
            json!({ "id": 4, "method": "thread/resume", "params": { "threadId": "t1", "cwd": "/w" } })
        ]
    );
    let resumed = r#"{"id":4,"result":{"model":"m1","reasoningEffort":null,"approvalPolicy":"on-request","sandbox":{"type":"readOnly","networkAccess":false},"thread":{"id":"t1","turns":[{"id":"u1","items":[{"type":"userMessage","id":"i1","content":[{"type":"text","text":"Hi"}]},{"type":"agentMessage","id":"a1","text":"Hello"}]}]}}}"#;
    assert_eq!(
        events(&mut reader, &[resumed]),
        [
            HarnessEvent::HistoryStart,
            HarnessEvent::SessionInfo { id: "t1".into() },
            HarnessEvent::Model {
                name: "m1".into(),
                thinking: None
            },
            HarnessEvent::Permissions {
                mode: "Read Only".into(),
                asks: true
            },
            HarnessEvent::UserMessageChunk {
                text: "Hi".into(),
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
            HarnessEvent::HistoryEnd,
        ]
    );
}

#[test]
fn a_reload_shows_and_sends_the_users_picks_over_the_threads_settings() {
    let picks = json!({ "model": "m2", "effort": "low" });
    let mut reader = Reader::new(
        Path::new("/w"),
        Some("t1"),
        picks.as_object().cloned().unwrap_or_default(),
    );
    reader.initialize();
    reader.feed(br#"{"id":1,"result":{}}"#);
    reader.feed(br#"{"id":2,"result":{"data":[]}}"#);
    reader.feed(br#"{"id":3,"result":{"data":[]}}"#);
    let resumed = r#"{"id":4,"result":{"model":"m1","reasoningEffort":"high","thread":{"id":"t1","turns":[]}}}"#;
    let opened = events(&mut reader, &[resumed]);
    assert!(opened.contains(&HarnessEvent::Model {
        name: "m2".into(),
        thinking: Some("low".into())
    }));
    let turn = reader.send("Hi", &[], &[]).map(|read| read.lines);
    let input = json!([{ "type": "text", "text": "Hi", "text_elements": [] }]);
    assert_eq!(
        turn.unwrap_or_default(),
        [
            json!({ "id": 5, "method": "turn/start", "params": { "model": "m2", "effort": "low", "threadId": "t1", "input": input } })
        ]
    );
}

#[test]
fn a_saved_thread_codex_cannot_open_gives_a_new_one() {
    let mut reader = reopened();
    reader.feed(br#"{"id":3,"result":{"data":[]}}"#);
    let refused = reader.feed(br#"{"id":4,"error":{"code":-32600,"message":"no rollout"}}"#);
    assert_eq!(
        refused.lines,
        [json!({ "id": 5, "method": "thread/start", "params": { "cwd": "/w" } })]
    );
}

#[test]
fn an_empty_reply_still_ends() {
    let mut reader = started();
    let record = r#"{"method":"item/completed","params":{"threadId":"t1","item":{"type":"agentMessage","id":"a1","text":""}}}"#;
    assert_eq!(
        events(&mut reader, &[record]),
        [HarnessEvent::MessageEnd {
            failed: false,
            at: None
        }]
    );
}

#[test]
fn a_mode_reads_as_its_name_and_says_whether_it_asks() {
    let read_only = json!({ "type": "readOnly", "networkAccess": false });
    let full = json!({ "type": "dangerFullAccess" });
    assert_eq!(
        permissions::mode(&json!("on-request"), &read_only),
        ("Read Only".to_string(), true)
    );
    assert_eq!(
        permissions::mode(&json!("never"), &full),
        ("Full Access".to_string(), false)
    );
}

#[test]
fn a_picked_mode_rides_the_next_turn() {
    let mut reader = started();
    let opened = reader
        .send("/permissions", &[], &[])
        .map(|read| read.events);
    assert!(matches!(
        opened.as_deref(),
        Ok([HarnessEvent::Dialog { id, .. }]) if id == "codex-permissions"
    ));
    let picked = DialogAnswer::Value {
        value: "Full Access".into(),
    };
    let told = reader
        .answer("codex-permissions", &picked)
        .map(|read| read.events);
    assert_eq!(
        told.unwrap_or_default(),
        [
            HarnessEvent::DialogClosed,
            HarnessEvent::Permissions {
                mode: "Full Access".into(),
                asks: false
            }
        ]
    );
    let turn = reader.send("go", &[], &[]).map(|read| read.lines);
    let input = json!([{ "type": "text", "text": "go", "text_elements": [] }]);
    assert_eq!(
        turn.unwrap_or_default(),
        [json!({ "id": 5, "method": "turn/start", "params": {
            "threadId": "t1",
            "input": input,
            "approvalPolicy": "never",
            "sandboxPolicy": { "type": "dangerFullAccess" },
        } })]
    );
}

#[test]
fn resume_lists_the_threads_no_other_pane_holds_and_opens_the_pick() {
    let mut reader = started();
    let held = ["t3".to_string()];
    let list = reader.send("/resume", &[], &held).map(|read| read.lines);
    assert_eq!(
        list.unwrap_or_default(),
        [
            json!({ "id": 5, "method": "thread/list", "params": { "cwd": "/w", "sortKey": "updated_at", "limit": 50 } })
        ]
    );
    let threads = r#"{"id":5,"result":{"data":[{"id":"t1","preview":"This one","updatedAt":0},{"id":"t2","preview":"Fix the login\nand more","updatedAt":0},{"id":"t3","preview":"Held","updatedAt":0}]}}"#;
    let opened = events(&mut reader, &[threads]);
    let [
        HarnessEvent::Dialog {
            id,
            ask: DialogAsk::Select { options, .. },
            ..
        },
    ] = opened.as_slice()
    else {
        panic!("no resume dialog: {opened:?}");
    };
    assert_eq!(options.len(), 1);
    assert!(options[0].starts_with("1. Fix the login · "));
    let picked = DialogAnswer::Value {
        value: options[0].clone(),
    };
    let resumed = reader.answer(id, &picked).map(|read| read.lines);
    assert_eq!(
        resumed.unwrap_or_default(),
        [
            json!({ "id": 6, "method": "thread/resume", "params": { "threadId": "t2", "cwd": "/w" } })
        ]
    );
}

// The select `command` opens on a thread whose turns are "u1" (a message), "c1" (a compaction) and "u2" (a message), and what picking the second message writes.
fn go_back(command: &str) -> (Vec<String>, Vec<HarnessEvent>, Vec<serde_json::Value>) {
    let mut reader = started();
    reader.send(command, &[], &[]).ok();
    let turns = r#"{"id":5,"result":{"data":[{"id":"u2","items":[{"type":"userMessage","content":[{"type":"text","text":"Second"}]}]},{"id":"c1","items":[]},{"id":"u1","items":[{"type":"userMessage","content":[{"type":"text","text":"First"}]}]}]}}"#;
    let opened = events(&mut reader, &[turns]);
    let [
        HarnessEvent::Dialog {
            id,
            ask: DialogAsk::Select { options, .. },
            ..
        },
    ] = opened.as_slice()
    else {
        panic!("no dialog: {opened:?}");
    };
    let picked = DialogAnswer::Value {
        value: options[1].clone(),
    };
    let read = reader.answer(id, &picked).unwrap_or_default();
    (options.clone(), read.events, read.lines)
}

#[test]
fn fork_goes_through_the_turn_before_the_picked_message() {
    let (options, events, lines) = go_back("/fork");
    assert_eq!(options, ["1. First", "2. Second"]);
    assert_eq!(
        events,
        [
            HarnessEvent::DialogClosed,
            HarnessEvent::Draft {
                text: "Second".into()
            }
        ]
    );
    assert_eq!(
        lines,
        [
            json!({ "id": 6, "method": "thread/fork", "params": { "threadId": "t1", "lastTurnId": "c1", "cwd": "/w" } })
        ]
    );
}

#[test]
fn rewind_drops_the_picked_messages_turn_and_opens_the_thread_again() {
    let (_, _, lines) = go_back("/rewind");
    assert_eq!(
        lines,
        [
            json!({ "id": 6, "method": "thread/revert", "params": { "threadId": "t1", "beforeTurnId": "u2" } })
        ]
    );
}

#[test]
fn resume_waits_for_the_run_to_end() {
    let mut reader = started();
    reader.feed(br#"{"method":"turn/started","params":{"threadId":"t1","turn":{"id":"u1"}}}"#);
    assert!(reader.send("/resume", &[], &[]).is_err());
}

#[test]
fn a_form_asks_its_fields_in_the_order_the_server_sent() {
    let mut reader = started();
    let form = r#"{"id":"e1","method":"mcpServer/elicitation/request","params":{"threadId":"t1","turnId":"u1","serverName":"tracker","mode":"form","_meta":null,"message":"Fill in","requestedSchema":{"type":"object","properties":{"zeta":{"type":"string","title":"Zeta"},"alpha":{"type":"boolean","title":"Alpha"}}}}}"#;
    let first = events(&mut reader, &[form]);
    assert!(matches!(first.as_slice(), [HarnessEvent::Dialog { title, .. }] if title == "Zeta"));
    let typed = DialogAnswer::Value { value: "z".into() };
    let second = reader.answer("1", &typed).map(|read| read.events);
    assert_eq!(
        second.unwrap_or_default(),
        [
            HarnessEvent::DialogClosed,
            HarnessEvent::Dialog {
                id: "2".into(),
                title: "Alpha".into(),
                ask: DialogAsk::Confirm {
                    message: "".into(),
                    url: None
                },
            }
        ]
    );
    let confirmed = DialogAnswer::Confirmed { confirmed: true };
    let answered = reader.answer("2", &confirmed).map(|read| read.lines);
    assert_eq!(
        answered.unwrap_or_default(),
        [
            json!({ "id": "e1", "result": { "action": "accept", "content": { "zeta": "z", "alpha": true }, "_meta": null } })
        ]
    );
}

#[test]
fn a_withdrawn_ask_closes_and_the_next_one_opens() {
    let mut reader = started();
    let second = r#"{"id":"r2","method":"item/fileChange/requestApproval","params":{"threadId":"t1","turnId":"u1","itemId":"f1"}}"#;
    events(&mut reader, &[COMMAND_ASK, second]);
    let resolved =
        r#"{"method":"serverRequest/resolved","params":{"threadId":"t1","requestId":"r1"}}"#;
    let after = events(&mut reader, &[resolved]);
    assert!(matches!(
        after.as_slice(),
        [HarnessEvent::DialogClosed, HarnessEvent::Dialog { id, title, .. }] if id == "2" && title == "Apply These Changes?"
    ));
}

#[test]
fn a_turn_end_drops_its_asks_and_keeps_the_messages_questions() {
    let mut reader = started();
    let questions = r#"{"method":"item/completed","params":{"threadId":"t1","item":{"type":"agentMessage","id":"a1","text":"Done","delivery":"async","questions":[{"title":"Ship it?","options":["Yes","No"]}]}}}"#;
    let ended = r#"{"method":"turn/completed","params":{"threadId":"t1","turn":{"id":"u1","status":"completed"}}}"#;
    events(&mut reader, &[questions, COMMAND_ASK]);
    assert_eq!(
        events(&mut reader, &[ended]),
        [HarnessEvent::State { running: false }]
    );
    let picked = DialogAnswer::Value {
        value: "Yes".into(),
    };
    let answered = reader.answer("1", &picked).map(|read| read.lines);
    let input = json!([{ "type": "text", "text": "Ship it?\nYes", "text_elements": [] }]);
    assert_eq!(
        answered.unwrap_or_default(),
        [
            json!({ "id": 5, "method": "turn/start", "params": { "threadId": "t1", "input": input } })
        ]
    );
}

#[test]
fn other_opens_an_input_whose_text_is_the_answer() {
    let mut reader = started();
    let question = r#"{"id":"q1","method":"item/tool/requestUserInput","params":{"threadId":"t1","turnId":"u1","itemId":"i1","isBlocking":true,"autoResolutionMs":null,"questions":[{"id":"pick","header":"Pick","question":"Which one?","isOther":true,"isSecret":false,"options":[{"label":"A","description":"The first"}]}]}}"#;
    assert_eq!(
        events(&mut reader, &[question]),
        [HarnessEvent::Dialog {
            id: "1".into(),
            title: "Which one?".into(),
            ask: DialogAsk::Select {
                options: vec!["A".into(), "Other".into()],
                current: None,
                detail: None,
                descriptions: vec!["The first".into(), "".into()],
                multi: false,
            },
        }]
    );
    let other = DialogAnswer::Value {
        value: "Other".into(),
    };
    let input = reader.answer("1", &other).map(|read| read.events);
    assert_eq!(
        input.unwrap_or_default(),
        [
            HarnessEvent::DialogClosed,
            HarnessEvent::Dialog {
                id: "2".into(),
                title: "Which one?".into(),
                ask: DialogAsk::Input {
                    placeholder: "".into()
                },
            }
        ]
    );
    let typed = DialogAnswer::Value {
        value: "Mine".into(),
    };
    let answered = reader.answer("2", &typed).map(|read| read.lines);
    assert_eq!(
        answered.unwrap_or_default(),
        [json!({ "id": "q1", "result": { "answers": { "pick": { "answers": ["Mine"] } } } })]
    );
}

#[test]
fn a_picked_model_without_the_level_takes_its_default() {
    let mut reader = Reader::new(Path::new("/w"), None, Default::default());
    reader.initialize();
    for answer in [
        r#"{"id":1,"result":{}}"#,
        r#"{"id":2,"result":{"data":[]}}"#,
        r#"{"id":3,"result":{"data":[{"id":"m1","displayName":"Model One","supportedReasoningEfforts":[{"reasoningEffort":"high"}],"defaultReasoningEffort":"high"},{"id":"m2","displayName":"Model Two","supportedReasoningEfforts":[{"reasoningEffort":"low"},{"reasoningEffort":"medium"}],"defaultReasoningEffort":"medium"}]}}"#,
        r#"{"id":4,"result":{"thread":{"id":"t1"},"model":"m1","reasoningEffort":"high"}}"#,
    ] {
        reader.feed(answer.as_bytes());
    }
    let picked = DialogAnswer::Value {
        value: "Model Two".into(),
    };
    let told = reader
        .answer("codex-model", &picked)
        .map(|read| read.events);
    assert_eq!(
        told.unwrap_or_default(),
        [
            HarnessEvent::DialogClosed,
            HarnessEvent::Model {
                name: "Model Two".into(),
                thinking: Some("medium".into())
            }
        ]
    );
    let turn = reader.send("go", &[], &[]).map(|read| read.lines);
    let input = json!([{ "type": "text", "text": "go", "text_elements": [] }]);
    assert_eq!(
        turn.unwrap_or_default(),
        [
            json!({ "id": 5, "method": "turn/start", "params": { "threadId": "t1", "input": input, "model": "m2", "effort": "medium" } })
        ]
    );
}
