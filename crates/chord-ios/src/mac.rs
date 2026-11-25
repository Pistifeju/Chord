use std::path::Path;
use std::process::Command;

use chord_core::Error;

use crate::console::{self, ConsoleTone};
use crate::plist;
use crate::process;

pub(crate) fn launch(
    app: &Path,
    register: &dyn Fn(u32),
    launched: &dyn Fn(),
    console: &dyn Fn(String, ConsoleTone),
) -> Result<Option<i32>, Error> {
    let executable = plist::value(
        &app.join("Contents/Info.plist"),
        "CFBundleExecutable",
        register,
    )?;
    let mut command = Command::new(app.join("Contents/MacOS").join(executable));
    command
        .current_dir(app.parent().unwrap_or(app))
        .env("OS_ACTIVITY_DT_MODE", "YES");
    let (mut child, terminal) = process::merged_on_terminal(command, None)?;
    register(child.id());
    let lines = process::read_lines(terminal);
    console::pump(&lines, None, None, launched, console, || {});
    Ok(child.wait()?.code())
}
