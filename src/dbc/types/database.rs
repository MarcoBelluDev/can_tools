//! DatabaseDBC model (SlotMap-backed).
//!
//! This module defines the in-memory **CAN database** used by the DBC/ARXML parsers.
//! Storage uses **SlotMap** arenas with **stable keys**: [`NodeKey`], [`MessageKey`], [`SignalKey`].
//! Public iteration follows **order vectors** via `iter_nodes()`, `iter_messages()`, `iter_signals()`
//! and you can reorder presentation with `sort_nodes_by_name()`, `sort_messages_by_name()`, `sort_signals_by_name()`.
//!
//! **Lookups** are normalized and O(1): `get_message_by_id/_hex/_name`, `get_node_by_name`, `get_signal_by_name`.
//! Names are case-insensitive; hexadecimal IDs use uppercase `0x...` form.
//!
//! Signal decoding utilities live on [`SignalDBC`]: `compile_inline()`, `extract_raw_u64/i64()`, and `to_sigframe()`.
//!
//! _Docs refreshed: 2025-08-22_
//!

use slotmap::{SlotMap, new_key_type};
use std::collections::{BTreeMap, HashMap};

use crate::dbc::types::{
    attributes::{AttributeSpec, AttributeValue},
    message::{IdFormat, MessageDBC, MuxInfo, MuxRole, MuxSelector},
    node::NodeDBC,
    signal::SignalDBC,
};

// --- Stable keys (SlotMap) ---
new_key_type! { pub struct NodeKey; }
new_key_type! { pub struct MessageKey; }
new_key_type! { pub struct SignalKey; }

/// In-memory representation of a CAN database (DBC).
///
/// Holds metadata (name, bus type, baud rates, version), the arenas of nodes/messages/signals
/// (SlotMaps with stable keys), optional order vectors to control iteration order, and
/// several normalized lookup maps for efficient queries.
#[derive(Default, Clone, Debug)]
pub struct DatabaseDBC {
    // --- General information ---
    /// DatabaseDBC version string.
    pub name: String,
    /// DatabaseDBC version string.
    pub version: String,
    /// DatabaseDBC comment.
    pub comment: String,

    // --- Main storage (stable-key maps) ---
    pub nodes: SlotMap<NodeKey, NodeDBC>,
    pub messages: SlotMap<MessageKey, MessageDBC>,
    pub signals: SlotMap<SignalKey, SignalDBC>,

    // --- Order "views"  ---
    pub nodes_order: Vec<NodeKey>,
    pub messages_order: Vec<MessageKey>,
    pub signals_order: Vec<SignalKey>,

    // --- DB Attribute Entry ---
    pub attributes: BTreeMap<String, AttributeValue>,

    // --- Attributes Spec ---
    pub db_attr_spec: BTreeMap<String, AttributeSpec>,
    pub node_attr_spec: BTreeMap<String, AttributeSpec>,
    pub msg_attr_spec: BTreeMap<String, AttributeSpec>,
    pub sig_attr_spec: BTreeMap<String, AttributeSpec>,

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

impl DatabaseDBC {
    // --------- Nodes --------
    /// Adds a node to the database if not already present and returns the corresponding `NodeKey`.
    pub fn add_node_if_absent(&mut self, name: &str) -> NodeKey {
        if let Some(r) = self.get_node_key_by_name(name) {
            return r;
        }
        let key: NodeKey = self.nodes.insert(NodeDBC {
            name: name.to_string(),
            ..Default::default()
        });
        self.nodes_order.push(key);
        self.node_key_by_name.insert(name.to_lowercase(), key);
        key
    }

    /// Insert `MessageKey` in `messages_sent` of Node `nk`. No duplicates.
    pub fn add_tx_msg_for_node(&mut self, nk: NodeKey, msg_key: MessageKey) {
        // Check that the message exist
        if self.get_message_by_key(msg_key).is_none() {
            return;
        }

        // take signals of the message as immutable borrow
        let msg_signals: Vec<SignalKey> = {
            let Some(msg) = self.get_message_by_key(msg_key) else {
                return;
            };
            msg.signals.clone()
        };

        // Update the node taking it as mutable borrow
        if let Some(node) = self.get_node_by_key_mut(nk) {
            // trasmitted message update
            if !node.messages_sent.contains(&msg_key) {
                node.messages_sent.push(msg_key);
            }

            // trasmitted signals update
            for sk in msg_signals {
                if !node.signals_sent.contains(&sk) {
                    node.signals_sent.push(sk);
                }
            }
        }
    }

    pub fn get_node_key_by_name(&self, name: &str) -> Option<NodeKey> {
        self.node_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub fn get_node_by_key(&self, key: NodeKey) -> Option<&NodeDBC> {
        self.nodes.get(key)
    }

    pub fn get_node_by_key_mut(&mut self, key: NodeKey) -> Option<&mut NodeDBC> {
        self.nodes.get_mut(key)
    }

    /// Returns a `&NodeDBC` given the name (case-insensitive).
    pub fn get_node_by_name(&self, name: &str) -> Option<&NodeDBC> {
        let key: NodeKey = *self.node_key_by_name.get(&name.to_lowercase())?;
        self.get_node_by_key(key)
    }

    /// Returns a `&mut NodeDBC` given the name (case-insensitive).
    pub fn get_node_by_name_mut(&mut self, name: &str) -> Option<&mut NodeDBC> {
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

        let msg_key: MessageKey = self.messages.insert(MessageDBC {
            attributes: BTreeMap::default(),
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
            sender_nodes: sender_node_id.into_iter().collect(),
            signals: Vec::new(),
            comment: String::new(),
            mux_multiplexors: Vec::new(),
            mux_cases: HashMap::new(),
        });

        self.messages_order.push(msg_key);

        self.msg_key_by_id.insert(id, msg_key);
        self.msg_key_by_hex.insert(id_hex, msg_key);
        self.msg_key_by_name.insert(name.to_lowercase(), msg_key);

        if let Some(nid) = sender_node_id
            && let Some(n) = self.nodes.get_mut(nid)
        {
            n.messages_sent.push(msg_key);
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

    pub fn get_message_by_key(&self, key: MessageKey) -> Option<&MessageDBC> {
        self.messages.get(key)
    }

    pub fn get_message_by_key_mut(&mut self, key: MessageKey) -> Option<&mut MessageDBC> {
        self.messages.get_mut(key)
    }

    /// Returns a `&MessageDBC` given the numeric CAN ID.
    pub fn get_message_by_id(&self, id: u32) -> Option<&MessageDBC> {
        let key: MessageKey = self.get_msg_key_by_id(&id)?;
        self.get_message_by_key(key)
    }

    /// Returns a `&mut MessageDBC` given the numeric CAN ID.
    pub fn get_message_by_id_mut(&mut self, id: u32) -> Option<&mut MessageDBC> {
        let key: MessageKey = self.get_msg_key_by_id(&id)?;
        self.get_message_by_key_mut(key)
    }

    /// Returns a `&MessageDBC` given a hexadecimal ID (case-insensitive).
    ///
    /// The argument may come in various forms, e.g., `"12dd54e3"`, `"0x12dd54e3"`, `"12DD54E3x"`;
    /// it will be normalized internally to `"0x12DD54E3"`.
    pub fn get_message_by_id_hex(&self, id_hex: &str) -> Option<&MessageDBC> {
        let key: MessageKey = self.get_msg_key_by_id_hex(id_hex)?;
        self.get_message_by_key(key)
    }

    /// Returns a `&mut MessageDBC` given a hexadecimal ID (case-insensitive).
    pub fn get_message_by_id_hex_mut(&mut self, id_hex: &str) -> Option<&mut MessageDBC> {
        let key: MessageKey = self.get_msg_key_by_id_hex(id_hex)?;
        self.get_message_by_key_mut(key)
    }

    /// Returns a `&MessageDBC` given the name (case-insensitive).
    pub fn get_message_by_name(&self, name: &str) -> Option<&MessageDBC> {
        let key: MessageKey = self.get_msg_key_by_name(name)?;
        self.get_message_by_key(key)
    }

    /// Returns a `&mut MessageDBC` given the name (case-insensitive).
    pub fn get_message_by_name_mut(&mut self, name: &str) -> Option<&mut MessageDBC> {
        let key: MessageKey = self.get_msg_key_by_name(name)?;
        self.get_message_by_key_mut(key)
    }

    // -------------- Signals ------------
    // Adds a signal to the database if not already present and returns the corresponding `SignalKey`.
    // valid only during construction of DB from .dbc because of current_message!
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
        mux_role: MuxRole,
        mux_selectors: &[MuxSelector],
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

        // if signal is Multiplexed, try to guess the Multiplexor if there is only one in the message
        let inferred_switch: Option<SignalKey> = if mux_role == MuxRole::Multiplexed {
            if let Some(msg) = self.get_message_by_key(msg_key) {
                if msg.mux_multiplexors.len() == 1 {
                    Some(msg.mux_multiplexors[0])
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let mux: Option<MuxInfo> = if mux_role == MuxRole::None {
            None
        } else {
            Some(MuxInfo {
                role: mux_role,
                group: 0,
                switch: inferred_switch,
                selectors: mux_selectors.to_vec(),
            })
        };

        let mut sig: SignalDBC = SignalDBC {
            attributes: BTreeMap::default(),
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
            mux,
        };
        sig.compile_inline();

        let sig_key: SignalKey = self.signals.insert(sig);
        self.signals_order.push(sig_key);

        // add the signal within current message
        if let Some(m) = self.messages.get_mut(msg_key)
            && !m.signals.contains(&sig_key)
        {
            m.signals.push(sig_key);
        }

        // --- update message multiplexing info ---
        match mux_role {
            MuxRole::None => { /* Nothing to do */ }
            MuxRole::Multiplexor => {
                // Register the Multiplexor inside proper message list
                if let Some(m) = self.get_message_by_key_mut(msg_key)
                    && !m.mux_multiplexors.contains(&sig_key)
                {
                    m.mux_multiplexors.push(sig_key);
                }

                // link dependant signals with no Multiplexor yet to this new Multiplexor
                // Usually, this should never happen because Multiplexor must always be first line in a message
                let dep_to_attach: Vec<(SignalKey, Vec<MuxSelector>)> = {
                    let msg: &MessageDBC = self.get_message_by_key(msg_key).unwrap();
                    msg.signals
                        .iter()
                        .copied()
                        .filter_map(|sk| {
                            let s = self.get_sig_by_key(sk)?;
                            let mi = s.mux.as_ref()?;
                            if mi.role == MuxRole::Multiplexed && mi.switch.is_none() {
                                Some((sk, mi.selectors.clone()))
                            } else {
                                None
                            }
                        })
                        .collect()
                };

                // Update the signals and the mux_cases
                for (sk, sels) in dep_to_attach {
                    // set the Multiplexor to the signal
                    if let Some(s) = self.get_sig_by_key_mut(sk)
                        && let Some(mi) = s.mux.as_mut()
                        && mi.role == MuxRole::Multiplexor
                        && mi.switch.is_none()
                    {
                        mi.switch = Some(sig_key);
                    }
                    // Update the map of the message
                    if let Some(m) = self.get_message_by_key_mut(msg_key) {
                        let by_sel = m.mux_cases.entry(sig_key).or_default();
                        for sel in &sels {
                            by_sel.entry(sel.clone()).or_default().push(sk);
                        }
                    }
                }
            }
            MuxRole::Multiplexed => {
                if let Some(sw) = inferred_switch
                    && let Some(m) = self.get_message_by_key_mut(msg_key)
                {
                    let by_sel = m.mux_cases.entry(sw).or_default();
                    for sel in mux_selectors {
                        by_sel.entry(sel.clone()).or_default().push(sig_key);
                    }
                }
            }
        }

        self.sig_key_by_name.insert(name.to_lowercase(), sig_key);
        sig_key
    }

    pub fn get_sig_key_by_name(&self, name: &str) -> Option<SignalKey> {
        self.sig_key_by_name.get(&name.to_lowercase()).copied()
    }

    pub fn get_sig_by_key(&self, key: SignalKey) -> Option<&SignalDBC> {
        self.signals.get(key)
    }

    pub fn get_sig_by_key_mut(&mut self, key: SignalKey) -> Option<&mut SignalDBC> {
        self.signals.get_mut(key)
    }

    /// Returns a `&SignalDBC` given the name (case-insensitive).
    pub fn get_signal_by_name(&self, name: &str) -> Option<&SignalDBC> {
        let key: SignalKey = *self.sig_key_by_name.get(&name.to_lowercase())?;
        self.get_sig_by_key(key)
    }

    /// Returns a `&mut SignalDBC` given the name (case-insensitive).
    pub fn get_signal_by_name_mut(&mut self, name: &str) -> Option<&mut SignalDBC> {
        let key: SignalKey = *self.sig_key_by_name.get(&name.to_lowercase())?;
        self.get_sig_by_key_mut(key)
    }

    /// Iterators according to the orders (defualt order is name based)
    pub fn iter_nodes(&self) -> impl Iterator<Item = &NodeDBC> + '_ {
        self.nodes_order.iter().filter_map(|&k| self.nodes.get(k))
    }
    /// Iterate messages following `messages_order`. If empty, insertion order is used.
    pub fn iter_messages(&self) -> impl Iterator<Item = &MessageDBC> + '_ {
        self.messages_order
            .iter()
            .filter_map(|&k| self.messages.get(k))
    }
    /// Iterate signals following `signals_order`. If empty, insertion order is used.
    pub fn iter_signals(&self) -> impl Iterator<Item = &SignalDBC> + '_ {
        self.signals_order
            .iter()
            .filter_map(|&k| self.signals.get(k))
    }

    // -------------- Sorting ---------------
    /// Sort nodes_by_name case insensitive
    pub fn sort_db_nodes_by_name(&mut self) {
        self.nodes_order
            .sort_by_key(|&k| self.nodes.get(k).map(|n| n.name.to_ascii_lowercase()));
    }

    /// Sort messages_by_name case insensitive
    pub fn sort_db_messages_by_name(&mut self) {
        self.messages_order
            .sort_by_key(|&k| self.messages.get(k).map(|m| m.name.to_ascii_lowercase()));
    }

    /// Sort signals_by_name case insensitive
    pub fn sort_db_signals_by_name(&mut self) {
        self.signals_order
            .sort_by_key(|&k| self.signals.get(k).map(|s| s.name.to_ascii_lowercase()));
    }

    /// Sort `messages_sent`, `signals_sent` and `signals_read` inside the specific given NodeDBC
    /// by the target names (ASCII case-insensitive).
    pub fn sort_node_fields(&mut self, node_key: NodeKey) {
        // Compute the new order on immutable borrows
        let (sorted_msgs, sorted_sigs_sent, sorted_sigs_received) = {
            let Some(node) = self.get_node_by_key(node_key) else {
                return;
            };

            // messages_sent -> by MessageDBC.name
            let mut ms: Vec<MessageKey> = node.messages_sent.clone();
            ms.sort_by_key(|&mk| {
                self.get_message_by_key(mk)
                    .map(|m| m.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            // signals_sent -> by SignalDBC.name
            let mut sr1: Vec<SignalKey> = node.signals_sent.clone();
            sr1.sort_by_key(|&sk| {
                self.get_sig_by_key(sk)
                    .map(|s| s.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            // signals_read -> by SignalDBC.name
            let mut sr2: Vec<SignalKey> = node.signals_read.clone();
            sr2.sort_by_key(|&sk| {
                self.get_sig_by_key(sk)
                    .map(|s| s.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            (ms, sr1, sr2)
        };

        // Write back with a mutable borrow
        if let Some(node) = self.get_node_by_key_mut(node_key) {
            node.messages_sent = sorted_msgs;
            node.signals_sent = sorted_sigs_sent;
            node.signals_read = sorted_sigs_received;
        }
    }

    /// Sort `sender_nodes` and `signals` inside the specific given MessageDBC
    /// by the target names (ASCII case-insensitive).
    pub fn sort_message_fields(&mut self, msg_key: MessageKey) {
        let (sorted_nodes, sorted_sigs) = {
            let Some(msg) = self.get_message_by_key(msg_key) else {
                return;
            };

            // sender_nodes -> by NodeDBC.name
            let mut ns: Vec<NodeKey> = msg.sender_nodes.clone();
            ns.sort_by_key(|&nk| {
                self.get_node_by_key(nk)
                    .map(|n| n.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            // signals -> by SignalDBC.name
            let mut ss: Vec<SignalKey> = msg.signals.clone();
            ss.sort_by_key(|&sk| {
                self.get_sig_by_key(sk)
                    .map(|s| s.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            (ns, ss)
        };

        if let Some(msg) = self.get_message_by_key_mut(msg_key) {
            msg.sender_nodes = sorted_nodes;
            msg.signals = sorted_sigs;
        }
    }

    /// Sort `receiver_nodes` inside the specific given SignalDBC
    /// by the target names (ASCII case-insensitive).
    pub fn sort_signal_fields(&mut self, sig_key: SignalKey) {
        let sorted_nodes: Vec<NodeKey> = {
            let Some(sig) = self.get_sig_by_key(sig_key) else {
                return;
            };

            // receiver_nodes -> by NodeDBC.name
            let mut ns: Vec<NodeKey> = sig.receiver_nodes.clone();
            ns.sort_by_key(|&nk| {
                self.get_node_by_key(nk)
                    .map(|n| n.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            ns
        };

        if let Some(sig) = self.get_sig_by_key_mut(sig_key) {
            sig.receiver_nodes = sorted_nodes;
        }
    }

    /// For ALL NodeDBC entries, sort:
    /// - `messages_sent`  by the target MessageDBC.name (ASCII case-insensitive)
    /// - `signals_sent`  by the target MessageDBC.name (ASCII case-insensitive)
    /// - `signals_read`   by the target SignalDBC.name  (ASCII case-insensitive)
    ///
    /// Missing/invalid keys are pushed to the end; ties are broken by the key for determinism.
    pub fn sort_all_node_fields(&mut self) {
        // Build write plans using only immutable borrows (avoids borrow conflicts).
        let plans: Vec<NodePlan> = self
            .nodes
            .iter()
            .map(|(nk, node)| {
                // messages_sent -> sort by message name (case-insensitive)
                let mut ms: Vec<MessageKey> = node.messages_sent.clone();
                ms.sort_by_cached_key(|&mk| {
                    let (missing, name) = match self.get_message_by_key(mk) {
                        Some(m) => (false, m.name.to_ascii_lowercase()),
                        None => (true, String::new()),
                    };
                    (missing, name, mk) // missing last, then by lowercase name, then key as tie-breaker
                });

                // signals_sent -> sort by signal name (case-insensitive)
                let mut sr1: Vec<SignalKey> = node.signals_sent.clone();
                sr1.sort_by_cached_key(|&sk| {
                    let (missing, name) = match self.get_sig_by_key(sk) {
                        Some(s) => (false, s.name.to_ascii_lowercase()),
                        None => (true, String::new()),
                    };
                    (missing, name, sk)
                });

                // signals_read -> sort by signal name (case-insensitive)
                let mut sr2: Vec<SignalKey> = node.signals_read.clone();
                sr2.sort_by_cached_key(|&sk| {
                    let (missing, name) = match self.get_sig_by_key(sk) {
                        Some(s) => (false, s.name.to_ascii_lowercase()),
                        None => (true, String::new()),
                    };
                    (missing, name, sk)
                });
                NodePlan {
                    nk,
                    messages_sent: ms,
                    signals_sent: sr1,
                    signals_read: sr2,
                }
            })
            .collect();

        // Apply the plans with mutable borrows.
        for p in plans {
            if let Some(node) = self.get_node_by_key_mut(p.nk) {
                node.messages_sent = p.messages_sent;
                node.signals_sent = p.signals_sent;
                node.signals_read = p.signals_read;
            }
        }
    }

    /// For ALL MessageDBC entries, sort:
    /// - `sender_nodes` by NodeDBC.name    (ASCII case-insensitive)
    /// - `signals`      by SignalDBC.name  (ASCII case-insensitive)
    ///
    /// Missing/invalid keys are pushed to the end; ties are broken by the key.
    pub fn sort_all_message_fields(&mut self) {
        let plans: Vec<(MessageKey, Vec<NodeKey>, Vec<SignalKey>)> = self
            .messages
            .iter()
            .map(|(mk, msg)| {
                // sender_nodes -> sort by node name (case-insensitive)
                let mut ns = msg.sender_nodes.clone();
                ns.sort_by_cached_key(|&nk| {
                    let (missing, name) = match self.get_node_by_key(nk) {
                        Some(n) => (false, n.name.to_ascii_lowercase()),
                        None => (true, String::new()),
                    };
                    (missing, name, nk)
                });

                // signals -> sort by signal name (case-insensitive)
                let mut ss = msg.signals.clone();
                ss.sort_by_cached_key(|&sk| {
                    let (missing, name) = match self.get_sig_by_key(sk) {
                        Some(s) => (false, s.name.to_ascii_lowercase()),
                        None => (true, String::new()),
                    };
                    (missing, name, sk)
                });

                (mk, ns, ss)
            })
            .collect();

        for (mk, ns, ss) in plans {
            if let Some(msg) = self.get_message_by_key_mut(mk) {
                msg.sender_nodes = ns;
                msg.signals = ss;
            }
        }
    }

    /// For ALL SignalDBC entries, sort:
    /// - `receiver_nodes` by NodeDBC.name (ASCII case-insensitive)
    ///
    /// Missing/invalid keys are pushed to the end; ties are broken by the key.
    pub fn sort_all_signal_fields(&mut self) {
        let plans: Vec<(SignalKey, Vec<NodeKey>)> = self
            .signals
            .iter()
            .map(|(sk, sig)| {
                let mut ns = sig.receiver_nodes.clone();
                ns.sort_by_cached_key(|&nk| {
                    let (missing, name) = match self.get_node_by_key(nk) {
                        Some(n) => (false, n.name.to_ascii_lowercase()),
                        None => (true, String::new()),
                    };
                    (missing, name, nk)
                });
                (sk, ns)
            })
            .collect();

        for (sk, ns) in plans {
            if let Some(sig) = self.get_sig_by_key_mut(sk) {
                sig.receiver_nodes = ns;
            }
        }
    }

    /// Clear the database
    pub fn clear(&mut self) {
        *self = DatabaseDBC::default();
    }
}

/// Bus type for a DBC-backed database.
#[derive(Default, Clone, PartialEq, Debug)]
pub enum BusType {
    #[default]
    Can,
    CanFd,
}

impl BusType {
    /// Returns a user-friendly string (e.g., `"CAN"`, `"CAN FD"`).
    pub fn to_str(&self) -> String {
        match self {
            BusType::Can => "CAN".to_string(),
            BusType::CanFd => "CAN FD".to_string(),
        }
    }
}

// suport struct for node parsing
#[derive(Debug, Clone)]
struct NodePlan {
    nk: NodeKey,
    messages_sent: Vec<MessageKey>,
    signals_sent: Vec<SignalKey>,
    signals_read: Vec<SignalKey>,
}
