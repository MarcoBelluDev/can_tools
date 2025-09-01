//! # can_tools
//!
//! Rust utilities for parsing and modeling **automotive CAN** data.
//!
//! ## Highlights
//! - **DBC parser**: load CAN databases from `.dbc` into a SlotMap-backed
//!   [`DatabaseDBC`](crate::dbc::types::database::DatabaseDBC).
//! - **ASC parser**: read Vector ASCII traces (`.asc`) into a decoupled
//!   [`CanLog`](crate::asc::types::canlog::CanLog).
//! - **Stable keys**: Nodes/Messages/Signals use SlotMap keys that remain valid across reordering.
//! - **Ordered iteration**: `DatabaseDBC::iter_*()` respects order vectors; use `sort_*_by_name()` to present alphabetically.
//! - **Fast lookups**: normalized helpers (`get_message_by_id/_hex/_name`, `get_node_by_name`, `get_signal_by_name`).
//! - **Signal decoding**: `SignalDBC::compile_inline`, `extract_raw_*`, and `to_sigframe`.
//!
//! _Crate docs refreshed: 2025-08-22_.
//!

pub mod arxml;
pub mod asc;
pub mod dbc;

// Helper re-export for UI convenience
pub use crate::asc::types::canlog::resolve_message_signals;
