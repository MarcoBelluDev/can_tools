//! # can_tools
//!
//! Utility crate for working with **automotive CAN** data in Rust.
//!
//! ## Highlights
//! - **DBC parser**: load CAN databases from `.dbc` files.
//! - **ASC trace parser**: read `.asc` logs and build a [`CanLog`] with frames, messages and signals.
//! - **Decoupled model**:
//!   - [`CanFrame`]: timing + channel + direction + pointer to [`MessageLog`].
//!   - [`MessageLog`]: identifier, name, payload, comment and the list of signal indices.
//!   - [`SignalLog`]: aggregated time-series for a single decoded signal with `values: Vec<[f64; 2]>`
//!     where each pair is `[timestamp, value]` in seconds.
//! - Helpers like [`resolve_message_signals`] and convenience conversion via [`SignalLog::value_text_at`].
//!
//! ## Example
//! Parse a DBC, then an ASC trace and walk the first frame and its signals:
//!
//! ```no_run
//! use can_tools::{dbc, asc, CanLog, CanFrame, resolve_message_signals};
//! use std::collections::HashMap;
//!
//! # fn main() -> Result<(), String> {
//! // Load a DBC (optional but recommended to decode signals)
//! let db = dbc::parse::from_file("path/to/file.dbc")?;
//!
//! // Parse a `.asc` trace; pass a map of channel→Database if you have more than one DB
//! let mut log = CanLog::default();
//! let mut last_by_id_ch: HashMap<String, usize> = HashMap::new();
//! let mut chart_by_key: HashMap<String, usize> = HashMap::new();
//! let mut dbs = HashMap::new();
//! dbs.insert(1u8, db);
//! asc::parse::from_file("path/to/file.asc", &dbs)?;
//!
//! // Read first frame
//! let frame: &CanFrame = &log.can_frames[0];
//! let msg = &log.messages[frame.message];
//! println!("id={} name={} len={}", msg.id, msg.name, msg.byte_length);
//!
//! // Iterate signals of that message and get value at the frame timestamp
//! for sig in resolve_message_signals(&log, frame.message) {
//!     if let Some((v, txt)) = sig.value_text_at(frame.timestamp as f64) {
//!         println!("{} = {} ({})", sig.name, v, txt);
//!     }
//! }
//! # Ok(()) }
//! ```
//!
//! ## Data Model
//!
//! ```text
//! ┌───────────────────────────────────────┐
//! │ Database                              │
//! │───────────────────────────────────────│
//! │ version: String                       │
//! │ bit_timing: String                    │
//! │ nodes: Vec<Node>                      │
//! │ messages: Vec<Message>                │
//! └───────────────────────┬───────────────┘
//!                         │
//!                         ▼
//! ┌───────────────────────────────────────┐
//! │ MessageDB                             │
//! │───────────────────────────────────────│
//! │ id: u64                               │
//! │ id_hex: String                        │
//! │ name: String                          │
//! │ byte_length: u16                      │
//! │ sender_nodes: Vec<Node>               │
//! │ signals: Vec<Signal>                  │
//! └───────────────────────┬───────────────┘
//!                         │
//!                         ▼
//! ┌───────────────────────────────────────┐
//! │ SignalDB                              │
//! │───────────────────────────────────────│
//! │ name: String                          │
//! │ bit_start: usize                      │
//! │ bit_length: usize                     │
//! │ endian: usize                         │
//! │ sign: usize                           │
//! │ factor: f64                           │
//! │ offset: f64                           │
//! │ min: f64                              │
//! │ max: f64                              │
//! │ unit_of_measurement: String           │
//! │ receiver_nodes: Vec<Node>             │
//! │ value_table: HashMap<i32, String>     │
//! └───────────────────────────────────────┘
//!
//! (Node: represents a CAN ECU, as sender or receiver.)
//! ```
//!
//! ## When to Use
//! Use `can_tools` when you need to:
//! - Read `.dbc` or `.arxml` CAN databases in Rust
//! - Read `.asc` CAN trace files in Rust
//! - Inspect messages, signals, and value tables
//!
//! ## Related Standards
//! - **DBC** and **ARXML** formats are de-facto standards for describing CAN messages/signals
//! - **ASC** format is commonly used to store CAN traces
//! - Compatible with tools like Vector CANdb++, CANalyzer, or open-source alternatives
//!
//! ## License
//! Licensed under the MIT License.


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
    database::{Database, MessageDB, NodeDB, SignalDB},
};

// Helper re-export for UI convenience
pub use crate::types::canlog::resolve_message_signals;
