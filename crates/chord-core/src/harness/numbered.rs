//! The numbered select Chord opens for a session command: its options, each led by its number, and the entry a pick names.

use super::{DialogAnswer, DialogAsk, HarnessEvent};

/// The select of `labels`, numbered from 1; `none` as a notice when there is no label.
pub(crate) fn select(id: &str, title: &str, labels: &[String], none: &str) -> HarnessEvent {
    if labels.is_empty() {
        return HarnessEvent::Notice { text: none.into() };
    }
    let options = labels
        .iter()
        .enumerate()
        .map(|(at, label)| format!("{}. {label}", at + 1))
        .collect();
    HarnessEvent::Dialog {
        id: id.into(),
        title: title.into(),
        ask: DialogAsk::Select {
            options,
            current: None,
            detail: None,
            descriptions: vec![],
            multi: false,
        },
    }
}

/// The entry of `entries` the answer's number names; none after a cancel.
pub(crate) fn pick<T>(entries: Vec<T>, answer: &DialogAnswer) -> Option<T> {
    let DialogAnswer::Value { value } = answer else {
        return None;
    };
    let (number, _) = value.split_once('.')?;
    let at = number.parse::<usize>().ok()?.checked_sub(1)?;
    entries.into_iter().nth(at)
}
