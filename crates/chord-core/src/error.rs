//! The one error type core functions return.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    MissingTool { tool: String, install: String },
    Message(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MissingTool { tool, install } => {
                write!(f, "{tool} is not installed. Install it with: {install}")
            }
            Error::Message(text) => f.write_str(text),
        }
    }
}

impl Error {
    /// A tool that did not start: missing from the PATH, with its install step, or failed for another reason.
    pub fn start_failed(tool: &str, install: &str, error: std::io::Error) -> Error {
        match error.kind() {
            std::io::ErrorKind::NotFound => Error::MissingTool {
                tool: tool.into(),
                install: install.into(),
            },
            _ => Error::Message(format!("{tool} failed to start: {error}")),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Message(error.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Message(error.to_string())
    }
}
