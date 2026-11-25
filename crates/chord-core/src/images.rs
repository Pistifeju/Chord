use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::Error;

pub fn read_base64(path: &Path) -> Result<String, Error> {
    Ok(STANDARD.encode(std::fs::read(path)?))
}

pub fn mime_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}
