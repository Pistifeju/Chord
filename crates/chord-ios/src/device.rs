use chord_core::{Error, login_shell};
use std::path::Path;

use crate::console::{self, ConsoleTone};
use crate::process::{self, xcrun};

const LAUNCHED: &str = "Launched application with";

pub(crate) fn install(udid: &str, app: &Path, register: &dyn Fn(u32)) -> Result<(), Error> {
    xcrun(
        &[
            "devicectl",
            "device",
            "install",
            "app",
            "--device",
            udid,
            &app.to_string_lossy(),
        ],
        register,
    )
    .map(drop)
}

pub(crate) fn launch(
    udid: &str,
    bundle_id: &str,
    register: &dyn Fn(u32),
    launched: &dyn Fn(),
    console: &dyn Fn(String, ConsoleTone),
) -> Result<Option<i32>, Error> {
    let command = login_shell(
        "xcrun",
        &[
            "devicectl",
            "device",
            "process",
            "launch",
            "--device",
            udid,
            "--terminate-existing",
            "--console",
            bundle_id,
        ],
    );
    let (mut child, reader) = process::merged(command, None)?;
    register(child.id());
    let lines = process::read_lines(reader);
    let pumped = console::pump(
        &lines,
        None,
        Some(&|text| text.contains(LAUNCHED)),
        launched,
        console,
        || {},
    );
    let status = child.wait()?;
    if pumped.up {
        return Ok(status.code());
    }
    let output = pumped.output;
    let output = output.trim();
    Err(Error::Message(if output.is_empty() {
        format!("The app did not launch: devicectl exited with {status}")
    } else {
        output.to_string()
    }))
}
