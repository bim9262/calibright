use thiserror::Error;

/// Result type returned from functions that can have our `Error`s.
pub type Result<T, E = CalibrightError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CalibrightError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    InvalidRegex(#[from] regex::Error),

    #[cfg(feature = "watch")]
    #[error("{0}")]
    Notify(#[from] notify::Error),

    #[error("{0}")]
    DBus(#[from] zbus::Error),

    #[error("{0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("No matching devices exist")]
    NoDevices,

    #[error("Invalid scale parameters: {low} > {high}")]
    InvalidScaleParameters { low: f64, high: f64 },

    #[error("{0}")]
    Other(String),

    #[error("Unknown error")]
    Unknown,
}
