use std::collections::VecDeque;
use std::path::Path;

use serde_json::{Map, Value, json};

use super::requests;
use crate::harness::asks::{self, Field, OTHER, Settled, Step, form_steps, select};
use crate::harness::{DialogAsk, relative, text};

pub(super) type Asks = asks::Asks<Reply>;
pub(super) type Parsed = asks::Parsed<Reply>;

pub(super) struct Reply {
    request: Value,
    kind: Kind,
}

enum Kind {
    Permission { reject: Option<Value> },
    Form,
}

pub(super) fn parse(id: &Value, method: &str, params: &Value, cwd: &Path) -> Parsed {
    let ask = |kind, steps| {
        Parsed::Ask(asks::Ask::new(
            Reply {
                request: id.clone(),
                kind,
            },
            steps,
        ))
    };
    match method {
        "session/request_permission" => {
            let call = &params["toolCall"];
            let options = params["options"].as_array().map_or(&[][..], Vec::as_slice);
            let reject = options
                .iter()
                .find(|option| option["kind"] == "reject_once")
                .map(|option| option["optionId"].clone());
            let title = match call["kind"].as_str() {
                Some("execute") => "Run This Command?".into(),
                Some("edit") => "Apply This Change?".into(),
                Some("read") => "Read This File?".into(),
                _ => Some(text(&call["title"]))
                    .filter(|title| !title.is_empty())
                    .unwrap_or_else(|| "Allow This Action?".into()),
            };
            let paths: Vec<String> = call["locations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|location| location["path"].as_str())
                .map(|path| relative(path, cwd))
                .collect();
            let detail = call["rawInput"]["command"]
                .as_str()
                .map(String::from)
                .or_else(|| (!paths.is_empty()).then(|| paths.join("\n")));
            let step = Step {
                title,
                ask: select(
                    options.iter().map(|option| text(&option["name"])).collect(),
                    detail,
                    vec![],
                    false,
                ),
                field: Field::Decision(
                    options
                        .iter()
                        .map(|option| option["optionId"].clone())
                        .collect(),
                ),
            };
            ask(Kind::Permission { reject }, VecDeque::from([step]))
        }
        "elicitation/create" if params["mode"] == "form" => {
            match form_steps(&params["requestedSchema"], &text(&params["message"])) {
                Some(steps) if !steps.is_empty() => ask(Kind::Form, offer_other(steps)),
                _ => Parsed::Refused {
                    line: line(id, json!({ "action": "cancel" })),
                    notice: "OpenCode asked for a form Chord cannot show".into(),
                },
            }
        }
        _ => Parsed::Refused {
            line: requests::refusal(id, method),
            notice: format!("OpenCode asked for {method}, which Chord does not handle"),
        },
    }
}

pub(super) fn answer(reply: Reply, settled: Settled) -> Value {
    let result = match (&reply.kind, settled) {
        (
            Kind::Permission {
                reject: Some(option),
            },
            Settled::Cancelled,
        ) => selected(option.clone()),
        (Kind::Permission { .. }, Settled::Picked(option)) => selected(option),
        (Kind::Form, Settled::Answers(answers)) => {
            let content: Map<String, Value> = answers
                .into_iter()
                .filter(|(_, value)| *value != "")
                .collect();
            json!({ "action": "accept", "content": content })
        }
        _ => cancelled(&reply.kind),
    };
    line(&reply.request, result)
}

pub(super) fn stopped(reply: &Reply) -> Value {
    line(&reply.request, cancelled(&reply.kind))
}

fn selected(option: Value) -> Value {
    json!({ "outcome": { "outcome": "selected", "optionId": option } })
}

fn cancelled(kind: &Kind) -> Value {
    match kind {
        Kind::Permission { .. } => json!({ "outcome": { "outcome": "cancelled" } }),
        Kind::Form => json!({ "action": "cancel" }),
    }
}

fn line(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn offer_other(mut steps: VecDeque<Step>) -> VecDeque<Step> {
    let inputs: Vec<String> = steps
        .iter()
        .filter_map(|step| match &step.field {
            Field::Text { key, number: None } => Some(key.clone()),
            _ => None,
        })
        .collect();
    let mut typed = vec![];
    for step in &mut steps {
        let (
            Field::Choice { key, values, other },
            DialogAsk::Select {
                options,
                descriptions,
                multi: false,
                ..
            },
        ) = (&mut step.field, &mut step.ask)
        else {
            continue;
        };
        let custom = format!("{key}_custom");
        if !inputs.contains(&custom) {
            continue;
        }
        options.push(OTHER.into());
        if !descriptions.is_empty() {
            descriptions.push(String::new());
        }
        values.push(json!(OTHER));
        *other = Some(custom.clone());
        typed.push(custom);
    }
    steps.retain(|step| !matches!(&step.field, Field::Text { key, .. } if typed.contains(key)));
    steps
}
