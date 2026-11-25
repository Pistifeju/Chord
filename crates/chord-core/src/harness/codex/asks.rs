//! Codex's asks as dialogs: the approvals, questions and forms Codex waits on, and the result each answer sends back.

use std::collections::VecDeque;

use serde_json::{Map, Value, json};

use crate::harness::asks::{self, Field, OTHER, Settled, Step, form_steps, input, select, strings};
use crate::harness::shell::shell_script;
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent};

pub(super) type Ask = asks::Ask<Reply>;
pub(super) type Asks = asks::Asks<Reply>;

pub(super) type Parsed = asks::Parsed<Reply>;

/// How an ask's answer goes back to Codex.
pub(super) struct Reply {
    /// The id of Codex's request; none for an agent message's questions, whose answer goes out as a new turn.
    pub(super) request: Option<Value>,
    kind: Kind,
    /// The result an Esc sends.
    cancel: Value,
}

enum Kind {
    /// The picked option's result is the whole answer.
    Decided,
    /// The picked action, accept or decline, of a link.
    Link,
    UserInput,
    Form,
    Turn,
}

/// What an answer leads to: events, the line that answers Codex, and the text a new turn sends.
#[derive(Default)]
pub(super) struct Answered {
    pub(super) events: Vec<HarnessEvent>,
    pub(super) line: Option<Value>,
    pub(super) text: Option<String>,
}

pub(super) fn answer(asks: &mut Asks, id: &str, answer: &DialogAnswer) -> Answered {
    let answered = asks.answer(id, answer);
    let mut out = Answered {
        events: answered.events,
        ..Answered::default()
    };
    let Some((reply, settled)) = answered.settled else {
        return out;
    };
    let result = match (settled, reply.kind) {
        (Settled::Cancelled, _) => reply.cancel,
        (Settled::Picked(action), Kind::Link) => {
            json!({ "action": action, "content": null, "_meta": null })
        }
        (Settled::Picked(result), _) => result,
        (Settled::Answers(answers), Kind::Turn) => {
            out.text = Some(turn_text(&answers));
            return out;
        }
        (Settled::Answers(answers), Kind::UserInput) => {
            let answers: Map<String, Value> = answers
                .into_iter()
                .map(|(key, value)| (key, json!({ "answers": [plain(&value)] })))
                .collect();
            json!({ "answers": answers })
        }
        (Settled::Answers(answers), Kind::Form) => {
            let content: Map<String, Value> = answers
                .into_iter()
                .filter(|(_, value)| *value != "")
                .collect();
            json!({ "action": "accept", "content": content, "_meta": null })
        }
        (Settled::Answers(_), Kind::Decided | Kind::Link) => Value::Null,
    };
    out.line = reply
        .request
        .map(|id| json!({ "id": id, "result": result }));
    out
}

fn turn_text(answers: &[(String, Value)]) -> String {
    answers
        .iter()
        .map(|(question, answer)| format!("{question}\n{}", plain(answer)))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn plain(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), String::from)
}

/// The ask a server request opens; `files` gives the paths of an open file change by its item id.
pub(super) fn parse(
    id: &Value,
    method: &str,
    params: &Value,
    files: impl Fn(&str) -> Option<String>,
) -> Parsed {
    let request = Some(id.clone());
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    let reason = |fallback: &str| {
        params["reason"]
            .as_str()
            .filter(|reason| !reason.is_empty())
            .unwrap_or(fallback)
            .to_string()
    };
    let decided = |title: String, detail: Option<String>, options: Vec<(&str, Value)>, cancel| {
        let (labels, results): (Vec<_>, Vec<_>) = options
            .into_iter()
            .map(|(label, result)| (label.to_string(), result))
            .unzip();
        Parsed::Ask(ask(
            request.clone(),
            Kind::Decided,
            cancel,
            VecDeque::from([Step {
                title,
                ask: select(labels, detail, vec![], false),
                field: Field::Decision(results),
            }]),
        ))
    };
    let decision = |decision: Value| json!({ "decision": decision });
    match method {
        "item/commandExecution/requestApproval" => {
            let command = text(&params["command"]);
            let script = shell_script(&command).unwrap_or(command);
            let mut options = vec![
                ("Allow", decision(json!("accept"))),
                (
                    "Allow for This Session",
                    decision(json!("acceptForSession")),
                ),
            ];
            let prefix = params["proposedExecpolicyAmendment"].as_array();
            let always = prefix.map(|prefix| {
                let words: Vec<String> = prefix.iter().map(&text).collect();
                format!("Always Allow \u{201c}{}\u{201d}", words.join(" "))
            });
            if let (Some(label), Some(prefix)) = (&always, prefix) {
                options.push((
                    label.as_str(),
                    decision(json!({ "acceptWithExecpolicyAmendment": { "execpolicy_amendment": prefix } })),
                ));
            }
            options.push(("Deny", decision(json!("decline"))));
            decided(
                reason("Run This Command?"),
                Some(script),
                options,
                decision(json!("cancel")),
            )
        }
        "item/fileChange/requestApproval" => decided(
            reason("Apply These Changes?"),
            files(params["itemId"].as_str().unwrap_or_default()),
            vec![
                ("Allow", decision(json!("accept"))),
                (
                    "Allow for This Session",
                    decision(json!("acceptForSession")),
                ),
                ("Deny", decision(json!("decline"))),
            ],
            decision(json!("cancel")),
        ),
        "item/permissions/requestApproval" => {
            let asked = &params["permissions"];
            let granted: Map<String, Value> = asked
                .as_object()
                .into_iter()
                .flatten()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let deny = json!({ "permissions": {}, "scope": "turn" });
            decided(
                reason("Allow More Access?"),
                permissions_detail(asked),
                vec![
                    (
                        "Allow for This Turn",
                        json!({ "permissions": granted, "scope": "turn" }),
                    ),
                    (
                        "Allow for This Session",
                        json!({ "permissions": granted, "scope": "session" }),
                    ),
                    ("Deny", deny.clone()),
                ],
                deny,
            )
        }
        "item/tool/requestUserInput" => Parsed::Ask(ask(
            request,
            Kind::UserInput,
            json!({ "answers": {} }),
            params["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .map(user_question)
                .collect(),
        )),
        "mcpServer/elicitation/request" if params["mode"] == "url" => Parsed::Ask(ask(
            request,
            Kind::Link,
            json!({ "action": "cancel", "content": null, "_meta": null }),
            VecDeque::from([Step {
                title: text(&params["serverName"]),
                ask: DialogAsk::Confirm {
                    message: text(&params["message"]),
                    detail: None,
                    url: Some(text(&params["url"])),
                },
                field: Field::Link,
            }]),
        )),
        "mcpServer/elicitation/request" => {
            let cancel = json!({ "action": "cancel", "content": null, "_meta": null });
            match form_steps(&params["requestedSchema"], &text(&params["message"])) {
                Some(steps) if !steps.is_empty() => {
                    Parsed::Ask(ask(request, Kind::Form, cancel, steps))
                }
                _ => Parsed::Refused {
                    line: json!({ "id": id, "result": cancel }),
                    notice: format!(
                        "{} asked for a form Chord cannot show",
                        text(&params["serverName"])
                    ),
                },
            }
        }
        _ => Parsed::Refused {
            line: json!({
                "id": id,
                "error": { "code": -32601, "message": format!("Chord does not answer {method}") },
            }),
            notice: format!("Codex asked for {method}, which Chord cannot answer"),
        },
    }
}

/// The questions an agent message ends with; their answers go out as the next message.
pub(super) fn message_questions(item: &Value) -> Option<Ask> {
    let steps: VecDeque<Step> = item["questions"]
        .as_array()?
        .iter()
        .map(|question| {
            let title = question["title"].as_str().unwrap_or_default().to_string();
            let options = strings(&question["options"]);
            let field = |values: &[String]| Field::Choice {
                key: title.clone(),
                values: values.iter().map(|value| json!(value)).collect(),
                other: None,
            };
            match options.is_empty() {
                true => input(title.clone(), String::new(), title, None),
                false => Step {
                    field: field(&options),
                    ask: select(options, None, vec![], false),
                    title,
                },
            }
        })
        .collect();
    (!steps.is_empty()).then(|| ask(None, Kind::Turn, Value::Null, steps))
}

fn ask(request: Option<Value>, kind: Kind, cancel: Value, steps: VecDeque<Step>) -> Ask {
    Ask::new(
        Reply {
            request,
            kind,
            cancel,
        },
        steps,
    )
}

fn user_question(question: &Value) -> Step {
    let key = question["id"].as_str().unwrap_or_default().to_string();
    let title = question["question"]
        .as_str()
        .or(question["header"].as_str())
        .unwrap_or_default()
        .to_string();
    let options = question["options"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    if options.is_empty() || question["isSecret"] == true {
        return input(title, String::new(), key, None);
    }
    let other = question["isOther"] == true;
    let mut labels: Vec<String> = options
        .iter()
        .map(|option| label(&option["label"]))
        .collect();
    let mut descriptions: Vec<String> = options
        .iter()
        .map(|option| label(&option["description"]))
        .collect();
    if other {
        labels.push(OTHER.into());
        descriptions.push(String::new());
    }
    Step {
        title,
        field: Field::Choice {
            other: other.then(|| key.clone()),
            key,
            values: labels.iter().map(|label| json!(label)).collect(),
        },
        ask: select(labels, None, descriptions, false),
    }
}

// The network access and the paths a permission ask names, one per line.
fn permissions_detail(asked: &Value) -> Option<String> {
    let files = &asked["fileSystem"];
    let mut lines = vec![];
    for (access, key) in [("Write", "write"), ("Read", "read")] {
        for path in strings(&files[key]) {
            lines.push(format!("{access} {path}"));
        }
    }
    for entry in files["entries"].as_array().into_iter().flatten() {
        let path = &entry["path"];
        let place = path["path"].as_str().or(path["pattern"].as_str());
        let access = match entry["access"].as_str() {
            Some("write") => "Write",
            Some("read") => "Read",
            _ => "Deny",
        };
        if let Some(place) = place {
            lines.push(format!("{access} {place}"));
        }
    }
    if asked["network"]["enabled"] == true {
        lines.push("Network access".into());
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn label(value: &Value) -> String {
    value.as_str().unwrap_or_default().into()
}
