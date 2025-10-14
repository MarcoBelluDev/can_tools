//! # dbc
//!
//! Parsing utilities for **DBC** CAN database files.
//! Use `dbc::parse::from_file(...)` to build a SlotMap-backed `Database`.
//! Supporting functions live under `dbc::support` (token normalization, ID parsing, etc.).
//! _Module docs refreshed_.

pub(crate) mod core;
pub mod create;
pub mod parse;
pub mod types;
