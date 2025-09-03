//! # can_tools
//!
//! Rust utilities for parsing and modeling **automotive CAN** data.
//! Default derive of this library include .dbc parser, .asc parser and .arxml parser
//! Use feature flag to use only a specific feature
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
//! - ARXML parser: Feature is currently in development and not ready yet.
//!   Please, do not use it.
//!
//! Docs updated: 2025-09-03.
//!
#[cfg(feature = "arxml")]
pub mod arxml;
#[cfg(feature = "asc")]
pub mod asc;
#[cfg(feature = "dbc")]
pub mod dbc;

// Helper re-export for UI convenience (only when `asc` is enabled)
#[cfg(feature = "asc")]
pub use crate::asc::types::canlog::resolve_message_signals;
