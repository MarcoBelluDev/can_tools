//! # can_tools
//!
//! Rust utilities for parsing and modeling **automotive CAN** data.
//!
//! ## Highlights
//! - **DBC parser**: load CAN databases from `.dbc` into a SlotMap-backed [`Database`].
//! - **ASC parser**: read Vector ASCII traces (`.asc`) into a decoupled [`CanLog`].
//! - **Stable keys**: Nodes/Messages/Signals use SlotMap keys that remain valid across reordering.
//! - **Ordered iteration**: `Database::iter_*()` respects order vectors; use `sort_*_by_name()` to present alphabetically.
//! - **Fast lookups**: normalized helpers (`get_message_by_id/_hex/_name`, `get_node_by_name`, `get_signal_by_name`).
//! - **Signal decoding**: `SignalDB::compile_inline`, `extract_raw_*`, and `to_sigframe`.
//!
//! _Crate docs refreshed: 2025-08-22_.
//!

pub mod arxml;
pub mod asc;
pub mod dbc;
#[doc(hidden)]
pub mod types;

// Top-level re-exports (appear under Crate Items → Structs)
#[doc(inline)]
pub use crate::types::{
    absolute_time::AbsoluteTime,
    canlog::{CanFrame, CanLog, MessageLog, SignalLog},
    database::{BusType, Database, MessageKey, NodeKey, Present, SignalKey},
    message_db::{IdFormat, MessageDB, MuxInfo, MuxRole, MuxSelector},
    node_db::NodeDB,
    signal_db::SignalDB,
};

// Helper re-export for UI convenience
pub use crate::types::canlog::resolve_message_signals;
