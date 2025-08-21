//! DBC data model.
//!
//! This module defines the "DB-side" types used to represent a CAN database
//! (`.dbc` or `.arxml` file) once parsed. The types here are designed to:
//! - Navigate messages, signals, and nodes (ECUs);
//! - Perform fast lookups via normalized keys;
//! - Provide utilities to extract/decode a signal's raw value
//!   starting from a byte payload.

use std::collections::HashMap;
use slotmap::{SlotMap, new_key_type};

use crate::types::canlog::SignalLog;

// --- Stable keys (SlotMap) ---
new_key_type! { pub struct NodeKey; }
new_key_type! { pub struct MessageKey; }
new_key_type! { pub struct SignalKey; }

/// In-memory representation of a CAN database (DBC/ARXML).
///
/// Holds metadata (name, bus type, baud rates, version), the arenas of nodes/messages/signals
/// (SlotMaps with stable keys), optional order vectors to control iteration order, and
/// several normalized lookup maps for efficient queries.
#[derive(Default, Clone, Debug)]
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

    // --- Main storage (stable-key arenas) ---
    pub nodes: SlotMap<NodeKey, NodeDB>,
    pub messages: SlotMap<MessageKey, MessageDB>,
    pub signals: SlotMap<SignalKey, SignalDB>,

    // --- Order "views" (you can reorder these without touching the arenas) ---
    pub nodes_order: Vec<NodeKey>,
    pub messages_order: Vec<MessageKey>,
    pub signals_order: Vec<SignalKey>,

    // --- Misc info (left as-is from your model) ---
    pub nm_type: String,
    pub manufacturer: String,
    pub nmh_message_count: u8,
    pub nmh_base_address: u32,
    pub nmh_n_start: u16,
    pub nmh_long_timer: u16,
    pub nmh_prepare_bus_sleep_timer: u16,
    pub nmh_wait_bus_sleep_timer: u16,
    pub nmh_timeout_timer: u16,
    pub nmh_nbt_max: u8,
    pub nmh_nbt_min: u8,
    pub sync_jump_width_max: u8,
    pub sync_jump_width_min: u8,
    pub sample_point_max: u8,
    pub sample_point_min: u8,
    pub version_number: u8,
    pub version_year: u8,
    pub version_week: u8,
    pub version_month: u8,
    pub version_day: u8,
    pub vagtp20_setup_start_address: u8,
    pub vagtp20_setup_message_count: u8,
    pub gen_nwm_talk_nm: String,
    pub gen_nwm_sleep_time: u16,
    pub gen_nwm_goto_mode_bus_sleep: String,
    pub gen_nwm_goto_mode_awake: String,
    pub gen_nwm_ap_can_wake_up: String,
    pub gen_nwm_ap_can_sleep: String,
    pub gen_nwm_ap_can_on: String,
    pub gen_nwm_ap_can_off: String,
    pub gen_nwm_ap_can_normal: String,
    pub gen_nwm_ap_bus_sleep: String,

    // --- Lookups (case-normalized) ---
    pub(crate) node_key_by_name: HashMap<String, NodeKey>,   // lower(name) → NodeKey
    pub(crate) msg_key_by_id:   HashMap<u64, MessageKey>,    // id10 → MessageKey
    pub(crate) msg_key_by_hex:  HashMap<String, MessageKey>, // "0x...." uppercase → MessageKey
    pub(crate) msg_key_by_name: HashMap<String, MessageKey>, // lower(name) → MessageKey

    // Global map for signals by (lower) name. Beware of collisions if two BO_ have same SG_ name.
    pub(crate) sig_key_by_name: HashMap<String, SignalKey>,

    // Parsing state: last message seen (used by SG_ decoder)
    pub(crate) current_msg: Option<MessageKey>,
}

impl Database {
    // ---- First time adders from DB, not for customer ----

    /// Adds a node to the database if not already present and returns the corresponding `NodeKey`.
    pub(crate) fn add_node_if_absent(&mut self, name: &str) -> NodeKey {
        if let Some(r) = self.get_node_key_by_name(name) {
            return r;
        }
        let key = self.nodes.insert(NodeDB {
            name: name.to_string(),
            comment: String::new(),
            messages_sent: Vec::new(),
        });
        self.nodes_order.push(key);
        self.node_key_by_name.insert(name.to_lowercase(), key);
        key
    }

    /// Adds a message and indexes its id/name. Also sets `current_msg` for subsequent SG_ lines.
    pub(crate) fn add_message_if_absent(
        &mut self,
        name: &str,
        id: u64,
        id_hex: &str,
        byte_length: u16,
        sender_name: &str,
    ) -> MessageKey {
        if let Some(r) = self.get_msg_key_by_name(name) {
            self.current_msg = Some(r);
            return r;
        }

        let sender_node_id = if !sender_name.is_empty() {
            Some(self.add_node_if_absent(sender_name))
        } else {
            None
        };

        let norm_id_hex: String = normalize_id_hex(id_hex);

        let msg_key = self.messages.insert(MessageDB {
            id,
            id_hex: norm_id_hex.clone(),
            name: name.to_string(),
            byte_length,
            msgtype: if byte_length <= 8 { "CAN".into() } else { "CAN FD".into() },
            cycle_time: 0,
            sender_nodes: sender_node_id.into_iter().collect(),
            signals: Vec::new(),
            comment: String::new(),
        });

        self.messages_order.push(msg_key);

        self.msg_key_by_id.insert(id, msg_key);
        self.msg_key_by_hex.insert(norm_id_hex, msg_key);
        self.msg_key_by_name.insert(name.to_lowercase(), msg_key);

        if let Some(nid) = sender_node_id {
            if let Some(n) = self.nodes.get_mut(nid) {
                n.messages_sent.push(msg_key);
            }
        }

        self.current_msg = Some(msg_key);
        msg_key
    }

    /// Adds a signal to the database if not already present and returns the corresponding `SignalKey`.
    pub(crate) fn add_signal_if_absent(
        &mut self,
        name: &str,
        bit_start: u16,
        bit_length: u16,
        endian: u8,
        sign: u8,
        factor: f64,
        offset: f64,
        min: f64,
        max: f64,
        unit: &str,
        receiver_nodes: Vec<NodeKey>,
    ) -> SignalKey {
        if let Some(r) = self.get_sig_key_by_name(name) {
            return r;
        }

        let msg_key = match self.current_msg {
            Some(k) => k,
            None => {
                // Create a fallback message if an SG_ appears before any BO_ (rare).
                self.add_message_if_absent("__UNBOUND__", 0, "0x0", 8, "")
            }
        };

        let mut sig = SignalDB {
            message: msg_key,
            name: name.to_string(),
            bit_start,
            bit_length,
            endian,
            sign,
            factor,
            offset,
            min,
            max,
            unit_of_measurement: unit.to_string(),
            receiver_nodes,
            comment: String::new(),
            value_table: HashMap::new(),
            steps: Vec::new(),
        };
        sig.compile_inline();

        let sig_key = self.signals.insert(sig);
        self.signals_order.push(sig_key);

        if let Some(m) = self.messages.get_mut(msg_key) {
            if !m.signals.contains(&sig_key) {
                m.signals.push(sig_key);
            }
        }

        self.sig_key_by_name.insert(name.to_lowercase(), sig_key);
        sig_key
    }

    // ---- Getter, not for customer, based on Keys ----

    // --------- Nodes --------
    pub(crate) fn get_node_key_by_name(&self, name: &str) -> Option<NodeKey> {
        self.node_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub(crate) fn get_node_by_key(&self, key: NodeKey) -> Option<&NodeDB> {
        self.nodes.get(key)
    }

    pub(crate) fn get_node_by_key_mut(&mut self, key: NodeKey) -> Option<&mut NodeDB> {
        self.nodes.get_mut(key)
    }

    // --------- Messages --------
    pub(crate) fn get_msg_key_by_name(&self, name: &str) -> Option<MessageKey> {
        self.msg_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub(crate) fn get_msg_key_by_id(&self, id: &u64) -> Option<MessageKey> {
        self.msg_key_by_id.get(id).copied()
    }

    pub(crate) fn get_msg_key_by_id_hex(&self, id_hex: &str) -> Option<MessageKey> {
        let key: String = normalize_id_hex(id_hex); // "0x...UPPERCASE"
        self.msg_key_by_hex.get(&key).copied()
    }

    pub(crate) fn get_message_by_key(&self, key: MessageKey) -> Option<&MessageDB> {
        self.messages.get(key)
    }

    pub(crate) fn get_message_by_key_mut(&mut self, key: MessageKey) -> Option<&mut MessageDB> {
        self.messages.get_mut(key)
    }

    // --------- Signals --------
    pub(crate) fn get_sig_key_by_name(&self, name: &str) -> Option<SignalKey> {
        self.sig_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub(crate) fn get_sig_by_key(&self, key: SignalKey) -> Option<&SignalDB> {
        self.signals.get(key)
    }

    pub(crate) fn get_sig_by_key_mut(&mut self, key: SignalKey) -> Option<&mut SignalDB> {
        self.signals.get_mut(key)
    }

    // ---- Public getters ----
    // --------- Nodes --------

    /// Returns a `&NodeDB` given the name (case-insensitive).
    pub fn get_node_by_name(&self, name: &str) -> Option<&NodeDB> {
        let key: NodeKey = *self.node_key_by_name.get(&name.to_lowercase())?;
        self.get_node_by_key(key)
    }

    /// Returns a `&mut NodeDB` given the name (case-insensitive).
    pub fn get_node_by_name_mut(&mut self, name: &str) -> Option<&mut NodeDB> {
        let key: NodeKey = *self.node_key_by_name.get(&name.to_lowercase())?;
        self.get_node_by_key_mut(key)
    }

    // --------- Messages --------

    /// Returns a `&MessageDB` given the numeric CAN ID.
    pub fn get_message_by_id(&self, id: u64) -> Option<&MessageDB> {
        let key: MessageKey = self.get_msg_key_by_id(&id)?;
        self.get_message_by_key(key)
    }

    /// Returns a `&mut MessageDB` given the numeric CAN ID.
    pub fn get_message_by_id_mut(&mut self, id: u64) -> Option<&mut MessageDB> {
        let key: MessageKey = self.get_msg_key_by_id(&id)?;
        self.get_message_by_key_mut(key)
    }

    /// Returns a `&MessageDB` given a hexadecimal ID (case-insensitive).
    ///
    /// The argument may come in various forms, e.g., `"12dd54e3"`, `"0x12dd54e3"`, `"12DD54E3x"`;
    /// it will be normalized internally to `"0x12DD54E3"`.
    pub fn get_message_by_id_hex(&self, id_hex: &str) -> Option<&MessageDB> {
        let key: MessageKey = self.get_msg_key_by_id_hex(id_hex)?;
        self.get_message_by_key(key)
    }

    /// Returns a `&mut MessageDB` given a hexadecimal ID (case-insensitive).
    pub fn get_message_by_id_hex_mut(&mut self, id_hex: &str) -> Option<&mut MessageDB> {
        let key: MessageKey = self.get_msg_key_by_id_hex(id_hex)?;
        self.get_message_by_key_mut(key)
    }

    /// Returns a `&MessageDB` given the name (case-insensitive).
    pub fn get_message_by_name(&self, name: &str) -> Option<&MessageDB> {
        let key: MessageKey = self.get_msg_key_by_name(name)?;
        self.get_message_by_key(key)
    }

    /// Returns a `&mut MessageDB` given the name (case-insensitive).
    pub fn get_message_by_name_mut(&mut self, name: &str) -> Option<&mut MessageDB> {
        let key: MessageKey = self.get_msg_key_by_name(name)?;
        self.get_message_by_key_mut(key)
    }

    // --------- Signals --------

    /// Returns a `&SignalDB` given the name (case-insensitive).
    pub fn get_signal_by_name(&self, name: &str) -> Option<&SignalDB> {
        let key: SignalKey = *self.sig_key_by_name.get(&name.to_lowercase())?;
        self.get_sig_by_key(key)
    }

    /// Returns a `&mut SignalDB` given the name (case-insensitive).
    pub fn get_signal_by_name_mut(&mut self, name: &str) -> Option<&mut SignalDB> {
        let key: SignalKey = *self.sig_key_by_name.get(&name.to_lowercase())?;
        self.get_sig_by_key_mut(key)
    }

    /// Iterators according to the orders (defualt order is name based)
    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeDB> + '_ {
        self.nodes_order.iter().filter_map(|&k| self.nodes.get(k))
    }
    pub fn iter_messages(&self) -> impl Iterator<Item = &MessageDB> + '_ {
        self.messages_order.iter().filter_map(|&k| self.messages.get(k))
    }
    pub fn iter_signals(&self) -> impl Iterator<Item = &SignalDB> + '_ {
        self.signals_order.iter().filter_map(|&k| self.signals.get(k))
    }

    /// Sort nodes_by_name
    pub fn sort_nodes_by_name(&mut self) {
        self.nodes_order.sort_by_key(|&k| self.nodes.get(k).map(|n| n.name.to_ascii_lowercase()));
    }

    /// Sort messages_by_name case insensitive
    pub fn sort_messages_by_name(&mut self) {
        self.messages_order
            .sort_by_key(|&k| self.messages.get(k).map(|m| m.name.to_ascii_lowercase()));
    }

    /// Sort signals_by_name case insensitive
    pub fn sort_signals_by_name(&mut self) {
        self.signals_order
            .sort_by_key(|&k| self.signals.get(k).map(|s| s.name.to_ascii_lowercase()));
    }

    /// Clear the database
    pub fn clear(&mut self) {
        *self = Database::default();
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
    pub sender_nodes: Vec<NodeKey>,
    /// Signals that belong to this message.
    pub signals: Vec<SignalKey>,
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
    pub fn signals<'a>(&'a self, db: &'a Database) -> impl Iterator<Item = &'a SignalDB> + 'a {
        self.signals.iter().filter_map(move |&key| db.get_sig_by_key(key))
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
    /// Parent message key.
    pub message: MessageKey,
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
    pub receiver_nodes: Vec<NodeKey>,
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
        let key = name.to_lowercase();
        self.receiver_nodes
            .iter()
            .filter_map(|&node_key| db.get_node_by_key(node_key))
            .find(|node| node.name.to_lowercase() == key)
    }

    /// Returns a mutable reference to a receiver node by name (case-insensitive).
    pub fn get_receiver_nodes_by_name_mut<'a>(
        &self,
        db: &'a mut Database,
        name: &str,
    ) -> Option<&'a mut NodeDB> {
        let input_name: String = name.to_lowercase();
        let nkey = self.receiver_nodes.iter().copied().find(|&node_key| {
            db.get_node_by_key(node_key)
                .map(|n| n.name.to_lowercase() == input_name)
                .unwrap_or(false)
        })?;
        db.get_node_by_key_mut(nkey)
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

    /// Converts a raw value into an "instantaneous" `SignalLog` with physical value, text, and metadata.
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
    pub messages_sent: Vec<MessageKey>,
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
