# can_tools

[![Crates.io](https://img.shields.io/crates/v/can_tools.svg)](https://crates.io/crates/can_tools)
[![Docs.rs](https://img.shields.io/docsrs/can_tools)](https://docs.rs/can_tools)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](#license)

`can_tools` is a Rust library that provides a small, fast toolkit for working with automotive CAN data.
It focuses on **parsing databases** (DBC/ARXML) into well-typed Rust structures and **reading CAN traces** (ASC).

---

## Features

- Parse full CAN **databases** from both
  - [`.dbc`](https://en.wikipedia.org/wiki/DBC_(file_format)) and
  - [`.arxml`](https://autosar.readthedocs.io/en/latest/basics.html)
  into structured Rust types.
- Read complete **`.asc`** CAN trace files.
- Convenient, strongly typed data model with helpful getters.
- No unsafe code; pure Rust.

> If you're building tools around Vector DBC/CANdb++ or AUTOSAR ARXML, this crate gives you a clean, idiomatic Rust API.

---

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
can_tools = "*"
```

> Tip: replace `*` with the latest version shown on crates.io for reproducible builds.

This crate targets Rust **edition 2024**.

---

## Quick Start

### Parse a DBC and inspect messages

```rust,no_run
use can_tools::dbc;

fn main() -> Result<(), String> {
    // Parse the DBC file
    let db = dbc::parse_from_file("path/to/file.dbc")?;

    println!("DBC Version: {}", db.version);
    println!("Messages: {}", db.messages.len());

    // Find a message by name
    if let Some(msg) = db.get_message_by_name("Motor_01") {
        println!("Message ID: {}", msg.id_hex);
        println!("Signals: {}", msg.signals.len());
    }

    Ok(())
}
```

### Parse ARXML (may contain multiple CAN clusters)

```rust,no_run
use can_tools::arxml;

fn main() -> Result<(), String> {
    let dbs = arxml::parse("path/to/file.arxml")?; // -> Vec<can_tools::Database>
    for db in dbs {
        println!("Cluster: {}  Type: {}  Version: {}", db.name, db.bustype, db.version);
        println!("Messages: {}", db.messages.len());
    }
    Ok(())
}
```

### Read a CAN trace (`.asc`)

```rust,no_run
use can_tools::asc;

fn main() -> Result<(), String> {
    // If you have a database for symbol names, pass Some(&db). Otherwise pass None.
    let log = asc::parse_from_file("path/to/trace.asc", None)?; // -> can_tools::CanLog

    println!("Frames read: {}", log.all_frame.len());
    println!("Unique (id, channel): {}", log.last_id_chn_frame.len());

    Ok(())
}
```

---

## Data Model (Core Types)

```text
┌───────────────────────────────────────┐
│               Database                │
│───────────────────────────────────────│
│ name: String                          │
│ bustype: String                       │
│ baudrate: usize                       │
│ baudrate_canfd: usize                 │
│ version: String                       │
│ nodes: Vec<Node>                      │
│ messages: Vec<Message>                │
└───────────────────────┬───────────────┘
                        │
                        ▼
┌───────────────────────────────────────┐
│               Message                 │
│───────────────────────────────────────│
│ id: u64                               │
│ id_hex: String                        │
│ name: String                          │
│ byte_length: usize                    │
│ sender_nodes: Vec<Node>               │
│ signals: Vec<Signal>                  │
│ comment: String                       │
└───────────────────────┬───────────────┘
                        │
                        ▼
┌───────────────────────────────────────┐
│                Signal                 │
│───────────────────────────────────────│
│ name: String                          │
│ bit_start: usize                      │
│ bit_length: usize                     │
│ endian: usize                         │
│ sign: usize                           │
│ factor: f64                           │
│ offset: f64                           │
│ min: f64                              │
│ max: f64                              │
│ unit_of_measurement: String           │
│ receiver_nodes: Vec<Node>             │
│ value_table: HashMap<i32, String>     │
│ comment: String                       │
└───────────────────────────────────────┘

Node: represents a CAN ECU (sender or receiver)
```

The crate re-exports the most important types at the top level for convenience:
`Database`, `Message`, `Signal`, `Node`, `CanLog`, `CanFrame`, `AbsoluteTime`, `SigLog`, `MsgLog`.

---

## When to Use

Use `can_tools` when you need to:

- Load and analyze **DBC** or **ARXML** CAN databases in Rust.
- Read and process **ASC** CAN trace files.
- Inspect messages, signals, senders/receivers, and enumerated **value tables**.
- Map raw frames to symbolic names using your database.

---

## API Highlights

- `dbc::parse_from_file(path) -> Result<Database, String>`
- `arxml::parse(path) -> Result<Vec<Database>, String>`
- `asc::parse_from_file(path, db_opt) -> Result<CanLog, String>`
- `Database::get_message_by_name(&self, name) -> Option<&Message>`
- `Message::get_signal_by_name(&self, name) -> Option<&Signal>`

See the full docs on **docs.rs** for more.

---

## Related Standards & Tools

- **DBC** and **ARXML** are widely used to describe signals/messages for CAN and CAN-FD.
- **ASC** is a common ASCII trace format for log files.
- Compatible with workflows involving Vector CANdb++, CANalyzer/CANoe, and many open-source tools.

---

## Roadmap (short)
- Expanded ARXML coverage and robustness.
- More helpers for signal decoding and physical value calculation.
- Additional trace formats.

Contributions are welcome—see below!

---

## Contributing

Issues and PRs are appreciated. Please run tests and `cargo fmt`/`cargo clippy` before submitting.

```bash
cargo test
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

---

## License

Licensed under the [MIT License](./LICENSE).

---

## Acknowledgements

Inspired by everyday workflows around DBC/ARXML/CAN trace handling in automotive engineering.
