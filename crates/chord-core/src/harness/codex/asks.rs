//! Codex's asks as dialogs: the approvals, questions and forms Codex waits on, one open per pane and the rest waiting oldest first, the walk through one ask's questions, and the answer that goes back.

use std::collections::VecDeque;

use serde_json::{Map, Value, json};

use crate::harness::shell::shell_script;
use crate::harness::{DialogAnswer, DialogAsk, HarnessEvent};

const OTHER: &str = "Other";

/// What a server request leads to: an ask to show, or an answer at once with a notice.
pub(super) enum Parsed {
    Ask(Ask),
    Refused { line: Value, notice: String },
}

/// One request's dialogs, shown one at a time, and what the answers become.
pub(super) struct Ask {
    /// The id of Codex's request; none for an agent message's questions, whose answer goes out as a new turn.
    request: Option<Value>,
    reply: Reply,
    /// The result an Esc sends.
    cancel: Value,
    steps: VecDeque<Step>,
    answers: Vec<(String, Value)>,
}

enum Reply {
    /// The picked option's result is the whole answer.
    Decided,
    UserInput,
    Form,
    Turn,
}

struct Step {
    title: String,
    ask: DialogAsk,
    field: Field,
}

enum Field {
    /// Each option's whole result.
    Decision(Vec<Value>),
    /// Each option's value; `other` turns the last option into an input.
    Choice {
        key: String,
        values: Vec<Value>,
        other: bool,
    },
    Text {
        key: String,
        number: Option<Number>,
    },
    Flag {
        key: String,
    },
    /// A link the user opens: accept after Open, decline after Decline.
    Link,
}

#[derive(Clone, Copy)]
enum Number {
    Integer,
    Float,
}

/// What an answer leads to: events, the line that answers Codex, and the text a new turn sends.
#[derive(Default)]
pub(super) struct Answered {
    pub(super) events: Vec<HarnessEvent>,
    pub(super) line: Option<Value>,
    pub(super) text: Option<String>,
}

/// The pane's asks: the one open and those that wait.
#[derive(Default)]
pub(super) struct Asks {
    open: Option<Ask>,
    waiting: VecDeque<Ask>,
    /// How many dialogs went out, so each one's id is new and a late answer to an old one does nothing.
    shown: u64,
}

impl Asks {
    /// Opens the ask, or queues it behind the open one.
    pub(super) fn push(&mut self, ask: Ask) -> Vec<HarnessEvent> {
        if self.open.is_some() {
            self.waiting.push_back(ask);
            return vec![];
        }
        self.open = Some(ask);
        self.show()
    }

    pub(super) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Answered {
        if id != self.shown.to_string() {
            return Answered::default();
        }
        let Some(mut ask) = self.open.take() else {
            return Answered::default();
        };
        let Some(step) = ask.steps.pop_front() else {
            return Answered::default();
        };
        let mut answered = Answered::default();
        let result = match step.take(answer) {
            Taken::Cancel => Some(ask.cancel.clone()),
            Taken::Result(result) => Some(result),
            Taken::Value(key, value) => {
                ask.answers.push((key, value));
                None
            }
            Taken::Next(next) => {
                ask.steps.push_front(next);
                None
            }
        };
        match result {
            Some(result) => answered.line = ask.respond(result),
            None if !ask.steps.is_empty() => {
                self.open = Some(ask);
                answered.events.push(HarnessEvent::DialogClosed);
                answered.events.extend(self.show());
                return answered;
            }
            None => match ask.reply {
                Reply::Turn => answered.text = Some(ask.turn_text()),
                _ => answered.line = ask.respond(ask.reply_result()),
            },
        }
        answered.events = self.close();
        answered
    }

    /// Codex withdrew a request: its dialog closes, or it stops waiting.
    pub(super) fn resolved(&mut self, request: &Value) -> Vec<HarnessEvent> {
        self.waiting
            .retain(|ask| ask.request.as_ref() != Some(request));
        match &self.open {
            Some(ask) if ask.request.as_ref() == Some(request) => self.close(),
            _ => vec![],
        }
    }

    /// The turn ended and Codex withdrew its requests; an agent message's questions stay.
    pub(super) fn turn_ended(&mut self) -> Vec<HarnessEvent> {
        self.waiting.retain(|ask| ask.request.is_none());
        match &self.open {
            Some(ask) if ask.request.is_some() => self.close(),
            _ => vec![],
        }
    }

    fn show(&mut self) -> Vec<HarnessEvent> {
        let Some(step) = self.open.as_ref().and_then(|ask| ask.steps.front()) else {
            return vec![];
        };
        self.shown += 1;
        vec![HarnessEvent::Dialog {
            id: self.shown.to_string(),
            title: step.title.clone(),
            ask: step.ask.clone(),
        }]
    }

    fn close(&mut self) -> Vec<HarnessEvent> {
        self.open = self.waiting.pop_front();
        let mut events = vec![HarnessEvent::DialogClosed];
        events.extend(self.show());
        events
    }
}

enum Taken {
    Cancel,
    Result(Value),
    Value(String, Value),
    Next(Step),
}

impl Step {
    fn take(self, answer: &DialogAnswer) -> Taken {
        let options = match &self.ask {
            DialogAsk::Select { options, .. } => options.as_slice(),
            _ => &[],
        };
        let at = |label: &str| options.iter().position(|option| option == label);
        match (self.field, answer) {
            (_, DialogAnswer::Cancelled) => Taken::Cancel,
            (Field::Decision(results), DialogAnswer::Value { value }) => at(value)
                .and_then(|at| results.get(at).cloned())
                .map_or(Taken::Cancel, Taken::Result),
            (Field::Link, DialogAnswer::Confirmed { confirmed }) => {
                let action = if *confirmed { "accept" } else { "decline" };
                Taken::Result(json!({ "action": action, "content": null, "_meta": null }))
            }
            (
                Field::Choice {
                    key, other: true, ..
                },
                DialogAnswer::Value { value },
            ) if value == OTHER => Taken::Next(Step {
                title: self.title,
                ask: DialogAsk::Input {
                    placeholder: String::new(),
                },
                field: Field::Text { key, number: None },
            }),
            (Field::Choice { key, values, .. }, DialogAnswer::Value { value }) => {
                let picked = at(value).and_then(|at| values.get(at).cloned());
                Taken::Value(key, picked.unwrap_or_else(|| json!(value)))
            }
            (Field::Choice { key, values, .. }, DialogAnswer::Values { values: picked }) => {
                let picked = picked
                    .iter()
                    .filter_map(|label| at(label).and_then(|at| values.get(at).cloned()))
                    .collect();
                Taken::Value(key, Value::Array(picked))
            }
            (Field::Text { key, number }, DialogAnswer::Value { value }) => {
                let text = value.trim();
                let parsed = match number {
                    Some(Number::Integer) => text.parse::<i64>().ok().map(Value::from),
                    Some(Number::Float) => text.parse::<f64>().ok().map(Value::from),
                    None => None,
                };
                Taken::Value(key, parsed.unwrap_or_else(|| json!(value)))
            }
            (Field::Flag { key }, DialogAnswer::Confirmed { confirmed }) => {
                Taken::Value(key, json!(confirmed))
            }
            _ => Taken::Cancel,
        }
    }
}

impl Ask {
    fn respond(&self, result: Value) -> Option<Value> {
        let id = self.request.as_ref()?;
        Some(json!({ "id": id, "result": result }))
    }

    fn reply_result(&self) -> Value {
        match self.reply {
            Reply::UserInput => {
                let answers: Map<String, Value> = self
                    .answers
                    .iter()
                    .map(|(key, value)| {
                        let text = value
                            .as_str()
                            .map_or_else(|| value.to_string(), String::from);
                        (key.clone(), json!({ "answers": [text] }))
                    })
                    .collect();
                json!({ "answers": answers })
            }
            Reply::Form => {
                let content: Map<String, Value> = self
                    .answers
                    .iter()
                    .filter(|(_, value)| *value != "")
                    .cloned()
                    .collect();
                json!({ "action": "accept", "content": content, "_meta": null })
            }
            Reply::Decided | Reply::Turn => Value::Null,
        }
    }

    fn turn_text(&self) -> String {
        self.answers
            .iter()
            .map(|(question, answer)| {
                let answer = answer
                    .as_str()
                    .map_or_else(|| answer.to_string(), String::from);
                format!("{question}\n{answer}")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
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
        Parsed::Ask(Ask {
            request: request.clone(),
            reply: Reply::Decided,
            cancel,
            steps: VecDeque::from([Step {
                title,
                ask: select(labels, detail, vec![], false),
                field: Field::Decision(results),
            }]),
            answers: vec![],
        })
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
        "item/tool/requestUserInput" => Parsed::Ask(Ask {
            request,
            reply: Reply::UserInput,
            cancel: json!({ "answers": {} }),
            steps: params["questions"]
                .as_array()
                .into_iter()
                .flatten()
                .map(user_question)
                .collect(),
            answers: vec![],
        }),
        "mcpServer/elicitation/request" if params["mode"] == "url" => Parsed::Ask(Ask {
            request,
            reply: Reply::Decided,
            cancel: json!({ "action": "cancel", "content": null, "_meta": null }),
            steps: VecDeque::from([Step {
                title: text(&params["serverName"]),
                ask: DialogAsk::Confirm {
                    message: text(&params["message"]),
                    url: Some(text(&params["url"])),
                },
                field: Field::Link,
            }]),
            answers: vec![],
        }),
        "mcpServer/elicitation/request" => {
            let cancel = json!({ "action": "cancel", "content": null, "_meta": null });
            match form_steps(&params["requestedSchema"], &text(&params["message"])) {
                Some(steps) if !steps.is_empty() => Parsed::Ask(Ask {
                    request,
                    reply: Reply::Form,
                    cancel,
                    steps,
                    answers: vec![],
                }),
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
                other: false,
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
    (!steps.is_empty()).then_some(Ask {
        request: None,
        reply: Reply::Turn,
        cancel: Value::Null,
        steps,
        answers: vec![],
    })
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
            key,
            values: labels.iter().map(|label| json!(label)).collect(),
            other,
        },
        ask: select(labels, None, descriptions, false),
    }
}

// An enum is a select, an array of enums a multi select, a boolean a confirm, a string or number an input. A field of any other type makes the form unreadable.
fn form_steps(schema: &Value, message: &str) -> Option<VecDeque<Step>> {
    schema["properties"]
        .as_object()?
        .iter()
        .map(|(key, field)| {
            let title = field["title"]
                .as_str()
                .filter(|title| !title.is_empty())
                .unwrap_or(if message.is_empty() { key } else { message })
                .to_string();
            let description = field["description"].as_str().map(String::from);
            let choice = |options: &Value, multi: bool| {
                let (labels, values) = enum_options(options, &field["enumNames"])?;
                Some(Step {
                    title: title.clone(),
                    ask: select(labels, description.clone(), vec![], multi),
                    field: Field::Choice {
                        key: key.clone(),
                        values,
                        other: false,
                    },
                })
            };
            match field["type"].as_str()? {
                "string" if field["enum"].is_array() => choice(&field["enum"], false),
                "string" if field["oneOf"].is_array() => choice(&field["oneOf"], false),
                "array" => {
                    let items = &field["items"];
                    choice(items.get("enum").or(items.get("anyOf"))?, true)
                }
                "boolean" => Some(Step {
                    title: title.clone(),
                    ask: DialogAsk::Confirm {
                        message: description.unwrap_or_default(),
                        url: None,
                    },
                    field: Field::Flag { key: key.clone() },
                }),
                "string" => Some(input(
                    title,
                    description.unwrap_or_default(),
                    key.clone(),
                    None,
                )),
                "integer" => Some(input(
                    title,
                    description.unwrap_or_default(),
                    key.clone(),
                    Some(Number::Integer),
                )),
                "number" => Some(input(
                    title,
                    description.unwrap_or_default(),
                    key.clone(),
                    Some(Number::Float),
                )),
                _ => None,
            }
        })
        .collect()
}

// Plain strings, with `enumNames` as their labels when given, or `{ const, title }` options.
fn enum_options(options: &Value, names: &Value) -> Option<(Vec<String>, Vec<Value>)> {
    let options = options.as_array()?;
    let names = strings(names);
    options
        .iter()
        .enumerate()
        .map(|(at, option)| match option.as_str() {
            Some(value) => Some((names.get(at).cloned().unwrap_or(value.into()), json!(value))),
            None => {
                let value = option["const"].as_str()?;
                let title = option["title"].as_str().unwrap_or(value);
                Some((title.into(), json!(value)))
            }
        })
        .collect::<Option<Vec<_>>>()
        .map(|pairs| pairs.into_iter().unzip())
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

fn select(
    options: Vec<String>,
    detail: Option<String>,
    descriptions: Vec<String>,
    multi: bool,
) -> DialogAsk {
    DialogAsk::Select {
        options,
        current: None,
        detail,
        descriptions,
        multi,
    }
}

fn input(title: String, placeholder: String, key: String, number: Option<Number>) -> Step {
    Step {
        title,
        ask: DialogAsk::Input { placeholder },
        field: Field::Text { key, number },
    }
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(String::from))
        .collect()
}

fn label(value: &Value) -> String {
    value.as_str().unwrap_or_default().into()
}
