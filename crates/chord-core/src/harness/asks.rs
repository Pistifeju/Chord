//! An agent's asks as dialogs: one open per pane and the rest waiting oldest first, the walk through one ask's steps, the elicitation form, and what the user settled.

use std::collections::VecDeque;

use serde_json::{Value, json};

use super::{DialogAnswer, DialogAsk, HarnessEvent};

pub(crate) const OTHER: &str = "Other";

/// One ask's dialogs, shown one at a time, with the adapter's own tag for its reply.
pub(crate) struct Ask<R> {
    pub(crate) reply: R,
    steps: VecDeque<Step>,
    answers: Vec<(String, Value)>,
}

/// What an agent's request leads to: an ask to show, or an answer at once with a notice.
pub(crate) enum Parsed<R> {
    Ask(Ask<R>),
    Refused { line: Value, notice: String },
}

impl<R> Ask<R> {
    pub(crate) fn new(reply: R, steps: VecDeque<Step>) -> Self {
        Ask {
            reply,
            steps,
            answers: vec![],
        }
    }
}

pub(crate) struct Step {
    pub(crate) title: String,
    pub(crate) ask: DialogAsk,
    pub(crate) field: Field,
}

pub(crate) enum Field {
    /// Each option's value; the pick settles the ask.
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
    /// A link the user opens: settles as "accept" after Open, "decline" after Decline.
    Link,
}

#[derive(Clone, Copy)]
pub(crate) enum Number {
    Integer,
    Float,
}

/// What the user settled an ask with.
pub(crate) enum Settled {
    Cancelled,
    /// One option's value, from a decision or a link.
    Picked(Value),
    /// Each step's answer by key, in the order asked.
    Answers(Vec<(String, Value)>),
}

/// The events an answer leads to, and the ask it settled with its reply tag.
pub(crate) struct Answered<R> {
    pub(crate) events: Vec<HarnessEvent>,
    pub(crate) settled: Option<(R, Settled)>,
}

/// The pane's asks: the one open and those that wait.
pub(crate) struct Asks<R> {
    open: Option<Ask<R>>,
    waiting: VecDeque<Ask<R>>,
    /// How many dialogs went out, so each one's id is new and a late answer to an old one does nothing.
    shown: u64,
}

impl<R> Default for Asks<R> {
    fn default() -> Self {
        Asks {
            open: None,
            waiting: VecDeque::new(),
            shown: 0,
        }
    }
}

impl<R> Asks<R> {
    /// Opens the ask, or queues it behind the open one.
    pub(crate) fn push(&mut self, ask: Ask<R>) -> Vec<HarnessEvent> {
        if self.open.is_some() {
            self.waiting.push_back(ask);
            return vec![];
        }
        self.open = Some(ask);
        self.show()
    }

    pub(crate) fn answer(&mut self, id: &str, answer: &DialogAnswer) -> Answered<R> {
        let nothing = || Answered {
            events: vec![],
            settled: None,
        };
        if id != self.shown.to_string() {
            return nothing();
        }
        let Some(mut ask) = self.open.take() else {
            return nothing();
        };
        let Some(step) = ask.steps.pop_front() else {
            return nothing();
        };
        let settled = match step.take(answer) {
            Taken::Cancel => Some(Settled::Cancelled),
            Taken::Picked(value) => Some(Settled::Picked(value)),
            Taken::Value(key, value) => {
                ask.answers.push((key, value));
                None
            }
            Taken::Next(next) => {
                ask.steps.push_front(next);
                None
            }
        };
        let settled = match settled {
            Some(settled) => settled,
            None if !ask.steps.is_empty() => {
                self.open = Some(ask);
                let mut events = vec![HarnessEvent::DialogClosed];
                events.extend(self.show());
                return Answered {
                    events,
                    settled: None,
                };
            }
            None => Settled::Answers(std::mem::take(&mut ask.answers)),
        };
        Answered {
            events: self.close(),
            settled: Some((ask.reply, settled)),
        }
    }

    /// The agent withdrew the asks `gone` names: the open one's dialog closes, the waiting ones drop.
    pub(crate) fn withdraw(&mut self, gone: impl Fn(&R) -> bool) -> Vec<HarnessEvent> {
        self.waiting.retain(|ask| !gone(&ask.reply));
        match &self.open {
            Some(ask) if gone(&ask.reply) => self.close(),
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
    Picked(Value),
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
            (Field::Decision(values), DialogAnswer::Value { value }) => at(value)
                .and_then(|at| values.get(at).cloned())
                .map_or(Taken::Cancel, Taken::Picked),
            (Field::Link, DialogAnswer::Confirmed { confirmed }) => {
                Taken::Picked(json!(if *confirmed { "accept" } else { "decline" }))
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

// An enum is a select, an array of enums a multi select, a boolean a confirm, a string or number an input. A field of any other type makes the form unreadable.
pub(crate) fn form_steps(schema: &Value, message: &str) -> Option<VecDeque<Step>> {
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

pub(crate) fn select(
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

pub(crate) fn input(
    title: String,
    placeholder: String,
    key: String,
    number: Option<Number>,
) -> Step {
    Step {
        title,
        ask: DialogAsk::Input { placeholder },
        field: Field::Text { key, number },
    }
}

pub(crate) fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item.as_str().map(String::from))
        .collect()
}
