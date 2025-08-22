
# can_tools

Rust utilities for parsing and modeling **automotive CAN** databases and logs.

> **Updated:** 2025-08-22. The library now uses **SlotMap** for Nodes/Messages/Signals and exposes
> stable iteration, sorting helpers, and normalized lookups on the `Database`.

---

## Features

- **DBC parsing** → build an in-memory `Database` (nodes, messages, signals).
- **ASC parsing** → parse Vector ASCII traces into a log model.
- **Stable keys via SlotMap** → reorder presentation without invalidating references.
- **Order-aware iteration** → `iter_nodes/messages/signals()` respect order vectors.
- **Sorting helpers** → `sort_nodes_by_name()`, `sort_messages_by_name()`, `sort_signals_by_name()`.
- **Fast lookups** → `get_message_by_id/_hex/_name`, `get_node_by_name`, `get_signal_by_name`.
- **Signal decoding** → compiled bit-extraction on `SignalDB` with factor/offset and optional value tables.

This README documents the **library only** (no application/UI specifics). See the `docs/` folder for reference guides.

---

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
can_tools = "*"
```

---

## Usage overview

### Parse a DBC

```rust
use can_tools::{{dbc, Database}};

let db: Database = dbc::parse::from_file("network.dbc")?;
```

### Iterate respecting order vectors

```rust
for n in db.iter_nodes() {{ println!("{}", n.name); }}
for m in db.iter_messages() {{ println!("{} (0x{:X})", m.name, m.id); }}
for s in db.iter_signals() {{ println!("{}", s.name); }}
```

### Sort by name

```rust
let mut db = db;
db.sort_nodes_by_name();
db.sort_messages_by_name();
db.sort_signals_by_name();
```

### Lookups

```rust
let msg = db.get_message_by_id(0x123);
let msg_hex = db.get_message_by_id_hex("0x123");
let msg_by_name = db.get_message_by_name("EngineData");
let node = db.get_node_by_name("Gateway");
let sig = db.get_signal_by_name("VehicleSpeed");
```

### Clear

```rust
let mut db = db;
db.clear(); // resets arenas, order vectors, and lookups
```

For details on structures and APIs, see `docs/database.md` and `docs/canlog.md`.
