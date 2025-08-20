//! DBC data model.
//!
//! This module defines the “DB-side” types used to represent a CAN database
//! (`.dbc` or `.arxml` file) once parsed. The types here are designed to:
//! - Navigate messages, signals, and nodes (ECUs);
//! - Perform fast lookups via normalized keys;
//! - Provide utilities to extract/decode a signal’s raw value
//!   starting from a byte payload.

use std::collections::HashMap;

use crate::types::canlog::SignalLog;

// --- Typed indices (simple wrappers; can be evolved into robust newtypes later) ---

/// Indexed identifier of a node (ECU) within `Database.nodes`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct NodeId(pub usize);

/// Indexed identifier of a message within `Database.messages`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct MessageId(pub usize);

/// Indexed identifier of a signal within `Database.signals`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Default)]
pub struct SignalId(pub usize);

/// In-memory representation of a CAN database (DBC/ARXML).
///
/// Holds metadata (name, bus type, baud rates, version), the lists of nodes/messages/signals,
/// and several normalized lookup maps for efficient queries.
///
/// ### Internal lookups
/// - `msg_by_id`: lookup by numeric CAN ID (`u64`);
/// - `msg_by_hex`: lookup by normalized hexadecimal CAN ID (`"0x..."`, uppercase);
/// - `msg_by_name`: lookup by message name, **case-insensitive** (lowercase key);
/// - `node_by_name`: lookup by node name, **case-insensitive** (lowercase key).
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Database {
    // --- General information ---
    /// Logical name of the database (if available).
    pub name: String,
    /// Bus type (e.g., `"CAN"`).
    pub bustype: String,
    /// Classic baud rate (bit/s). `0` if unspecified.
    pub baudrate: u32,
    /// CAN FD baud rate (bit/s). `0` if unspecified.
    pub baudrate_canfd: u32,
    /// Database version string.
    pub version: String,
    /// Database comment.
    pub comment: String,

    // --- Main storage (indexed lists) ---
    /// List of nodes/ECUs present in the database.
    pub nodes: Vec<NodeDB>,
    /// List of defined messages.
    pub messages: Vec<MessageDB>,
    /// List of defined signals.
    pub signals: Vec<SignalDB>,

    // --- Internal lookups (normalized keys) ---
    msg_by_id: HashMap<u64, MessageId>,
    msg_by_hex: HashMap<String, MessageId>,  // normalized hexadecimal “0x...”, uppercase
    msg_by_name: HashMap<String, MessageId>, // message name in lowercase
    node_by_name: HashMap<String, NodeId>,   // node name in lowercase
}

impl Database {
    // ---- Adders: keep relationships and indices consistent ----

    /// Adds a node to the database and returns the corresponding `NodeId`.
    ///
    /// Automatically updates the `node_by_name` (case-insensitive) lookup.
    pub fn add_node(&mut self, node: NodeDB) -> NodeId {
        let id: NodeId = NodeId(self.nodes.len());
        let key: String = node.name.to_lowercase();
        self.nodes.push(node);
        self.node_by_name.insert(key, id);
        id
    }

    /// Adds a message and indexes its id/name.
    ///
    /// Updates:
    /// - `msg_by_id` with the numeric ID;
    /// - `msg_by_hex` with the **normalized** hexadecimal ID;
    /// - `msg_by_name` with the lowercase name.
    ///
    /// Additionally, registers the message within `messages_sent` of each sender node.
    pub fn add_message(&mut self, mut msg: MessageDB) -> MessageId {
        let id: MessageId = MessageId(self.messages.len());

        // normalize and index id/name
        let hex: String = normalize_id_hex(&msg.id_hex);
        msg.id_hex = hex.clone();
        self.msg_by_id.insert(msg.id, id);
        self.msg_by_hex.insert(hex, id);
        self.msg_by_name.insert(msg.name.to_lowercase(), id);

        // back-reference: from sender nodes to the message
        for &nid in &msg.sender_nodes {
            if let Some(node) = self.nodes.get_mut(nid.0) {
                node.messages_sent.push(id);
            }
        }

        self.messages.push(msg);
        id
    }

    /// Adds a signal and links it to its parent message (`MessageDB.signals`).
    pub fn add_signal(&mut self, sig: SignalDB) -> SignalId {
        let id: SignalId = SignalId(self.signals.len());

        // attach the signal to its message
        let midx: MessageId = sig.message;
        if let Some(msg) = self.messages.get_mut(midx.0) {
            msg.signals.push(id);
        }

        self.signals.push(sig);
        id
    }

    /// Completely clears the database (metadata, lists, and lookups).
    pub fn clear(&mut self) {
        self.name.clear();
        self.bustype.clear();
        self.baudrate = 0;
        self.baudrate_canfd = 0;
        self.version.clear();

        self.nodes.clear();
        self.messages.clear();
        self.signals.clear();
        self.msg_by_id.clear();
        self.msg_by_hex.clear();
        self.msg_by_name.clear();
        self.node_by_name.clear();
    }

    // ---- Public accessors ----

    /// Returns a `&MessageDB` given the numeric CAN ID.
    pub fn get_message_by_id(&self, id: u64) -> Option<&MessageDB> {
        self.msg_by_id.get(&id).map(|&mid| &self.messages[mid.0])
    }

    /// Returns a `&mut MessageDB` given the numeric CAN ID.
    pub fn get_message_by_id_mut(&mut self, id: u64) -> Option<&mut MessageDB> {
        if let Some(&mid) = self.msg_by_id.get(&id) {
            self.messages.get_mut(mid.0)
        } else {
            None
        }
    }

    /// Returns a `&MessageDB` given a hexadecimal ID (case-insensitive).
    ///
    /// The argument may come in various forms, e.g., `"12dd54e3"`, `"0x12dd54e3"`, `"12DD54E3x"`;
    /// it will be normalized internally to `"0x12DD54E3"`.
    pub fn get_message_by_id_hex(&self, id_hex: &str) -> Option<&MessageDB> {
        let key: String = normalize_id_hex(id_hex);
        self.msg_by_hex.get(&key).map(|&mid| &self.messages[mid.0])
    }

    /// Returns a `&mut MessageDB` given a hexadecimal ID (case-insensitive).
    pub fn get_message_by_id_hex_mut(&mut self, id_hex: &str) -> Option<&mut MessageDB> {
        let key: String = normalize_id_hex(id_hex);
        if let Some(&mid) = self.msg_by_hex.get(&key) {
            self.messages.get_mut(mid.0)
        } else {
            None
        }
    }

    /// Returns a `&MessageDB` given the name (case-insensitive).
    pub fn get_message_by_name(&self, name: &str) -> Option<&MessageDB> {
        self.msg_by_name
            .get(&name.to_lowercase())
            .map(|&mid| &self.messages[mid.0])
    }

    /// Returns a `&mut MessageDB` given the name (case-insensitive).
    pub fn get_message_by_name_mut(&mut self, name: &str) -> Option<&mut MessageDB> {
        if let Some(&mid) = self.msg_by_name.get(&name.to_lowercase()) {
            self.messages.get_mut(mid.0)
        } else {
            None
        }
    }

    /// Returns a `&NodeDB` given the name (case-insensitive).
    ///
    /// _Note_: the method name is plural for backward compatibility,
    /// but it returns a single node if present.
    pub fn get_nodes_by_name(&self, name: &str) -> Option<&NodeDB> {
        self.node_by_name
            .get(&name.to_lowercase())
            .map(|&nid| &self.nodes[nid.0])
    }

    /// Returns a `&mut NodeDB` given the name (case-insensitive).
    ///
    /// _Note_: the method name is plural for backward compatibility,
    /// but it returns a single node if present.
    pub fn get_nodes_by_name_mut(&mut self, name: &str) -> Option<&mut NodeDB> {
        if let Some(&nid) = self.node_by_name.get(&name.to_lowercase()) {
            self.nodes.get_mut(nid.0)
        } else {
            None
        }
    }

    /// Returns the `NodeId` of a node by name (case-insensitive).
    pub fn get_node_id_by_name(&self, name: &str) -> Option<NodeId> {
        self.node_by_name.get(&name.to_lowercase()).copied()
    }
}

/// CAN message defined in the database (DBC/ARXML).
///
/// Maintains the numeric ID (`id`), the normalized hexadecimal ID (`id_hex`),
/// the `name`, payload length (`byte_length`), and metadata such as `msgtype`, `cycle_time`,
/// the transmitting nodes (`sender_nodes`), and the list of composing signals (`signals`).
#[derive(Default, Clone, PartialEq, Debug)]
pub struct MessageDB {
    /// Numeric CAN ID (base 10).
    pub id: u64,
    /// **Normalized** hexadecimal CAN ID (`"0x..."`, uppercase).
    pub id_hex: String,
    /// Message name.
    pub name: String,
    /// Payload length in bytes.
    pub byte_length: u16,
    /// Message type (free-form; if present in the DBC).
    pub msgtype: String,
    /// Cycle time in milliseconds (if defined; 0 if unknown).
    pub cycle_time: u16,
    /// Transmitting nodes (ECUs) for this message.
    pub sender_nodes: Vec<NodeId>,
    /// Signals that belong to this message.
    pub signals: Vec<SignalId>,
    /// Associated comment (DBC `CM_ BO_` section).
    pub comment: String,
}

impl MessageDB {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        self.id = 0;
        self.id_hex.clear();
        self.name.clear();
        self.byte_length = 0;
        self.msgtype.clear();
        self.cycle_time = 0;
        self.sender_nodes.clear();
        self.signals.clear();
        self.comment.clear();
    }

    /// Convenience iterator over the `SignalDB`s belonging to this message.
    ///
    /// Example:
    /// ```
    /// # use can_tools::types::database::{Database, MessageDB, SignalDB, MessageId, NodeDB};
    /// # let db = Database::default();
    /// # let msg = MessageDB::default();
    /// # let _ = msg.signals(&db).count();
    /// ```
    pub fn signals<'a>(&'a self, db: &'a Database) -> impl Iterator<Item = &'a SignalDB> + 'a {
        self.signals
            .iter()
            .filter_map(move |&sid| db.signals.get(sid.0))
    }
}

/// Elementary step for extracting a bit field from a payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Step {
    /// Source byte index.
    pub(crate) byte_index: u8,
    /// LSB within the source byte (0..7).
    pub(crate) src_lsb: u8,
    /// Number of bits to take (1..8).
    pub(crate) width: u8,
    /// Destination LSB in the final value (LSB-first).
    pub(crate) dst_lsb: u16,
}

/// Definition of a signal within a CAN message (DBC).
///
/// Describes position/bit-length, endianness, sign, scaling (factor/offset),
/// valid range, unit of measure, value tables, and receiver nodes.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct SignalDB {
    /// Parent message (index in `Database.messages`).
    pub message: MessageId,
    /// Signal name.
    pub name: String,
    /// Bit start in the payload (bit 0 = LSB of the first byte).
    pub bit_start: u16,
    /// Bit length.
    pub bit_length: u16,
    /// Endianness: `1` = little-endian (Intel), `0` = big-endian (Motorola).
    pub endian: u8,
    /// Sign: `1` = signed, `0` = unsigned.
    pub sign: u8,
    /// Scaling factor.
    pub factor: f64,
    /// Scaling offset.
    pub offset: f64,
    /// Minimum physical value.
    pub min: f64,
    /// Maximum physical value.
    pub max: f64,
    /// Unit of measure (normalized elsewhere by removing the optional `"Unit_"` prefix).
    pub unit_of_measurement: String,
    /// Receiver nodes.
    pub receiver_nodes: Vec<NodeId>,
    /// Associated comment (DBC `CM_ SG_` section).
    pub comment: String,
    /// Value-to-text mapping (value table).
    pub value_table: HashMap<i32, String>,
    /// Precomputed extraction steps for fast decoding.
    pub(crate) steps: Vec<Step>,
}

impl SignalDB {
    /// Returns an immutable reference to a receiver node by name (case-insensitive).
    pub fn get_receiver_nodes_by_name<'a>(
        &self,
        db: &'a Database,
        name: &str,
    ) -> Option<&'a NodeDB> {
        let key: String = name.to_lowercase();
        self.receiver_nodes
            .iter()
            .filter_map(|&nid| db.nodes.get(nid.0))
            .find(|node| node.name.to_lowercase() == key)
    }

    /// Returns a mutable reference to a receiver node by name (case-insensitive).
    pub fn get_receiver_nodes_by_name_mut<'a>(
        &self,
        db: &'a mut Database,
        name: &str,
    ) -> Option<&'a mut NodeDB> {
        let key: String = name.to_lowercase();
        let nid = self.receiver_nodes.iter().copied().find(|&nid| {
            db.nodes
                .get(nid.0)
                .map(|n| n.name.to_lowercase() == key)
                .unwrap_or(false)
        })?;
        db.nodes.get_mut(nid.0)
    }

    /// Precomputes bit → value extraction steps to speed up decoding.
    pub fn compile_inline(&mut self) {
        if !self.steps.is_empty() {
            return;
        }
        // ceil((bit_len + (bit_start % 8)) / 8)
        let n_steps: usize = (self.bit_length as usize + (self.bit_start as usize & 7))
            .div_ceil(8)
            .max(1);
        self.steps.reserve_exact(n_steps);

        if self.endian == 1 {
            self.compile_intel();
        } else {
            self.compile_motorola();
        }
    }

    #[inline]
    fn push_step(&mut self, st: Step) {
        self.steps.push(st);
    }

    /// Step compilation for little-endian (Intel) signals.
    fn compile_intel(&mut self) {
        let mut remaining: u16 = self.bit_length;
        let mut bit: u16 = self.bit_start;
        let mut dst: u16 = 0u16;

        while remaining > 0 {
            let byte_idx: u8 = (bit / 8) as u8;
            let bit_off: u8 = (bit % 8) as u8;
            let avail: u8 = 8 - bit_off;
            let take: u8 = remaining.min(avail as u16) as u8;

            self.push_step(Step {
                byte_index: byte_idx,
                src_lsb: bit_off,
                width: take,
                dst_lsb: dst,
            });

            bit += take as u16;
            dst += take as u16;
            remaining -= take as u16;
        }
    }

    /// Step compilation for big-endian (Motorola) signals.
    fn compile_motorola(&mut self) {
        // In DBC, @0: the start bit is the MSB of the signal; we advance MSB-first.
        let mut remaining: u16 = self.bit_length;
        let mut byte: usize = (self.bit_start / 8) as usize;
        let mut bit_msb: u8 = 7 - (self.bit_start % 8) as u8;

        while remaining > 0 {
            let can_take: u16 = (bit_msb as u16 + 1).min(remaining);
            let src_lsb: u8 = bit_msb + 1 - can_take as u8;
            let dst_lsb: u16 = remaining - can_take;

            self.push_step(Step {
                byte_index: byte as u8,
                src_lsb,
                width: can_take as u8,
                dst_lsb,
            });

            remaining -= can_take;
            if src_lsb == 0 {
                byte += 1;
                bit_msb = 7;
            } else {
                bit_msb = src_lsb - 1;
            }
        }
    }

    /// Extracts the **unsigned** raw value (LSB-first accumulation) from the payload.
    #[inline]
    pub fn extract_raw_u64(&self, bytes: &[u8]) -> u64 {
        let mut out: u64 = 0;
        for st in &self.steps {
            if let Some(&b) = bytes.get(st.byte_index as usize) {
                let mask: u8 = if st.width == 8 {
                    0xFF
                } else {
                    ((1u16 << st.width) - 1) as u8
                };
                let chunk = ((b >> st.src_lsb) & mask) as u64;
                out |= chunk << st.dst_lsb;
            }
        }
        out
    }

    /// Extracts the **signed** raw value from the payload, performing sign extension if needed.
    #[inline]
    pub fn extract_raw_i64(&self, bytes: &[u8]) -> i64 {
        let raw_u: u64 = self.extract_raw_u64(bytes);
        let n: u16 = self.bit_length.min(64);
        if self.sign == 1 && n > 0 {
            let sign_bit = 1u64 << (n - 1);
            if (raw_u & sign_bit) != 0 {
                let mask = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
                (raw_u | !mask) as i64
            } else {
                raw_u as i64
            }
        } else {
            raw_u as i64
        }
    }

    /// Converts a raw value into an “instantaneous” `SignalLog` with physical value, text, and metadata.
    ///
    /// *Note*: the unit is normalized by removing an optional `"Unit_"` prefix.
    #[inline]
    pub fn to_sigframe(&self, raw_i: i64) -> SignalLog {
        let value: f64 = (raw_i as f64) * self.factor + self.offset;
        let text: String = self
            .value_table
            .get(&(raw_i as i32))
            .cloned()
            .unwrap_or_default();
        SignalLog {
            message: 0,
            name: self.name.clone(),
            factor: self.factor,
            offset: self.offset,
            channel: 0,
            raw: raw_i,
            value,
            unit: self
                .unit_of_measurement
                .strip_prefix("Unit_")
                .unwrap_or(&self.unit_of_measurement)
                .to_string(),
            text,
            comment: self.comment.clone(),
            value_table: self.value_table.clone(),
            values: Vec::new(),
        }
    }
}

/// Node/ECU defined in the database.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct NodeDB {
    /// Node/ECU name.
    pub name: String,
    /// Associated comment (if present).
    pub comment: String,
    /// Messages transmitted by this node.
    pub messages_sent: Vec<MessageId>,
}

impl NodeDB {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        self.name.clear();
        self.comment.clear();
        self.messages_sent.clear();
    }
}

// --- helpers ---

/// Normalizes a hexadecimal ID string.
///
/// Converts variants such as `"12DD54E3x"`, `"0x12dd54e3"`, `"12dd54e3"`
/// into the canonical form `"0x12DD54E3"`.
fn normalize_id_hex(s: &str) -> String {
    let t: &str = s.trim();
    let t: &str = t
        .strip_suffix('x')
        .or_else(|| t.strip_suffix('X'))
        .unwrap_or(t);
    let t: &str = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    format!("0x{}", t.to_uppercase())
}
