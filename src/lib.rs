//! # can_tools
//!
//! Rust utilities for parsing and modeling **automotive CAN** data.
//! The default feature set enables both the `.dbc` database parser and the `.asc`
//! trace parser. Use Cargo feature flags to pick only the pieces you need.
//!
//! ## Highlights
//! - DBC parser: loads CAN databases from `.dbc` into a SlotMap-backed
//!   [`DatabaseDBC`](crate::dbc::types::database::DatabaseDBC). The reader streams
//!   the file line by line, decodes Windows‑1252, and applies a single‑pass
//!   transliteration for a few special characters.
//! - ASC parser: reads Vector ASCII traces (`.asc`) into a decoupled
//!   [`CanLog`](crate::asc::types::canlog::CanLog). It keeps per `(id, channel)`
//!   only the index of the most recent frame and formats absolute timestamps
//!   with a lightweight formatter.
//!
//! Docs updated: 2025-10-23
//!
#[cfg(feature = "asc")]
pub mod asc;
#[cfg(feature = "dbc")]
pub mod dbc;

#[cfg(feature = "asc")]
pub use crate::asc::types::errors::AscParseError;
#[cfg(feature = "dbc")]
pub use crate::dbc::types::errors::{DatabaseError, DbcParseError, MessageLayoutError};

// Helper re-export for UI convenience (only when `asc` is enabled)
#[cfg(feature = "asc")]
pub use crate::asc::types::canlog::resolve_message_signals;
