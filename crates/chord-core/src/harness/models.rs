use std::collections::BTreeMap;
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;

use super::process::{Output, Process};
use super::{Harness, HarnessEvent, ModelOption, claude, codex, login_shell, pi};
use crate::{Error, Store};

const WAIT: Duration = Duration::from_secs(20);

type ListModels = fn() -> Result<Vec<ModelOption>, Error>;

pub(super) enum Step<T = Vec<ModelOption>> {
    Write(Vec<Value>),
    Done(T),
    Wait,
}

pub(super) fn listed(event: HarnessEvent) -> Step {
    match event {
        HarnessEvent::Models { models, .. } => Step::Done(models),
        _ => Step::Wait,
    }
}

pub(super) fn ask(
    harness: &Harness,
    args: &[&str],
    first: &[Value],
    step: impl FnMut(&Value) -> Step + Send + 'static,
) -> Result<Vec<ModelOption>, Error> {
    answer(harness, args, first, step)?
        .ok_or_else(|| Error::Message(format!("{} listed no models", harness.title)))
}

pub(super) fn answer<T: Send + 'static>(
    harness: &Harness,
    args: &[&str],
    first: &[Value],
    mut step: impl FnMut(&Value) -> Step<T> + Send + 'static,
) -> Result<Option<T>, Error> {
    let mut command = login_shell(harness.binary, args);
    command.current_dir(std::env::temp_dir());
    let (found, answers) = mpsc::channel();
    let process = Process::start(command, move |output, stdin| match output {
        Output::Line(line) => {
            let Ok(record) = serde_json::from_slice::<Value>(line) else {
                return;
            };
            match step(&record) {
                Step::Write(lines) => {
                    for line in lines {
                        let _ = stdin.write(&line);
                    }
                }
                Step::Done(answer) => {
                    let _ = found.send(Some(answer));
                }
                Step::Wait => {}
            }
        }
        Output::Closed { .. } => {
            let _ = found.send(None);
        }
    })
    .map_err(|error| Error::start_failed(harness.binary, harness.install, error))?;
    for line in first {
        process.write(line)?;
    }
    let answer = answers.recv_timeout(WAIT).ok().flatten();
    drop(process);
    Ok(answer)
}

pub fn refresh(store: &Store) -> Result<BTreeMap<String, Vec<ModelOption>>, Error> {
    let readers: [(&str, ListModels); 3] = [
        (pi::NAME, pi::models),
        (codex::NAME, codex::models),
        (claude::NAME, claude::models),
    ];
    let lists: Vec<(&str, Vec<ModelOption>)> = std::thread::scope(|scope| {
        let reading: Vec<_> = readers
            .into_iter()
            .map(|(name, read)| (name, scope.spawn(read)))
            .collect();
        reading
            .into_iter()
            .filter_map(|(name, reading)| Some((name, reading.join().ok()?.ok()?)))
            .filter(|(_, models)| !models.is_empty())
            .collect()
    });
    store.update(|state| {
        for (name, models) in lists {
            state.models.insert(name.into(), models);
        }
    })?;
    Ok(store.get().models)
}
