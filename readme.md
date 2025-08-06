# can_tools

`can_tools` is a Rust library for parsing [DBC files](https://en.wikipedia.org/wiki/DBC_(file_format))
used in automotive applications to describe CAN network messages and signals.

It provides a clean and easy-to-use API to read `.dbc` files, inspect messages, signals, and nodes,
and access comments and value tables.

---

## ✨ Features

- Parses complete `.dbc` files into structured Rust types
- Reads:
  - **Version** information
  - **Bit timing**
  - **Nodes**
  - **Messages**
  - **Signals**
  - **Comments**
  - **Value tables**
- Case-insensitive search utilities for messages, signals, and nodes
- Simple API: one call to [`parse`](src/file/dbc.rs) produces a ready-to-use `Database`

---

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
can_tools = "0.1.7"
```

---

## 🚀 Example

```rust
use can_tools::file::parse;

fn main() -> Result<(), String> {
    // Parse the DBC file
    let db = parse("path/to/file.dbc")?;

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

---

## 📊 Data Model

```text
┌───────────────────────────────────────┐
│               Database                │
│───────────────────────────────────────│
│ version: String                       │
│ bit_timing: String                    │
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
└───────────────────────────────────────┘

Node: represents a CAN ECU (sender or receiver)
```

---

## 📚 Main Types

- **[`Database`](src/models/database.rs)**  
  Holds the parsed DBC file structure, including version, bit timing, all `Node`s, and `Message`s.

- **[`Message`](src/models/message.rs)**  
  Represents a CAN message. Contains message ID, name, sender nodes, and its list of `Signal`s.

- **[`Signal`](src/models/signal.rs)**  
  Represents a data field within a CAN message, including bit position, length, scaling factor, unit,
  receiver nodes, and optional value descriptions.

- **[`Node`](src/models/node.rs)**  
  Represents a CAN network node (ECU) that can send or receive messages.

---

## 📜 License

Licensed under the [MIT License](LICENSE).

---

## 🔗 Related Links

- [DBC File Format (Wikipedia)](https://en.wikipedia.org/wiki/DBC_(file_format))
- [Vector CANdb++](https://vector.com)
- [docs.rs Documentation](https://docs.rs/can_tools)
