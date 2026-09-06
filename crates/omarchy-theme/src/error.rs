use std::fmt;
use std::io;

/// Errors produced while resolving an Omarchy theme.
#[derive(Debug)]
pub enum Error {
    /// An I/O failure while reading theme state.
    Io(io::Error),
    /// No theme is currently staged (`current/theme` missing or unreadable).
    NoTheme,
    /// A required palette key could not be resolved even after the fallback
    /// cascade (only possible for malformed themes).
    MissingKey(String),
    /// A value could not be parsed as expected.
    Parse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io error: {e}"),
            Error::NoTheme => write!(f, "no omarchy theme is currently staged"),
            Error::MissingKey(k) => write!(f, "palette key `{k}` could not be resolved"),
            Error::Parse(m) => write!(f, "parse error: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
