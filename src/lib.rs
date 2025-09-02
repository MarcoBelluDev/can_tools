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
//! - **Signal decoding**: `SignalDBC::compile_inline`, `extract_raw_*`.
//!   Conversion to `SignalLog` lives in `asc::core::signal_conversion::to_sigframe` (feature `asc`).
//!
//! _Crate docs refreshed: 2025-08-22_.
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
