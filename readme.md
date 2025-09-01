
# can_tools

Rust utilities for parsing and modeling automotive CAN databases and logs.

Updated 2025-08-22: uses SlotMap-backed arenas (stable keys), order-aware iteration, sorting helpers, and normalized lookups.

---

## Features

- DBC parsing → build an in-memory `DatabaseDBC` (nodes, messages, signals).
- ASC parsing → parse Vector ASCII traces into a `CanLog` model.
- ARXML parsing → extract CAN clusters into `DatabaseARXML` entries.
- Stable keys via SlotMap → reorder presentation without invalidating references.
- Order-aware iteration → `iter_nodes/messages/signals()` respect order vectors.
- Sorting helpers → `sort_nodes_by_name()`, `sort_messages_by_name()`, `sort_signals_by_name()`.
- Fast lookups → `get_message_by_id/_hex/_name`, `get_node_by_name`, `get_signal_by_name`.
- Signal decoding → compiled bit extraction with factor/offset and value tables.

This README documents the library API (no application/UI specifics).

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
can_tools = "1.2.7"
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
db.sort_nodes_by_name();
db.sort_messages_by_name();
db.sort_signals_by_name();
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

// Walk latest frame per (id,channel)
for idx in &log.last_id_chn_frame {
    if let Some(frame) = log.can_frames.get(*idx) {
        println!("ch{} @{} -> msg #{}", frame.channel, frame.timestamp, frame.message);
    }
}

// Resolve signals for a message index
for sig in resolve_message_signals(&log, 0) {
    println!("{} => {} {}", sig.name, sig.value, sig.unit);
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
