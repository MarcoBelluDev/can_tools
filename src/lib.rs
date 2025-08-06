//! # can_tools
//!
//! `can_tools` is a Rust library for parsing [DBC files](https://en.wikipedia.org/wiki/DBC_(file_format))
//! used in automotive applications to describe CAN network messages and signals.
//!
//! ## Features
//! - Parses complete `.dbc` files into structured Rust types
//! - Reads **messages**, **signals**, **nodes**, **comments**, and **value tables**
//! - Case-insensitive search utilities for messages, signals, and nodes
//! - Simple API: one call to [`parse`](crate::file::dbc::parse) produces a ready-to-use [`Database`](crate::models::database::Database)
//!
//! ## Example
//! ```no_run
//! use can_tools::file::dbc::parse;
//!
//! fn main() -> Result<(), String> {
//!     // Parse the DBC file
//!     let db = parse("path/to/file.dbc")?;
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
//! ## Main Types
//!
//! - [`Database`](crate::models::database::Database):
//!   Holds the parsed DBC file structure, including version, bit timing,
//!   all [`Node`](crate::models::node::Node)s, and [`Message`](crate::models::message::Message)s.
//!
//! - [`Message`](crate::models::message::Message):
//!   Represents a CAN message. Contains message ID, name, sender nodes, and its list of [`Signal`](crate::models::signal::Signal)s.
//!
//! - [`Signal`](crate::models::signal::Signal):
//!   Represents a data field within a CAN message, including bit position, length,
//!   scaling factor, unit, receiver nodes, and optional value descriptions.
//!
//! - [`Node`](crate::models::node::Node):
//!   Represents a CAN network node (ECU) that can send or receive messages.
//!
//! ## When to Use
//! Use `can_tools` when you need to:
//! - Read `.dbc` files in Rust
//! - Inspect messages, signals, and value tables
//! - Integrate CAN signal definitions into automotive tools or simulations
//!
//! ## Related Standards
//! - **DBC** format is a de-facto standard for describing CAN messages/signals
//! - Compatible with tools like Vector CANdb++, CANalyzer, or open-source alternatives
//!
//! ## License
//! Licensed under the [MIT License](https://opensource.org/licenses/MIT).

pub mod file;
pub mod models;
