use std::io;
use thiserror::Error;

use crate::dbc::types::database::{MessageKey, NodeKey, SignalKey};

/// Errors produced while parsing a `.dbc` file.
#[derive(Debug, Error)]
pub enum DbcParseError {
    #[error("Not a valid .dbc file: {path}")]
    InvalidExtension { path: String },
    #[error("Failed to open '{path}'. \nError: {source}")]
    OpenFile {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("Failed while reading '{path}'. \nError: {source}")]
    Read {
        path: String,
        #[source]
        source: io::Error,
    },
}

/// Errors produced while verifying that a signal fits a CAN frame layout.
#[derive(Debug, Error)]
pub enum MessageLayoutError {
    #[error("Signal Bit Length cannot be zero")]
    ZeroBitLength,
    #[error(
        "Out of bounds (Intel)! \nSignal end bit = {end} \nMessage total bits = {total_bits} (bytes={dlc})"
    )]
    IntelOutOfBounds {
        end: usize,
        total_bits: usize,
        dlc: u16,
    },
    #[error(
        "Out of bounds (Motorola)! \nSignal linearized  start = {start} \nMessage total bits = {total_bits} (bytes={dlc})"
    )]
    MotorolaStartOutOfBounds {
        start: usize,
        total_bits: usize,
        dlc: u16,
    },
    #[error("Out of bounds (Motorola): Signal linearized  end = {end} < 0 (bytes={dlc})")]
    MotorolaEndOutOfBounds { end: isize, dlc: u16 },
}

/// Errors returned by high-level operations on [`DatabaseDBC`](crate::dbc::types::database::DatabaseDBC).
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Node '{name}' already exists")]
    NodeAlreadyExists { name: String },
    #[error("Node not found for key {node_key:?}")]
    NodeMissing { node_key: NodeKey },
    #[error("Message '{name}' already exists")]
    MessageAlreadyExists { name: String },
    #[error("Message ID {id_hex} already assigned to an existing message")]
    MessageIdAlreadyAssigned { id_hex: String },
    #[error("Message not found for key {message_key:?}")]
    MessageMissing { message_key: MessageKey },
    #[error("Signal not found for key {signal_key:?}")]
    SignalMissing { signal_key: SignalKey },
    #[error("Signal '{signal}' is already associated with {associated_with}")]
    SignalAlreadyAssociated {
        signal: String,
        associated_with: String,
    },
    #[error("Value table entry {entry} already exists for signal '{signal}'")]
    ValueTableEntryAlreadyExists { signal: String, entry: i32 },
    #[error("Value table entry {entry} is not defined for signal '{signal}'")]
    ValueTableEntryMissing { signal: String, entry: i32 },
    #[error("Value table entry for signal '{signal}' cannot have an empty description")]
    ValueTableEntryDescriptionEmpty { signal: String },
    #[error("Message missing while updating multiplexor relation.")]
    MessageMissingDuringMultiplexing,
    #[error("Database is in an inconsistent state: {details}")]
    InconsistentState { details: &'static str },
    #[error(transparent)]
    Layout(#[from] MessageLayoutError),
}
