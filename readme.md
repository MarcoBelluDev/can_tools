
# can_tools

Rust utilities for parsing and modeling automotive CAN databases and logs.

Updated 2025-09-03: streaming DBC reader (Windows-1252, single-pass transliteration),
ASC absolute-time formatting optimized, latest-frame index maintenance optimized,
SlotMap-backed arenas (stable keys), order-aware iteration and caching in sorts.

---

## Features

- DBC parsing → build an in-memory `DatabaseDBC` (nodes, messages, signals).
- ASC parsing → parse Vector ASCII traces into a `CanLog` model.

This README documents the library API (no application/UI specifics).

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
can_tools = "1.2.15"
```

Use only the DBC parser (disable default features):

```toml
[dependencies]
can_tools = { version = "1.2.15", default-features = false, features = ["dbc"] }
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

### Parse ARXML (CAN clusters)

```rust
use can_tools::arxml;
use can_tools::arxml::types::database::DatabaseARXML;

let clusters: Vec<DatabaseARXML> = arxml::parse_from_file("network.arxml")?;
for c in &clusters {
    println!("{} [{}] v{}", c.name, c.bustype.to_str(), c.version);
}
```

---

## License

MIT
