//! Tests for the Claude Code adapter: hand-written records fed to the reader.

use std::path::Path;

use serde_json::json;

use super::reader::{Open, Reader};
use super::transcript;
use crate::harness::earlier::Message;
use crate::harness::subagents::Subagents;
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent, SubagentState, UsageWindow};

fn events(reader: &mut Reader, records: &[&str]) -> Vec<HarnessEvent> {
    records
        .iter()
        .flat_map(|record| reader.feed(record.as_bytes()).events)
        .collect()
}

#[test]
fn a_streamed_turn_runs_says_its_reply_and_settles() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let records = [
        r#"{"type":"system","subtype":"init"}"#,
        r#"{"type":"stream_event","parent_tool_use_id":null,"event":{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}}"#,
        r#"{"type":"stream_event","parent_tool_use_id":null,"event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}}"#,
        r#"{"type":"stream_event","parent_tool_use_id":null,"event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"lo"}}}"#,
        r#"{"type":"assistant","parent_tool_use_id":null,"timestamp":"2026-01-02T03:04:05.678Z","message":{"content":[{"type":"text","text":"Hello"}]}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed"}"#,
    ];
    assert_eq!(
        events(&mut reader, &records),
        [
            HarnessEvent::State { running: true },
            HarnessEvent::MessageChunk { text: "Hel".into() },
            HarnessEvent::MessageChunk { text: "lo".into() },
            HarnessEvent::MessageEnd {
                failed: false,
                at: Some(1767323045678.0)
            },
            HarnessEvent::State { running: false },
        ]
    );
}

#[test]
fn a_reply_that_failed_at_the_api_says_its_text_as_failed() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let record = r#"{"type":"assistant","parent_tool_use_id":null,"error":"authentication_failed","message":{"content":[{"type":"text","text":"Not logged in"}]}}"#;
    assert_eq!(
        events(&mut reader, &[record]),
        [
            HarnessEvent::MessageChunk {
                text: "Not logged in".into()
            },
            HarnessEvent::MessageEnd {
                failed: true,
                at: None
            },
        ]
    );
}

#[test]
fn an_echo_of_a_message_the_adapter_did_not_send_gives_nothing() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let _ = reader.send("Hi", &[], &[]);
    let record = r#"{"type":"user","isReplay":true,"uuid":"00000000-0000-4000-8000-000000000000","parent_tool_use_id":null,"message":{"role":"user","content":"<local-command-stdout>Set model</local-command-stdout>"}}"#;
    assert_eq!(events(&mut reader, &[record]), []);
}

#[test]
fn an_interrupted_turn_settles_as_aborted_without_a_notice() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let records = [
        r#"{"type":"system","subtype":"init"}"#,
        r#"{"type":"result","subtype":"error_during_execution","is_error":true,"terminal_reason":"aborted_tools","errors":["Interrupted"]}"#,
    ];
    assert_eq!(
        events(&mut reader, &records),
        [
            HarnessEvent::State { running: true },
            HarnessEvent::State { running: false },
            HarnessEvent::Aborted,
        ]
    );
}

#[test]
fn an_always_allow_answers_with_its_one_rule() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let ask = r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"touch notes.txt"},"permission_suggestions":[{"type":"addRules","rules":[{"toolName":"Bash","ruleContent":"touch notes.txt"}],"behavior":"allow","destination":"localSettings"},{"type":"addDirectories","directories":["/w"],"destination":"session"}]}}"#;
    let shown = events(&mut reader, &[ask]);
    assert_eq!(
        shown,
        [HarnessEvent::Dialog {
            id: "1".into(),
            title: "Run This Command?".into(),
            ask: DialogAsk::Select {
                options: vec![
                    "Allow".into(),
                    "Always Allow \u{201c}touch notes.txt\u{201d}".into(),
                    "Deny".into()
                ],
                current: None,
                detail: Some("touch notes.txt".into()),
                descriptions: vec![],
                multi: false,
            },
        }]
    );
    let always = DialogAnswer::Value {
        value: "Always Allow \u{201c}touch notes.txt\u{201d}".into(),
    };
    let rule = json!({ "type": "addRules", "rules": [{ "toolName": "Bash", "ruleContent": "touch notes.txt" }], "behavior": "allow", "destination": "localSettings" });
    assert_eq!(
        reader.answer("1", &always).lines,
        [json!({
            "type": "control_response",
            "response": { "subtype": "success", "request_id": "r1", "response": { "behavior": "allow", "updatedPermissions": [rule] } },
        })]
    );
}

#[test]
fn questions_answer_once_after_the_last_under_their_texts() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let ask = r#"{"type":"control_request","request_id":"r1","request":{"subtype":"can_use_tool","tool_name":"AskUserQuestion","input":{"questions":[{"question":"Which color?","header":"Color","options":[{"label":"Red","description":""},{"label":"Blue","description":""}],"multiSelect":false},{"question":"Which sizes?","header":"Size","options":[{"label":"S","description":""},{"label":"M","description":""}],"multiSelect":true}]}}}"#;
    events(&mut reader, &[ask]);
    let first = reader.answer(
        "1",
        &DialogAnswer::Value {
            value: "Blue".into(),
        },
    );
    assert!(first.lines.is_empty());
    let sizes = DialogAnswer::Values {
        values: vec!["S".into(), "M".into()],
    };
    let input = json!({ "questions": [
        { "question": "Which color?", "header": "Color", "options": [{ "label": "Red", "description": "" }, { "label": "Blue", "description": "" }], "multiSelect": false },
        { "question": "Which sizes?", "header": "Size", "options": [{ "label": "S", "description": "" }, { "label": "M", "description": "" }], "multiSelect": true },
    ], "answers": { "Which color?": "Blue", "Which sizes?": "S, M" } });
    assert_eq!(
        reader.answer("2", &sizes).lines,
        [json!({
            "type": "control_response",
            "response": { "subtype": "success", "request_id": "r1", "response": { "behavior": "allow", "updatedInput": input } },
        })]
    );
}

#[test]
fn a_transcript_gives_the_last_lines_chain_alone() {
    let file = [
        r#"{"type":"user","uuid":"u1","parentUuid":null,"promptSource":"sdk","message":{"role":"user","content":"First"}}"#,
        r#"{"type":"assistant","uuid":"a1","parentUuid":"u1","message":{"content":[{"type":"text","text":"Left behind"}]}}"#,
        r#"{"type":"attachment","uuid":"t1","parentUuid":"u1"}"#,
        r#"{"type":"assistant","uuid":"a2","parentUuid":"t1","message":{"content":[{"type":"text","text":"Kept"}]}}"#,
    ]
    .join("\n");
    assert_eq!(
        transcript::history(&file, None, Path::new("/w")),
        [
            HarnessEvent::UserMessageChunk {
                text: "First".into(),
                images: vec![],
                at: None
            },
            HarnessEvent::MessageChunk {
                text: "Kept".into()
            },
            HarnessEvent::MessageEnd {
                failed: false,
                at: None
            },
        ]
    );
}

#[test]
fn a_compaction_on_the_chain_keeps_the_turns_before_it() {
    let file = [
        r#"{"type":"user","uuid":"u1","parentUuid":null,"promptSource":"sdk","message":{"role":"user","content":"Before"}}"#,
        r#"{"type":"system","subtype":"compact_boundary","uuid":"c1","parentUuid":null,"logicalParentUuid":"u1","compactMetadata":{"trigger":"manual","preTokens":5000}}"#,
        r#"{"type":"user","uuid":"s1","parentUuid":"c1","isCompactSummary":true,"message":{"role":"user","content":"Summary"}}"#,
        r#"{"type":"user","uuid":"u2","parentUuid":"s1","promptSource":"sdk","message":{"role":"user","content":"After"}}"#,
    ]
    .join("\n");
    let user = |text: &str| HarnessEvent::UserMessageChunk {
        text: text.into(),
        images: vec![],
        at: None,
    };
    assert_eq!(
        transcript::history(&file, None, Path::new("/w")),
        [
            user("Before"),
            HarnessEvent::Compaction {
                summary: "".into(),
                tokens_before: Some(5000)
            },
            user("After"),
        ]
    );
}

#[test]
fn a_prompt_goes_back_to_the_last_reply_before_it() {
    let file = [
        r#"{"type":"user","uuid":"u1","parentUuid":null,"promptSource":"sdk","message":{"role":"user","content":"First"}}"#,
        r#"{"type":"assistant","uuid":"a1","parentUuid":"u1","message":{"content":[{"type":"text","text":"One"}]}}"#,
        r#"{"type":"system","uuid":"s1","parentUuid":"a1"}"#,
        r#"{"type":"user","uuid":"u2","parentUuid":"s1","promptSource":"sdk","message":{"role":"user","content":"Second"}}"#,
    ]
    .join("\n");
    assert_eq!(
        transcript::prompts(&file, None),
        [
            Message {
                id: "u1".into(),
                before: None,
                text: "First".into()
            },
            Message {
                id: "u2".into(),
                before: Some("a1".into()),
                text: "Second".into()
            },
        ]
    );
}

#[test]
fn a_worktree_path_names_its_folder() {
    assert_eq!(
        transcript::folder_name("/work/my app/feature.one"),
        "-work-my-app-feature-one"
    );
}

#[test]
fn a_reload_reopens_a_fork_until_its_first_message_names_the_new_session() {
    let fork = Open {
        session: Some("s1".into()),
        fork: true,
        at: Some("a1".into()),
        ..Open::default()
    };
    let mut reader = Reader::new(Path::new("/w"), None, fork.clone());
    let reload = |reader: &mut Reader| reader.send("/reload", &[], &[]).unwrap().relaunch;
    assert_eq!(reload(&mut reader), Some(fork));
    events(
        &mut reader,
        &[
            r#"{"type":"system","subtype":"init","session_id":"s2"}"#,
            r#"{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed"}"#,
        ],
    );
    assert_eq!(
        reload(&mut reader),
        Some(Open {
            session: Some("s2".into()),
            ..Open::default()
        })
    );
}

#[test]
fn get_usage_becomes_the_five_hour_and_weekly_windows() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    reader.initialize();
    reader.feed(br#"{"type":"control_response","response":{"subtype":"success","request_id":"chord-1","response":{}}}"#);
    let usage = r#"{"type":"control_response","response":{"subtype":"success","request_id":"chord-5","response":{"rate_limits_available":true,"rate_limits":{"five_hour":{"utilization":10,"resets_at":"2026-01-02T03:04:05.678901+00:00"},"seven_day":{"utilization":40.4,"resets_at":null}}}}}"#;
    assert_eq!(
        events(&mut reader, &[usage]),
        [HarnessEvent::Usage {
            harness: "claude",
            windows: vec![
                UsageWindow {
                    label: "5h 10%".into(),
                    resets_at: Some(1767323045678.0)
                },
                UsageWindow {
                    label: "week 40%".into(),
                    resets_at: None
                },
            ]
        }]
    );
}

#[test]
fn an_api_key_has_no_usage_and_its_refusal_says_nothing() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    reader.initialize();
    reader.feed(br#"{"type":"control_response","response":{"subtype":"success","request_id":"chord-1","response":{}}}"#);
    let refused = r#"{"type":"control_response","response":{"subtype":"error","request_id":"chord-5","error":"no subscription"}}"#;
    assert_eq!(events(&mut reader, &[refused]), []);
}

#[test]
fn agent_tasks_report_their_lineage_and_a_shell_task_none() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let shell = r#"{"type":"system","subtype":"task_started","task_id":"b1","task_type":"local_bash","description":"npm test"}"#;
    assert_eq!(events(&mut reader, &[shell]), []);
    let records = [
        r#"{"type":"system","subtype":"task_started","task_id":"a1","task_type":"local_agent","subagent_type":"Explore","description":"Find the auth code"}"#,
        r#"{"type":"system","subtype":"task_started","task_id":"a2","task_type":"local_agent","parent_task_id":"a1","description":"Read the tests"}"#,
        r#"{"type":"system","subtype":"task_progress","task_id":"a1","last_tool_name":"Grep"}"#,
        r#"{"type":"system","subtype":"task_notification","task_id":"a2","status":"failed"}"#,
    ];
    assert_eq!(
        Subagents::lineage(&events(&mut reader, &records)),
        [
            (
                0,
                "Explore".into(),
                SubagentState::Running,
                Some("Grep".into())
            ),
            (1, "Read the tests".into(), SubagentState::Error, None),
        ]
    );
    let done = r#"{"type":"system","subtype":"task_updated","task_id":"a1","patch":{"status":"completed"}}"#;
    assert_eq!(
        Subagents::lineage(&events(&mut reader, &[done]))[0],
        (
            0,
            "Explore".into(),
            SubagentState::Done,
            Some("Grep".into())
        )
    );
}

#[test]
fn a_rate_limit_event_reads_the_usage_at_most_every_30_seconds() {
    let mut reader = Reader::new(Path::new("/w"), None, Open::default());
    let event = br#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"five_hour"}}"#;
    assert_eq!(
        reader.feed(event).lines,
        [
            json!({ "type": "control_request", "request_id": "chord-1", "request": { "subtype": "get_usage" } })
        ]
    );
    assert!(reader.feed(event).lines.is_empty());
}
