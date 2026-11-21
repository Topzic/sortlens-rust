//! The one error type for the whole application.

use thiserror::Error;

/// Result alias used across Sortlens.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Something the user can be told about, or a bug we can log.
#[derive(Debug, Error)]
pub enum Error {
    #[error("no such photo: {0}")]
    NoSuchPhoto(crate::ids::PhotoId),

    #[error("no such collection: {0}")]
    NoSuchCollection(crate::ids::CollectionId),

    #[error("the folder is not readable: {path}")]
    UnreadableFolder { path: String },

    #[error("the file format is not supported: {0}")]
    UnsupportedFormat(String),

    #[error("the operation is not available yet: {0}")]
    NotYetAvailable(&'static str),

    #[error("{0}")]
    Invalid(String),

    #[error("i/o error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    /// Wrap an I/O error with the path that caused it, which is what the user needs to see.
    pub fn io(path: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io { path: path.into(), source }
    }
}
