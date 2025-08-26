//! # asc
//!
//! Parsing utilities for Vector **ASC** trace files.
//! Use `asc::parse::from_file(...)` to create a `CanLog`.
//! Helper routines are in `asc::support` (absolute-time header, line parsing, utilities).
//! _Module docs refreshed_.

pub mod parse;
pub(crate) mod support;
