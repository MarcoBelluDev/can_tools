
# can_tools

Rust utilities for parsing and modeling automotive CAN databases and logs.

---

## Highlights

- DBC parsing → streaming reader that decodes Windows-1252, transliterates a few common characters,
  and materialises a SlotMap-backed `DatabaseDBC` (nodes, messages, signals, attributes, relations).
- ASC parsing → single-pass Vector ASCII reader that discovers optional `date` headers, formats
  absolute timestamps, keeps every frame in `can_frames`, and tracks one latest frame per `(id, channel)`
  in `last_id_chn_frame`.
- Programmatic authoring → build an empty `DatabaseDBC` with `dbc::create::new_database` and export it
  back to disk via `dbc::save::save_to_file`.

## Feature Flags

The crate enables both parsers by default. Use Cargo features to opt out:

| Feature | Enabled by default? | Description |
|---------|---------------------|-------------|
| `dbc`   | ✅                  | DBC database parser and models (brings `encoding_rs`, `slotmap`). |
| `asc`   | ✅                  | Vector ASCII trace parser and models (depends on `dbc`, adds `chrono`). |

This README documents the library API (no application/UI specifics).

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
can_tools = "1.5.1"
```

Use only the DBC parser (disable default features):

```toml
[dependencies]
can_tools = { version = "1.5.1", default-features = false, features = ["dbc"] }
```

Minimal usage with DBC only:

```rust
use can_tools::dbc;
use can_tools::dbc::types::database::DatabaseDBC;

fn main() -> Result<(), String> {
    // Parse a .dbc file into an in-memory database
    let mut db: DatabaseDBC = dbc::parse::from_file("network.dbc")?;

    // Optional: sort presentation (ASCII case-insensitive)
    db.sort_db_nodes_by_name();
    db.sort_db_messages_by_name();
    db.sort_db_signals_by_name();

    // Iterate and use
    for m in db.iter_messages() {
        println!("{} ({})", m.name, m.msgtype);
    }

    Ok(())
}
```

---

## Usage Overview

### Parse a DBC

```rust
use can_tools::dbc;
use can_tools::dbc::types::database::DatabaseDBC;

let db: DatabaseDBC = dbc::parse::from_file("network.dbc")?;
```

Iterate in presentation order:

```rust
for n in db.iter_nodes() { println!("{}", n.name); }
for m in db.iter_messages() { println!("{} ({} bytes)", m.name, m.byte_length); }
for s in db.iter_signals() { println!("{}", s.name); }
```

Sort by name (ASCII case-insensitive):

```rust
let mut db = db;
db.sort_db_nodes_by_name();
db.sort_db_messages_by_name();
db.sort_db_signals_by_name();
```

Lookups:

```rust
let by_id = db.get_message_by_id(0x123);
let by_hex = db.get_message_by_id_hex("0x1A2B");
let by_name = db.get_message_by_name("EngineData");
let node = db.get_node_by_name("Gateway");
let sig = db.get_signal_by_name("VehicleSpeed");
```

Reset:

```rust
let mut db = db;
db.clear();
```

Notes on attributes:
- Attribute definitions come from `BA_DEF_*` and defaults from `BA_DEF_DEF_`.
- Assignments `BA_` set values on DB/Node/Message/Signal.
- ENUM assignments in `BA_` use numeric indices into the declared enum list.

### Create and Save a DBC

```rust
use can_tools::dbc::{
    create::new_database,
    save::save_to_file,
    types::database::BusType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Scaffold a fresh database with canonical metadata defaults.
    let mut db = new_database("ExampleNetwork", BusType::CanFd, "1.0.0")?;

    // TODO: add nodes/messages/signals here...

    // Persist the database to disk. The path must end with ".dbc".
    save_to_file("artifacts/example_network.dbc", db)?;
    Ok(())
}
```

`new_database` validates that both `name` and `version` are non-empty and
pre-populates common attribute definitions such as baud rates, date stamp,
and manufacturer placeholders. `save_to_file` creates parent directories on
the fly, serialises every section (NS_, BA_DEF_*, BA_, CM_, VAL_, etc.), and
returns a `DbcSaveError` variant when the path or write fails.

### Parse an ASC trace

```rust
use std::collections::HashMap;
use can_tools::asc;
use can_tools::asc::types::canlog::{CanLog, resolve_message_signals};
use can_tools::dbc::types::database::DatabaseDBC;

// Optional: provide per-channel databases for enrichment
let mut dbs: HashMap<u8, DatabaseDBC> = HashMap::new();
dbs.insert(1, dbc::parse::from_file("network.dbc")?);

let log: CanLog = asc::parse::from_file("trace.asc", &dbs)?;

// `CanLog` contains:
// - `can_frames`: every frame in file order;
// - `messages`: one entry per frame with enriched metadata;
// - `signals`: aggregated decoded signals updated as frames arrive;
// - `last_id_chn_frame`: one index per `(id, channel)` pointing to the freshest frame;
// - `absolute_time`: optional trace start timestamp derived from the `date` header.

// Iterate all frames in file order and access their messages
for frame in &log.can_frames {
    let msg = &log.messages[frame.message];
    println!(
        "{} ch{} {} {} [{}] {}",
        frame.absolute_time, // absolute time or synthetic fallback
        frame.channel,
        msg.id,              // raw id token as in the log
        msg.name,            // may be empty if no DB was provided
        msg.protocol,        // "CAN" or "CAN FD"
        msg.data,            // hex payload as space-separated bytes
    );

    // Resolve decoded signals for each frame's message
    for sig in resolve_message_signals(&log, frame.message) {
        println!(
            "ch{} {}: {} => {} {}",
            frame.channel,
            frame.absolute_time,
            sig.name,
            sig.value,
            sig.unit,
        );
    }
}
```

---

## License

MIT
