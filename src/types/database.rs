//! Database model (SlotMap-backed).
//!
//! This module defines the in-memory **CAN database** used by the DBC/ARXML parsers.
//! Storage uses **SlotMap** arenas with **stable keys**: [`NodeKey`], [`MessageKey`], [`SignalKey`].
//! Public iteration follows **order vectors** via `iter_nodes()`, `iter_messages()`, `iter_signals()`
//! and you can reorder presentation with `sort_nodes_by_name()`, `sort_messages_by_name()`, `sort_signals_by_name()`.
//!
//! **Lookups** are normalized and O(1): `get_message_by_id/_hex/_name`, `get_node_by_name`, `get_signal_by_name`.
//! Names are case-insensitive; hexadecimal IDs use uppercase `0x...` form.
//!
//! Signal decoding utilities live on [`SignalDB`]: `compile_inline()`, `extract_raw_u64/i64()`, and `to_sigframe()`.
//!
//! _Docs refreshed: 2025-08-22_
//!

use slotmap::{SlotMap, new_key_type};
use std::collections::HashMap;
use std::cmp::Ordering;

use crate::{IdFormat, MessageDB, NodeDB, SignalDB};

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
    pub bustype: BusType,
    /// Classic baud rate (bit/s). `0` if unspecified.
    pub baudrate: u32,
    /// CAN FD baud rate (bit/s). `0` if unspecified.
    pub baudrate_canfd: u32,
    /// Database version string.
    pub version: String,
    /// Database comment.
    pub comment: String,

    // --- Main storage (stable-key maps) ---
    pub nodes: SlotMap<NodeKey, NodeDB>,
    pub messages: SlotMap<MessageKey, MessageDB>,
    pub signals: SlotMap<SignalKey, SignalDB>,

    // --- Order "views"  ---
    pub nodes_order: Vec<NodeKey>,
    pub messages_order: Vec<MessageKey>,
    pub signals_order: Vec<SignalKey>,

    // --- CANoe CAPL-Generator parameters ---
    pub gen_nwm_talk_nm: String,
    pub gen_nwm_goto_mode_awake: String,
    pub gen_nwm_goto_mode_bus_sleep: String,

    // --- Network managment parameter ---
    pub nm_type: String,

    // --- Other parameters ---
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
    pub gen_nwm_sleep_time: u16,
    pub gen_nwm_ap_can_wake_up: String,
    pub gen_nwm_ap_can_sleep: String,
    pub gen_nwm_ap_can_on: String,
    pub gen_nwm_ap_can_off: String,
    pub gen_nwm_ap_can_normal: String,
    pub gen_nwm_ap_bus_sleep: String,

    // --- Lookups (case-normalized) ---
    pub(crate) node_key_by_name: HashMap<String, NodeKey>, // lower(name) → NodeKey
    pub(crate) msg_key_by_id: HashMap<u32, MessageKey>,    // id10 → MessageKey
    pub(crate) msg_key_by_hex: HashMap<String, MessageKey>, // "0x...." uppercase → MessageKey
    pub(crate) msg_key_by_name: HashMap<String, MessageKey>, // lower(name) → MessageKey

    // Global map for signals by (lower) name. Beware of collisions if two BO_ have same SG_ name.
    pub(crate) sig_key_by_name: HashMap<String, SignalKey>,

    // Parsing state: last message seen (used by SG_ decoder)
    pub(crate) current_msg: Option<MessageKey>,
}

impl Database {
    // --------- Nodes --------
    /// Adds a node to the database if not already present and returns the corresponding `NodeKey`.
    pub fn add_node_if_absent(&mut self, name: &str) -> NodeKey {
        if let Some(r) = self.get_node_key_by_name(name) {
            return r;
        }
        let key: NodeKey = self.nodes.insert(NodeDB {
            name: name.to_string(),
            ..Default::default()
        });
        self.nodes_order.push(key);
        self.node_key_by_name.insert(name.to_lowercase(), key);
        key
    }

    /// Insert `MessageKey` in `messages_sent` of Node `nk`
    /// Keep case-insensitive alfabetical order. No duplicates.
    pub fn add_tx_msg_for_node(&mut self, nk: NodeKey, msg_key: MessageKey) {
        let Some(target_name) = self.get_message_by_key(msg_key).map(|m| m.name.as_str()) else {
            return;
        };

        // immutable borrow to get the position
        let insert_pos: usize = {
            let Some(node_ro) = self.get_node_by_key(nk) else { return; };

            match node_ro.messages_sent.binary_search_by(|k| {
                let name = self.get_message_by_key(*k).map(|m| m.name.as_str()).unwrap_or("");
                // order by name (case insensitive)
                cmp_ascii_ci(name, target_name).then_with(|| k.cmp(&msg_key))
            }) {
                Ok(_) => return,       // already present, nothing to do
                Err(ins) => ins,       // insert position
            }
        };

        // Mutable borrow to write
        if let Some(node_rw) = self.get_node_by_key_mut(nk) {
            node_rw.messages_sent.insert(insert_pos, msg_key);
        }
    }

    pub fn get_node_key_by_name(&self, name: &str) -> Option<NodeKey> {
        self.node_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub fn get_node_by_key(&self, key: NodeKey) -> Option<&NodeDB> {
        self.nodes.get(key)
    }

    pub fn get_node_by_key_mut(&mut self, key: NodeKey) -> Option<&mut NodeDB> {
        self.nodes.get_mut(key)
    }

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

    // ------------- Messages ------------
    /// Adds a message and indexes its id/name. Also sets `current_msg` for subsequent SG_ lines.
    pub(crate) fn add_message_if_absent(
        &mut self,
        name: &str,
        id: u32,
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

        let id_hex: String = id_hex.to_string();
        let id_format: IdFormat = if id > 2048 {
            IdFormat::Extended
        } else {
            IdFormat::Standard
        };

        let msg_key: MessageKey = self.messages.insert(MessageDB {
            id_format,
            id,
            id_hex: id_hex.clone(),
            name: name.to_string(),
            byte_length,
            msgtype: if byte_length <= 8 {
                "CAN".into()
            } else {
                "CAN FD".into()
            },
            cycle_time: 0,
            sender_nodes: sender_node_id.into_iter().collect(),
            signals: Vec::new(),
            comment: String::new(),
            mux_switches: Vec::new(),
            mux_cases: HashMap::new(),
        });

        self.messages_order.push(msg_key);

        self.msg_key_by_id.insert(id, msg_key);
        self.msg_key_by_hex.insert(id_hex, msg_key);
        self.msg_key_by_name.insert(name.to_lowercase(), msg_key);

        if let Some(nid) = sender_node_id {
            if let Some(n) = self.nodes.get_mut(nid) {
                n.messages_sent.push(msg_key);
            }
        }

        self.current_msg = Some(msg_key);
        msg_key
    }

    pub fn get_msg_key_by_name(&self, name: &str) -> Option<MessageKey> {
        self.msg_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub fn get_msg_key_by_id(&self, id: &u32) -> Option<MessageKey> {
        self.msg_key_by_id.get(id).copied()
    }

    pub fn get_msg_key_by_id_hex(&self, id_hex: &str) -> Option<MessageKey> {
        // let key: String = normalize_id_hex(id_hex); // "0x...UPPERCASE"
        self.msg_key_by_hex.get(id_hex).copied()
    }

    pub fn get_message_by_key(&self, key: MessageKey) -> Option<&MessageDB> {
        self.messages.get(key)
    }

    pub fn get_message_by_key_mut(&mut self, key: MessageKey) -> Option<&mut MessageDB> {
        self.messages.get_mut(key)
    }

    /// Returns a `&MessageDB` given the numeric CAN ID.
    pub fn get_message_by_id(&self, id: u32) -> Option<&MessageDB> {
        let key: MessageKey = self.get_msg_key_by_id(&id)?;
        self.get_message_by_key(key)
    }

    /// Returns a `&mut MessageDB` given the numeric CAN ID.
    pub fn get_message_by_id_mut(&mut self, id: u32) -> Option<&mut MessageDB> {
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


    // -------------- Signals ------------
    /// Adds a signal to the database if not already present and returns the corresponding `SignalKey`.
    #[allow(clippy::too_many_arguments)]
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

        let msg_key: MessageKey = match self.current_msg {
            Some(k) => k,
            None => {
                // Create a fallback message if an SG_ appears before any BO_ (rare).
                self.add_message_if_absent("__UNBOUND__", 0, "0x0", 8, "")
            }
        };

        let mut sig: SignalDB = SignalDB {
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
            mux: None, 
        };
        sig.compile_inline();

        let sig_key: SignalKey = self.signals.insert(sig);
        self.signals_order.push(sig_key);

        if let Some(m) = self.messages.get_mut(msg_key) {
            if !m.signals.contains(&sig_key) {
                m.signals.push(sig_key);
            }
        }

        self.sig_key_by_name.insert(name.to_lowercase(), sig_key);
        sig_key
    }
    
    pub fn get_sig_key_by_name(&self, name: &str) -> Option<SignalKey> {
        self.sig_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub fn get_sig_by_key(&self, key: SignalKey) -> Option<&SignalDB> {
        self.signals.get(key)
    }

    pub fn get_sig_by_key_mut(&mut self, key: SignalKey) -> Option<&mut SignalDB> {
        self.signals.get_mut(key)
    }

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
    /// Iterate messages following `messages_order`. If empty, insertion order is used.
    pub fn iter_messages(&self) -> impl Iterator<Item = &MessageDB> + '_ {
        self.messages_order
            .iter()
            .filter_map(|&k| self.messages.get(k))
    }
    /// Iterate signals following `signals_order`. If empty, insertion order is used.
    pub fn iter_signals(&self) -> impl Iterator<Item = &SignalDB> + '_ {
        self.signals_order
            .iter()
            .filter_map(|&k| self.signals.get(k))
    }

    // -------------- Sorting ---------------
    /// Sort nodes_by_name
    pub fn sort_nodes_by_name(&mut self) {
        self.nodes_order
            .sort_by_key(|&k| self.nodes.get(k).map(|n| n.name.to_ascii_lowercase()));
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

#[derive(Default, Clone, PartialEq, Debug)]
pub enum Present {
    Yes,
    #[default]
    No,
}

impl Present {
    pub fn to_str(&self) -> String {
        match self {
            Present::Yes => "Yes".to_string(),
            Present::No => "No".to_string(),
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub enum BusType {
    #[default]
    Can,
    CanFd,
}

impl BusType {
    pub fn to_str(&self) -> String {
        match self {
            BusType::Can => "CAN".to_string(),
            BusType::CanFd => "CAN FD".to_string(),
        }
    }
}


fn cmp_ascii_ci(a: &str, b: &str) -> Ordering {
    a.as_bytes().iter().map(u8::to_ascii_lowercase)
        .cmp(b.as_bytes().iter().map(u8::to_ascii_lowercase))
}