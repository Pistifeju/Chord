use std::path::Path;
use std::process::Command;

use chord_core::Error;

use crate::process;

pub(crate) const PLUTIL: &str = "/usr/bin/plutil";

pub(crate) fn app_value(app: &Path, key: &str, register: &dyn Fn(u32)) -> Result<String, Error> {
    value(&app.join("Info.plist"), key, register)
}

pub(crate) fn value(file: &Path, key: &str, register: &dyn Fn(u32)) -> Result<String, Error> {
    let mut command = Command::new(PLUTIL);
    command.args(["-extract", key, "raw", "-o", "-"]).arg(file);
    let output = process::captured(command, register)?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !value.is_empty() {
        Ok(value)
    } else {
        Err(Error::Message(format!("No {key} in {}", file.display())))
    }
}
