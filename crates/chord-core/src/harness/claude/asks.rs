use std::collections::VecDeque;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::{blocks, permissions, requests};
use crate::harness::asks::{self, Field, OTHER, Settled, Step, form_steps, select};
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent, relative, text};

pub(super) type Ask = asks::Ask<Reply>;
pub(super) type Asks = asks::Asks<Reply>;

pub(super) type Parsed = asks::Parsed<Reply>;

pub(super) struct Reply {
    pub(super) request: String,
    kind: Kind,
    cancel: Value,
}

enum Kind {
    Decided,
    Questions(Value),
    Form,
    Link,
}

pub(super) fn answer(
    asks: &mut Asks,
    id: &str,
    answer: &DialogAnswer,
) -> (Vec<HarnessEvent>, Option<Value>) {
    let answered = asks.answer(id, answer);
    let Some((reply, settled)) = answered.settled else {
        return (answered.events, None);
    };
    let response = match (settled, reply.kind) {
        (Settled::Cancelled, _) => reply.cancel,
        (Settled::Picked(action), Kind::Link) => json!({ "action": action }),
        (Settled::Picked(response), _) => response,
        (Settled::Answers(answers), Kind::Questions(mut input)) => {
            let answers: Map<String, Value> = answers
                .into_iter()
                .map(|(question, value)| (question, json!(joined(&value))))
                .collect();
            if let Some(input) = input.as_object_mut() {
                input.insert("answers".into(), Value::Object(answers));
            }
            json!({ "behavior": "allow", "updatedInput": input })
        }
        (Settled::Answers(answers), Kind::Form) => {
            let content: Map<String, Value> = answers
                .into_iter()
                .filter(|(_, value)| *value != "")
                .collect();
            json!({ "action": "accept", "content": content })
        }
        (Settled::Answers(_), Kind::Decided | Kind::Link) => reply.cancel,
    };
    (
        answered.events,
        Some(requests::control_success(&reply.request, response)),
    )
}

pub(super) fn parse(record: &Value, cwd: &Path) -> Parsed {
    let id = text(&record["request_id"]);
    let request = &record["request"];
    let subtype = text(&request["subtype"]);
    match subtype.as_str() {
        "can_use_tool" => {
            let input = &request["input"];
            match request["tool_name"].as_str() {
                Some("AskUserQuestion") => {
                    questions(id.clone(), input).unwrap_or_else(|| permission(id, request, cwd))
                }
                Some("ExitPlanMode") => Parsed::Ask(plan(id)),
                _ => permission(id, request, cwd),
            }
        }
        "elicitation" => elicitation(id, request),
        _ => Parsed::Refused {
            line: requests::control_error(&id, &format!("Chord does not answer {subtype}")),
            notice: format!("Claude Code asked for {subtype}, which Chord cannot answer"),
        },
    }
}

fn permission(id: String, request: &Value, cwd: &Path) -> Parsed {
    let tool = text(&request["tool_name"]);
    let input = &request["input"];
    let shown = request["display_name"].as_str().unwrap_or(&tool);
    let path = input["file_path"]
        .as_str()
        .or(input["notebook_path"].as_str());
    let (fallback, detail) = match (tool.as_str(), path) {
        ("Bash", _) => ("Run This Command?".into(), text(&input["command"])),
        ("Write" | "Edit" | "NotebookEdit", Some(path)) => {
            ("Apply This Change?".into(), relative(path, cwd))
        }
        _ => (format!("Allow {shown}?"), blocks::arguments(input, "")),
    };
    let title = request["title"]
        .as_str()
        .filter(|title| !title.is_empty())
        .map_or(fallback, String::from);
    let mut options = vec![("Allow".to_string(), json!({ "behavior": "allow" }))];
    let suppressed = request["suppress_always_allow_rule"] == true;
    for suggestion in request["permission_suggestions"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let label = match suggestion["type"].as_str() {
            Some("addRules") if !suppressed => {
                let rules: Vec<String> = suggestion["rules"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|rule| match rule["ruleContent"].as_str() {
                        Some(content) => format!("\u{201c}{content}\u{201d}"),
                        None => text(&rule["toolName"]),
                    })
                    .collect();
                format!("Always Allow {}", rules.join(", "))
            }
            Some("setMode") => format!(
                "Allow and Switch to {}",
                permissions::label(suggestion["mode"].as_str().unwrap_or_default())
            ),
            _ => continue,
        };
        options.push((
            label,
            json!({ "behavior": "allow", "updatedPermissions": [suggestion] }),
        ));
    }
    options.push((
        "Deny".into(),
        json!({ "behavior": "deny", "message": "The user denied this tool call" }),
    ));
    Parsed::Ask(decided(
        id,
        title,
        (!detail.is_empty()).then_some(detail),
        options,
    ))
}

fn plan(id: String) -> Ask {
    let allow = |mode: &str| {
        json!({
            "behavior": "allow",
            "updatedPermissions": [{ "type": "setMode", "mode": mode, "destination": "session" }],
        })
    };
    decided(
        id,
        "Run This Plan?".into(),
        None,
        vec![
            ("Yes, Accept Edits".into(), allow(permissions::ACCEPT_EDITS)),
            ("Yes, Ask Before Edits".into(), allow(permissions::DEFAULT)),
            (
                "Keep Planning".into(),
                json!({ "behavior": "deny", "message": "The user wants to keep planning" }),
            ),
        ],
    )
}

fn questions(id: String, input: &Value) -> Option<Parsed> {
    let steps: VecDeque<Step> = input["questions"]
        .as_array()?
        .iter()
        .map(|question| {
            let title = text(&question["question"]);
            let multi = question["multiSelect"] == true;
            let options = question["options"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let mut labels: Vec<String> = options
                .iter()
                .map(|option| text(&option["label"]))
                .collect();
            let mut descriptions: Vec<String> = options
                .iter()
                .map(|option| text(&option["description"]))
                .collect();
            if !multi {
                labels.push(OTHER.into());
                descriptions.push(String::new());
            }
            Step {
                field: Field::Choice {
                    key: title.clone(),
                    values: labels.iter().map(|label| json!(label)).collect(),
                    other: (!multi).then(|| title.clone()),
                },
                ask: select(labels, None, descriptions, multi),
                title,
            }
        })
        .collect();
    (!steps.is_empty()).then(|| {
        Parsed::Ask(Ask::new(
            Reply {
                request: id,
                kind: Kind::Questions(input.clone()),
                cancel: interrupt(),
            },
            steps,
        ))
    })
}

fn elicitation(id: String, request: &Value) -> Parsed {
    let server = text(&request["mcp_server_name"]);
    let message = text(&request["message"]);
    let cancel = json!({ "action": "cancel" });
    if request["mode"] == "url" {
        let step = Step {
            title: server,
            ask: DialogAsk::Confirm {
                message,
                detail: None,
                url: Some(text(&request["url"])),
            },
            field: Field::Link,
        };
        let reply = Reply {
            request: id,
            kind: Kind::Link,
            cancel,
        };
        return Parsed::Ask(Ask::new(reply, VecDeque::from([step])));
    }
    match form_steps(&request["requested_schema"], &message) {
        Some(steps) if !steps.is_empty() => Parsed::Ask(Ask::new(
            Reply {
                request: id,
                kind: Kind::Form,
                cancel,
            },
            steps,
        )),
        _ => Parsed::Refused {
            line: requests::control_success(&id, cancel),
            notice: format!("{server} asked for a form Chord cannot show"),
        },
    }
}

fn decided(
    id: String,
    title: String,
    detail: Option<String>,
    options: Vec<(String, Value)>,
) -> Ask {
    let (labels, answers): (Vec<_>, Vec<_>) = options.into_iter().unzip();
    Ask::new(
        Reply {
            request: id,
            kind: Kind::Decided,
            cancel: interrupt(),
        },
        VecDeque::from([Step {
            title,
            ask: select(labels, detail, vec![], false),
            field: Field::Decision(answers),
        }]),
    )
}

fn interrupt() -> Value {
    json!({ "behavior": "deny", "message": "The user stopped the turn", "interrupt": true })
}

fn joined(value: &Value) -> String {
    match value {
        Value::Array(labels) => labels
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", "),
        value => value.as_str().unwrap_or_default().into(),
    }
}
