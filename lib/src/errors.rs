use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

pub use std::error::Error as StdError;

/// Result type returned from functions that can have our `Error`s.
pub type Result<T, E = Error> = std::result::Result<T, E>;

type ErrorMsg = Cow<'static, str>;

/// Error type
#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: Option<ErrorMsg>,
    pub cause: Option<Arc<dyn StdError + Send + Sync + 'static>>,
}

/// A set of errors that can occur during the runtime
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Config,
    Other,
}

impl Error {
    pub fn new<T: Into<ErrorMsg>>(message: T) -> Self {
        Self {
            kind: ErrorKind::Other,
            message: Some(message.into()),
            cause: None,
        }
    }
}

pub trait ResultExt<T> {
    fn error<M: Into<ErrorMsg>>(self, message: M) -> Result<T>;
    fn or_error<M: Into<ErrorMsg>, F: FnOnce() -> M>(self, f: F) -> Result<T>;
    fn config_error(self) -> Result<T>;
}

impl<T, E: StdError + Send + Sync + 'static> ResultExt<T> for Result<T, E> {
    fn error<M: Into<ErrorMsg>>(self, message: M) -> Result<T> {
        self.map_err(|e| Error {
            kind: ErrorKind::Other,
            message: Some(message.into()),
            cause: Some(Arc::new(e)),
        })
    }

    fn or_error<M: Into<ErrorMsg>, F: FnOnce() -> M>(self, f: F) -> Result<T> {
        self.map_err(|e| Error {
            kind: ErrorKind::Other,
            message: Some(f().into()),
            cause: Some(Arc::new(e)),
        })
    }

    fn config_error(self) -> Result<T> {
        self.map_err(|e| Error {
            kind: ErrorKind::Config,
            message: None,
            cause: Some(Arc::new(e)),
        })
    }
}

pub trait OptionExt<T> {
    fn error<M: Into<ErrorMsg>>(self, message: M) -> Result<T>;
    fn or_error<M: Into<ErrorMsg>, F: FnOnce() -> M>(self, f: F) -> Result<T>;
    fn config_error(self) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn error<M: Into<ErrorMsg>>(self, message: M) -> Result<T> {
        self.ok_or_else(|| Error {
            kind: ErrorKind::Other,
            message: Some(message.into()),
            cause: None,
        })
    }

    fn or_error<M: Into<ErrorMsg>, F: FnOnce() -> M>(self, f: F) -> Result<T> {
        self.ok_or_else(|| Error {
            kind: ErrorKind::Other,
            message: Some(f().into()),
            cause: None,
        })
    }

    fn config_error(self) -> Result<T> {
        self.ok_or(Error {
            kind: ErrorKind::Config,
            message: None,
            cause: None,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.message.as_deref().unwrap_or("Error"))?;
        if let Some(cause) = &self.cause {
            write!(f, ". (Cause: {cause})")?;
        }

        Ok(())
    }
}

impl From<Error> for std::io::Error {
    fn from(err: Error) -> Self {
        Self::new(std::io::ErrorKind::Other, err.to_string())
    }
}

impl StdError for Error {}

pub trait ToSerdeError<T> {
    fn serde_error<E: serde::de::Error>(self) -> Result<T, E>;
}

impl<T, F> ToSerdeError<T> for Result<T, F>
where
    F: fmt::Display,
{
    fn serde_error<E: serde::de::Error>(self) -> Result<T, E> {
        self.map_err(E::custom)
    }
}

pub struct BoxErrorWrapper(pub Box<dyn StdError + Send + Sync + 'static>);

impl fmt::Debug for BoxErrorWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for BoxErrorWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl StdError for BoxErrorWrapper {}
