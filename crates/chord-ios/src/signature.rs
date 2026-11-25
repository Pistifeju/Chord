use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use sha1::{Digest, Sha1};

use crate::process;

pub(crate) const CODESIGN: &str = "/usr/bin/codesign";

pub(crate) fn identity(app: &Path) -> Result<Option<String>, String> {
    static N: AtomicUsize = AtomicUsize::new(0);
    let folder = std::env::temp_dir().join(format!(
        "chord-signature-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let mut command = Command::new(CODESIGN);
    command
        .arg("-d")
        .arg(format!(
            "--extract-certificates={}",
            folder.join("cert").display()
        ))
        .arg(app);
    let read = process::checked(command, "codesign");
    let leaf = fs::read(folder.join("cert0"));
    let _ = fs::remove_dir_all(&folder);
    read?;
    Ok(leaf.ok().map(|leaf| format!("{:X}", Sha1::digest(leaf))))
}

pub(crate) fn sign(file: &Path, identity: &str) -> Result<(), String> {
    let mut command = Command::new(CODESIGN);
    command.args(["--force", "--sign", identity]).arg(file);
    process::checked(command, "codesign").map(drop)
}
