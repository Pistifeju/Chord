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

impl std::error::Error for Error {}
