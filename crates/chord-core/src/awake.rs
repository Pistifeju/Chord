use crate::lock;
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::process::{Child, Command};
use std::sync::{Mutex, MutexGuard};

use crate::Error;

#[derive(Default)]
struct Inner {
    enabled: bool,
    working: BTreeSet<String>,
    caffeinate: Option<Child>,
}

pub struct Awake {
    inner: Mutex<Inner>,
}

impl Awake {
    pub fn new(enabled: bool) -> Awake {
        Awake {
            inner: Mutex::new(Inner {
                enabled,
                ..Inner::default()
            }),
        }
    }

    fn inner(&self) -> MutexGuard<'_, Inner> {
        lock(&self.inner)
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<(), Error> {
        if enabled && !caffeinate_on_path() {
            return Err(missing_caffeinate());
        }
        let mut inner = self.inner();
        let before = std::mem::replace(&mut inner.enabled, enabled);
        let synced = inner.sync();
        if synced.is_err() {
            inner.enabled = before;
        }
        synced
    }

    pub fn set_working(&self, pane: &str, working: bool) {
        let mut inner = self.inner();
        if working {
            inner.working.insert(pane.to_string());
        } else {
            inner.working.remove(pane);
        }
        let _ = inner.sync();
    }

    pub fn clear(&self) {
        let mut inner = self.inner();
        inner.working.clear();
        let _ = inner.sync();
    }
}

fn caffeinate_on_path() -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|dir| dir.join("caffeinate").is_file())
    })
}

fn missing_caffeinate() -> Error {
    Error::MissingTool {
        tool: "caffeinate".into(),
        install: "it ships with macOS in /usr/bin; restore it by reinstalling macOS".into(),
    }
}

impl Inner {
    fn sync(&mut self) -> Result<(), Error> {
        if self
            .caffeinate
            .as_mut()
            .is_some_and(|child| child.try_wait().is_ok_and(|exit| exit.is_some()))
        {
            self.caffeinate = None;
        }
        let wanted = self.enabled && !self.working.is_empty();
        match (&mut self.caffeinate, wanted) {
            (None, true) => {
                let child = Command::new("caffeinate")
                    .args(["-i", "-w", &std::process::id().to_string()])
                    .spawn()
                    .map_err(|error| match error.kind() {
                        ErrorKind::NotFound => missing_caffeinate(),
                        _ => Error::Message(format!("caffeinate failed to start: {error}")),
                    })?;
                self.caffeinate = Some(child);
            }
            (Some(child), false) => {
                let _ = child.kill();
                let _ = child.wait();
                self.caffeinate = None;
            }
            _ => {}
        }
        Ok(())
    }
}

impl Drop for Awake {
    fn drop(&mut self) {
        self.inner().working.clear();
        let _ = self.inner().sync();
    }
}
