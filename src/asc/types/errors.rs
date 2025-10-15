use std::io;
use thiserror::Error;

/// Errors produced while parsing a `.asc` trace file.
#[derive(Debug, Error)]
pub enum AscParseError {
    #[error("Not a valid .asc file: {path}")]
    InvalidExtension { path: String },
    #[error("Failed to open '{path}': {source}")]
    OpenFile {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("Failed while reading '{path}': {source}")]
    Read {
        path: String,
        #[source]
        source: io::Error,
    },
}
