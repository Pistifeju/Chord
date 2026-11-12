//! Reads an image file the user attached, as the base64 text a harness takes.

use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::Error;

pub fn read_base64(path: &Path) -> Result<String, Error> {
    Ok(STANDARD.encode(std::fs::read(path)?))
}
