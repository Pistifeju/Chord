use super::{DialogAnswer, DialogAsk, HarnessEvent};

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

pub(crate) fn pick<T>(entries: Vec<T>, answer: &DialogAnswer) -> Option<T> {
    let DialogAnswer::Value { value } = answer else {
        return None;
    };
    let (number, _) = value.split_once('.')?;
    let at = number.parse::<usize>().ok()?.checked_sub(1)?;
    entries.into_iter().nth(at)
}
