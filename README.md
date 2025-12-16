# can_tools

Rust utilities for reading, creating, editing and writing CAN DBC files, with optional AUTOSAR `.arxml` import support.

## Features
- Parse DBC files into an in-memory `CanDatabase` (tolerant to comments, extra spaces, and Windows-1252 encoded files).
- Convert AUTOSAR `.arxml` clusters into `CanDatabase` instances.
- Serialize databases back to DBC text with attributes, comments, value tables, multiplexing, and sender/receiver relations.
- Build a fresh database with sensible defaults for attributes such as bus type, baud rate, and version metadata.
- SlotMap-backed storage with stable keys for nodes, messages, and signals, plus helper lookups and sort utilities.

## Quick start
Add to your `Cargo.toml`:
```toml
can_tools = "2.1.2"
```

Parse a DBC, tweak it, and write it back:
```rust
use can_tools::{parse, save, DatabaseError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = parse::from_dbc_file("input.dbc")?;

    // Access a message by name
    if let Some(msg) = db.get_message_by_name_mut("EngineData") {
        msg.comment = "Edited by can_tools".into();
    }

    // Persist the changes
    save::save_to_file("output.dbc", &db)?;
    Ok(())
}
```

Create a brand new database:
```rust
use can_tools::{create, save, types::database::BusType};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = create::new_database("MyDbc", BusType::CanFd, "1.0.0")?;
    save::save_to_file("my.dbc", &db)?;
    Ok(())
}
```

Import AUTOSAR `.arxml`:
```rust
use can_tools::parse;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let databases = parse::from_arxml_file("network.arxml")?;
    println!("Extracted {} CAN clusters", databases.len());
    Ok(())
}
```

Iterate messages, nodes, and signals:
```rust
use can_tools::{parse, types::database::CanDatabase};

fn dump(db: &CanDatabase) {
    println!("Database: {} ({} nodes, {} messages)", db.name, db.nodes.len(), db.messages.len());

    for node in db.iter_nodes() {
        println!("- Node: {}", node.name);
    }

    for msg in db.iter_messages() {
        println!("BO_ {} {} ({} bytes)", msg.id, msg.name, msg.byte_length);
        for sig in msg.signals(db) {
            println!(
                "    SG_ {} [{}|{}] {} {:?}",
                sig.name, sig.bit_start, sig.bit_length, sig.endian, sig.sign
            );
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = parse::from_dbc_file("input.dbc")?;
    dump(&db);
    Ok(())
}
```

## Modules at a glance
- `parse`: `from_dbc_file` and `from_arxml_file` entry points for ingestion.
- `save`: `save_to_file` and helpers to serialize a `CanDatabase`.
- `create`: builds a `CanDatabase` pre-populated with canonical attributes.
- `types`: core data structures (`CanDatabase`, `CanMessage`, `CanSignal`, `CanNode`, attributes, errors).
- `core`: internal decoders/encoders for DBC sections (attributes, comments, signals, value tables, etc.).

## Error handling
All public operations return strongly-typed errors (e.g. `DbcParseError`, `DbcSaveError`, `DatabaseError`). Many parsing helpers are resilient: malformed lines are skipped where safe, while structural issues (wrong extensions, I/O errors) bubble up as errors.

## Notes
- DBC files are decoded as Windows-1252 with common German characters transliterated to ASCII.
- Names and lookups are case-insensitive; message IDs are tracked in both decimal and hex.
- Multiplexing is supported (both multiplexer and multiplexed signals), as are value tables and relational attributes (`BU_SG_REL_`, `BU_BO_REL_`).
