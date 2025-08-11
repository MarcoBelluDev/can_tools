//! # can_tools
//!
//! `can_tools` is a collection of useful tool for automotive CAN bus completely developed in Rust
//!
//! ## Features
//! - Parses complete CAN databases from both [.dbc](https://en.wikipedia.org/wiki/DBC_(file_format)) and [.arxml](https://autosar.readthedocs.io/en/latest/basics.html) formats into structured Rust types
//! - Reads complete `.asc` CAN trace file 
//!
//! ## Example
//! ```no_run
//! use can_tools::dbc;
//!
//! fn main() -> Result<(), String> {
//!     // Parse the DBC file
//!     let db = dbc::parse_from_file("path/to/file.dbc")?;
//!
//!     println!("DBC Version: {}", db.version);
//!     println!("Messages: {}", db.messages.len());
//!
//!     // Find a message by name
//!     if let Some(msg) = db.get_message_by_name("Motor_01") {
//!         println!("Message ID: {}", msg.id_hex);
//!         println!("Signals: {}", msg.signals.len());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Data Model
//!
//! ```text
//! ┌───────────────────────────────────────┐
//! │               Database                │
//! │───────────────────────────────────────│
//! │ version: String                       │
//! │ bit_timing: String                    │
//! │ nodes: Vec<Node>                      │
//! │ messages: Vec<Message>                │
//! └───────────────────────┬───────────────┘
//!                         │
//!                         ▼
//! ┌───────────────────────────────────────┐
//! │               Message                 │
//! │───────────────────────────────────────│
//! │ id: u64                               │
//! │ id_hex: String                        │
//! │ name: String                          │
//! │ byte_length: usize                    │
//! │ sender_nodes: Vec<Node>               │
//! │ signals: Vec<Signal>                  │
//! └───────────────────────┬───────────────┘
//!                         │
//!                         ▼
//! ┌───────────────────────────────────────┐
//! │                Signal                 │
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
//! Node: represents a CAN ECU (sender or receiver)
//! ```
//!
//! ## When to Use
//! Use `can_tools` when you need to:
//! - Read `.dbc` or `.arxml` CAN databases in Rust
//! - Read `.asc` CAN trace files in Rust
//! - Inspect messages, signals, and value tables
//!
//! ## Related Standards
//! - **DBC** and **ARXML** formats are de-facto standard for describing CAN messages/signals
//! - **ASC** format is the most used to save CAN traces
//! - Compatible with tools like Vector CANdb++, CANalyzer, or open-source alternatives
//!
//! ## License
//! Licensed under the [MIT License](https://opensource.org/licenses/MIT).

pub mod arxml;
pub mod dbc;
pub mod asc;
#[doc(hidden)] pub mod types;

// Top-level re-exports (appear under Crate Items → Structs)
#[doc(inline)]
pub use crate::types::{
    abs_time::AbsoluteTime,
    canframe::CanFrame,
    canlog::CanLog,
    database::Database,
    message::Message,
    node::Node,
    signal::Signal,
    siglog::SigLog,
};