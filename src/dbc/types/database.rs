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
//! Signal decoding utilities live on [`SignalDBC`]: `compile_inline()`, `extract_raw_u64/i64()`.
//! Conversion to `SignalLog` is provided under `asc::core::signal_conversion` when the `asc` feature is enabled.
//!
//! Docs updated: 2025-10-09 — refreshed field documentation and clarified ordering invariants.
//!

use slotmap::{Key, SlotMap, new_key_type};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::dbc::{
    core::message_layout,
    types::{
        attributes::{AttributeSpec, AttributeValue},
        errors::DatabaseError,
        message::{IdFormat, MessageDBC, MuxInfo, MuxRole, MuxSelector},
        node::NodeDBC,
        signal::{Endianness, SignalDBC, Signess},
    },
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
    /// Human-readable database name (`BA_ "DBName"`), empty if absent.
    pub name: String,
    /// Bus type advertised by `BA_ "BusType"` (defaults to `BusType::Can`).
    pub bustype: BusType,
    /// Free-form version string parsed from the `VERSION` line.
    pub version: String,
    /// Global database comment (populated by the standalone `CM_ "..."` statement).
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

    // --- Relational Attributes Spec ---
    // Definitions (BA_DEF_REL_) and defaults (BA_DEF_DEF_REL_) for attributes that
    // apply to relations between two entities.
    //
    // Vector DBC supports at least these relation kinds:
    // - BU_SG_REL_: Node ↔ Signal
    // - BU_BO_REL_: Node ↔ Message
    pub rel_attr_spec_bu_sg: BTreeMap<String, AttributeSpec>,
    pub rel_attr_spec_bu_bo: BTreeMap<String, AttributeSpec>,

    // --- Lookups (case-normalized) ---
    /// Global map for nodes by (lower) name.
    pub node_key_by_name: HashMap<String, NodeKey>, // lower(name) → NodeKey
    /// Global map for messages by id.
    pub msg_key_by_id: HashMap<u32, MessageKey>, // id10 → MessageKey
    /// Global map for messages by id_hex.
    pub msg_key_by_hex: HashMap<String, MessageKey>, // "0x...." uppercase → MessageKey
    /// Global map for messages by (lower) name.
    pub msg_key_by_name: HashMap<String, MessageKey>, // lower(name) → MessageKey
    /// Global map for signals by (lower) name. Beware of collisions if two BO_ have same SG_ name.
    pub sig_key_by_name: HashMap<String, SignalKey>, // lower(name) → SignalKey

    // Parsing state: last message seen (used by SG_ decoder)
    pub(crate) current_msg: Option<MessageKey>,

    // --- Relational Attributes (BA_REL_) ---
    // Concrete values attached to a pair of entities.
    // Attribute names are kept sorted (BTreeMap) for stable iteration.
    // Keys for pairs use HashMap since order is not important and SlotMap keys are hashable.
    /// BU_SG_REL_: attributes on (Node, Signal) pairs.
    pub bu_sg_rel_attributes: HashMap<(NodeKey, SignalKey), BTreeMap<String, AttributeValue>>,
    /// BU_BO_REL_: attributes on (Node, Message) pairs.
    pub bu_bo_rel_attributes: HashMap<(NodeKey, MessageKey), BTreeMap<String, AttributeValue>>,
}

impl DatabaseDBC {
    // --------- Nodes --------
    /// Adds a node to the database, seeding attributes with spec defaults, and returns the `NodeKey`.
    pub fn add_node(&mut self, name: &str) -> Result<NodeKey, DatabaseError> {
        // check that the node name is not already present
        if self.get_node_key_by_name(name).is_some() {
            return Err(DatabaseError::NodeAlreadyExists {
                name: name.to_string(),
            });
        }

        let mut node: NodeDBC = NodeDBC {
            name: name.to_string(),
            ..Default::default()
        };

        for (attr_name, spec) in &self.node_attr_spec {
            if let Some(default_value) = spec.default.as_ref() {
                node.attributes
                    .insert(attr_name.clone(), default_value.clone());
            }
        }

        // create NodeKey and NodeDBC
        let key: NodeKey = self.nodes.insert(node);

        // push NodeKey in relevant variables
        self.nodes_order.push(key);
        self.node_key_by_name.insert(name.to_lowercase(), key);
        Ok(key)
    }

    /// Link a sender node to a message, keeping both sides in sync.
    pub fn add_sender_relation(
        &mut self,
        msg_key: MessageKey,
        node_key: NodeKey,
    ) -> Result<(), DatabaseError> {
        let mut pending_tx: Vec<SignalKey>;
        {
            let message =
                self.get_message_by_key(msg_key)
                    .ok_or(DatabaseError::MessageMissing {
                        message_key: msg_key,
                    })?;
            let node = self
                .get_node_by_key(node_key)
                .ok_or(DatabaseError::NodeMissing { node_key })?;

            // signals of the Message that needs to be added as NodeDBC.tx_signals
            pending_tx = message
                .signals
                .iter()
                .copied()
                .filter(|sig| !node.tx_signals.contains(sig))
                .collect();
        }

        // check that a MessageDBC exist for given MessageKey
        let Some(message) = self.get_message_by_key_mut(msg_key) else {
            return Err(DatabaseError::MessageMissing {
                message_key: msg_key,
            });
        };

        // add the NodeKey to MessageDBC if not already present
        if !message.sender_nodes.contains(&node_key) {
            message.sender_nodes.push(node_key);
        }

        // check that a NodeDBC exist for given NodeKey
        let Some(node) = self.get_node_by_key_mut(node_key) else {
            return Err(DatabaseError::NodeMissing { node_key });
        };

        // add the MessageKey to NodeDBC if not already present
        if !node.messages_sent.contains(&msg_key) {
            node.messages_sent.push(msg_key);
        }

        // add the SignalKeys missing from NodeDBC
        for signal_key in pending_tx.drain(..) {
            node.tx_signals.push(signal_key);
        }

        Ok(())
    }

    /// Remove a message from a Sender Node, keeping both sides in sync.
    pub fn remove_sender_relation(
        &mut self,
        msg_key: MessageKey,
        node_key: NodeKey,
    ) -> Result<(), DatabaseError> {
        let mut to_prune: Vec<SignalKey>;
        {
            let message =
                self.get_message_by_key(msg_key)
                    .ok_or(DatabaseError::MessageMissing {
                        message_key: msg_key,
                    })?;
            // signals of the Message that needs to be removed as NodeDBC.tx_signals
            to_prune = message.signals.to_vec();
        }

        {
            // check that a MessageDBC exist for given MessageKey
            let Some(message) = self.get_message_by_key_mut(msg_key) else {
                return Err(DatabaseError::MessageMissing {
                    message_key: msg_key,
                });
            };
            // remove the NodeKey from MessageDBC.sender_nodes
            message.sender_nodes.retain(|x| x != &node_key);
        }

        // check that a NodeDBC exist for given NodeKey
        let Some(node) = self.get_node_by_key_mut(node_key) else {
            return Err(DatabaseError::NodeMissing { node_key });
        };

        node.messages_sent.retain(|x| x != &msg_key);

        // remove the SignalKeys from NodeDBC.tx_signals
        if !to_prune.is_empty() {
            let prune_set: HashSet<SignalKey> = to_prune.drain(..).collect();
            node.tx_signals.retain(|sig| !prune_set.contains(sig));
        }

        Ok(())
    }

    /// Create a new Node from an existing one adding "_copy" to the name
    /// Messages and Signals are modified to include new node relations
    pub fn copy_node(&mut self, source_node_key: NodeKey) -> Result<NodeKey, DatabaseError> {
        let new_node: NodeDBC = {
            // check that the source node key correspond to a Node
            let Some(node) = self.get_node_by_key(source_node_key) else {
                return Err(DatabaseError::NodeMissing {
                    node_key: source_node_key,
                });
            };

            // check that new copy name does not already exist
            let mut copy_counter: u32 = 0;
            let mut new_name: String = format!("{}_copy", &node.name);
            while self.get_node_by_name(&new_name).is_some() {
                new_name = format!("{}_copy{}", &node.name, copy_counter);
                copy_counter += 1;
            }
            let mut cloned: NodeDBC = node.clone();
            cloned.name = new_name;
            cloned
        };

        // Collect current relations to refresh after the insertion
        let messages_sent: Vec<MessageKey> = new_node.messages_sent.clone();
        let rx_signals: Vec<SignalKey> = new_node.rx_signals.clone();

        // Validate that related messages still exist
        for &msg_key in &messages_sent {
            if self.get_message_by_key(msg_key).is_none() {
                return Err(DatabaseError::MessageMissing {
                    message_key: msg_key,
                });
            }
        }

        // Gather signal/message pairs; ensure the message is still present
        let mut signal_message_pairs: Vec<(SignalKey, MessageKey)> =
            Vec::with_capacity(rx_signals.len());
        for &sig_key in &rx_signals {
            let Some(signal) = self.get_sig_by_key(sig_key) else {
                return Err(DatabaseError::SignalMissing {
                    signal_key: sig_key,
                });
            };
            if self.get_message_by_key(signal.message).is_none() {
                return Err(DatabaseError::MessageMissing {
                    message_key: signal.message,
                });
            }
            signal_message_pairs.push((sig_key, signal.message));
        }

        let new_name_lower = new_node.name.to_lowercase();
        let new_key: NodeKey = self.nodes.insert(new_node);
        self.nodes_order.push(new_key);
        self.node_key_by_name.insert(new_name_lower, new_key);

        // re-link messages_sent and tx_signals
        for msg_key in messages_sent {
            self.add_sender_relation(msg_key, new_key)?;
        }

        // re-link receivers for each signal (and aggregate at message level)
        for (sig_key, msg_key) in signal_message_pairs {
            {
                let Some(signal) = self.get_sig_by_key_mut(sig_key) else {
                    return Err(DatabaseError::SignalMissing {
                        signal_key: sig_key,
                    });
                };
                if !signal.receiver_nodes.contains(&new_key) {
                    signal.receiver_nodes.push(new_key);
                }
            }

            if let Some(message) = self.get_message_by_key_mut(msg_key) {
                if !message.receiver_nodes.contains(&new_key) {
                    message.receiver_nodes.push(new_key);
                }
            } else {
                return Err(DatabaseError::MessageMissing {
                    message_key: msg_key,
                });
            }
        }

        Ok(new_key)
    }

    /// Deletes the node identified by `node_key`, removing every reference across the database.
    pub fn delete_node(&mut self, node_key: NodeKey) -> Result<(), DatabaseError> {
        let removed_node: NodeDBC = self
            .nodes
            .remove(node_key)
            .ok_or(DatabaseError::NodeMissing { node_key })?;

        let node_name_lower: String = removed_node.name.to_lowercase();

        self.nodes_order.retain(|&k| k != node_key);
        self.node_key_by_name.remove(&node_name_lower);

        self.bu_sg_rel_attributes
            .retain(|(nk, _), _| *nk != node_key);
        self.bu_bo_rel_attributes
            .retain(|(nk, _), _| *nk != node_key);

        for (_msg_key, message) in self.messages.iter_mut() {
            message.sender_nodes.retain(|&nk| nk != node_key);
            message.receiver_nodes.retain(|&nk| nk != node_key);
        }

        for (_sig_key, signal) in self.signals.iter_mut() {
            signal.receiver_nodes.retain(|&nk| nk != node_key);
        }

        Ok(())
    }

    /// Looks up the `NodeKey` for a given node name (case-insensitive).
    pub fn get_node_key_by_name(&self, name: &str) -> Option<NodeKey> {
        self.node_key_by_name.get(&name.to_lowercase()).copied()
    }

    /// Returns an immutable reference to the node addressed by the supplied key.
    pub fn get_node_by_key(&self, key: NodeKey) -> Option<&NodeDBC> {
        self.nodes.get(key)
    }

    /// Returns a mutable reference to the node addressed by the supplied key.
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
    /// Adds a message, indexes its id/name and updates `current_msg` for upcoming SG_ rows.
    pub fn add_message(
        &mut self,
        name: &str,
        id: u32,
        byte_length: u16,
    ) -> Result<MessageKey, DatabaseError> {
        // check if message with provided name already exist
        if let Some(r) = self.get_msg_key_by_name(name) {
            self.current_msg = Some(r); // set found message as current_msg
            return Err(DatabaseError::MessageAlreadyExists {
                name: name.to_string(),
            });
        }

        // check if message with provided ID already exist
        if let Some(r) = self.get_msg_key_by_id(&id) {
            self.current_msg = Some(r); // set found message as current_msg
            let id_hex: String = id_to_hex(id);
            return Err(DatabaseError::MessageIdAlreadyAssigned { id_hex });
        }

        let id_hex: String = id_to_hex(id).to_string();

        let id_format: IdFormat = if id > 2048 {
            IdFormat::Extended
        } else {
            IdFormat::Standard
        };

        let msg_key: MessageKey = self.messages.insert(MessageDBC {
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
            ..Default::default()
        });

        self.messages_order.push(msg_key);

        self.msg_key_by_id.insert(id, msg_key);
        self.msg_key_by_hex.insert(id_hex, msg_key);
        self.msg_key_by_name.insert(name.to_lowercase(), msg_key);

        self.current_msg = Some(msg_key); // set created message as current_msg
        Ok(msg_key)
    }

    /// Deletes the Message identified by `msg_key`, removing every reference across the database.
    pub fn delete_message(&mut self, msg_key: MessageKey) -> Result<(), DatabaseError> {
        let removed_msg: MessageDBC =
            self.messages
                .remove(msg_key)
                .ok_or(DatabaseError::MessageMissing {
                    message_key: msg_key,
                })?;

        let msg_name_lower: String = removed_msg.name.to_lowercase();

        self.messages_order.retain(|&k| k != msg_key);
        self.msg_key_by_name.remove(&msg_name_lower);

        self.bu_bo_rel_attributes
            .retain(|(_, mk), _| *mk != msg_key);

        // remove the Message from the Nodes.messages_sent
        // remove associate Signals from Node.tx_signals
        for (_node_key, node) in self.nodes.iter_mut() {
            node.messages_sent.retain(|&mk| mk != msg_key);
            for sig_key in &removed_msg.signals {
                node.tx_signals.retain(|&sk| sk != *sig_key);
            }
        }

        // remove the Message from the signal.message
        for (_sig_key, signal) in self.signals.iter_mut() {
            if signal.message == msg_key {
                signal.message = MessageKey::default();
            }
        }

        Ok(())
    }

    /// Create a new Message from an existing one adding "_copy" to the name and +1 to ID.
    /// Inside Signals will be copied too.
    pub fn copy_message(
        &mut self,
        source_msg_key: MessageKey,
    ) -> Result<MessageKey, DatabaseError> {
        // check that the source message key correspond to a Message
        let (src_name, src_id, src_byte_len, src_comment, src_attrs, src_sender_nodes, src_signals) = {
            let source_msg =
                self.get_message_by_key(source_msg_key)
                    .ok_or(DatabaseError::MessageMissing {
                        message_key: source_msg_key,
                    })?;
            (
                source_msg.name.clone(),
                source_msg.id,
                source_msg.byte_length,
                source_msg.comment.clone(),
                source_msg.attributes.clone(),
                source_msg.sender_nodes.clone(),
                source_msg.signals.clone(),
            )
        };

        // check that new copy name does not already exist
        let mut copy_counter: u32 = 0;
        let mut new_name: String = format!("{}_copy", &src_name);
        while self.get_message_by_name(&new_name).is_some() {
            new_name = format!("{}_copy{}", &src_name, copy_counter);
            copy_counter += 1;
        }

        // increment the id by 1 until it is not already existing
        let mut new_id: u32 = src_id + 1;
        while self.get_message_by_id(new_id).is_some() {
            new_id += 1;
        }

        let new_msg_key: MessageKey = self.add_message(&new_name, new_id, src_byte_len)?;
        let Some(new_msg) = self.get_message_by_key_mut(new_msg_key) else {
            return Err(DatabaseError::InconsistentState {
                details: "newly created message missing",
            });
        };

        // update comments and attributes
        new_msg.comment = src_comment;
        new_msg.attributes = src_attrs;

        // useful info from old_signals
        let useful_sig_info: Vec<(SignalKey, MuxRole, Option<MuxSelector>)> = src_signals
            .iter()
            .filter_map(|&old_sk| {
                let s = self.get_sig_by_key(old_sk)?;
                let (role, sel) = if let Some(mi) = &s.mux {
                    (mi.role, Some(mi.selector.clone()))
                } else {
                    (MuxRole::None, None)
                };
                Some((old_sk, role, sel))
            })
            .collect();

        // copy internal signals and attach them to new message
        for (old_sk, role, sel) in useful_sig_info {
            if let Ok(new_sk) = self.copy_signal(old_sk) {
                let _ = self.add_msg_sig_relation(new_sk, new_msg_key, role, sel.clone());
            }
        }

        // update Nodes.message_sent
        for node_key in src_sender_nodes {
            let _ = self.add_sender_relation(new_msg_key, node_key);
        }

        Ok(new_msg_key)
    }

    /// Looks up the `MessageKey` from a case-insensitive message name.
    pub fn get_msg_key_by_name(&self, name: &str) -> Option<MessageKey> {
        self.msg_key_by_name.get(&name.to_lowercase()).copied()
    }

    /// Looks up the `MessageKey` by numeric CAN identifier.
    pub fn get_msg_key_by_id(&self, id: &u32) -> Option<MessageKey> {
        self.msg_key_by_id.get(id).copied()
    }

    /// Looks up the `MessageKey` by hexadecimal CAN identifier.
    pub fn get_msg_key_by_id_hex(&self, id_hex: &str) -> Option<MessageKey> {
        // let key: String = normalize_id_hex(id_hex); // "0x...UPPERCASE"
        self.msg_key_by_hex.get(id_hex).copied()
    }

    /// Returns an immutable reference to a message given its key.
    pub fn get_message_by_key(&self, key: MessageKey) -> Option<&MessageDBC> {
        self.messages.get(key)
    }

    /// Returns a mutable reference to a message given its key.
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
    /// Adds a signal to the database and returns the corresponding `SignalKey`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_signal(
        &mut self,
        name: &str,
        endian: Endianness,
        sign: Signess,
        factor: f64,
        offset: f64,
        min: f64,
        max: f64,
        unit: &str,
    ) -> SignalKey {
        let mut sig: SignalDBC = SignalDBC {
            name: name.to_string(),
            endian,
            sign,
            factor,
            offset,
            min,
            max,
            unit_of_measurement: unit.to_string(),
            ..Default::default()
        };
        sig.compile_inline();

        let sig_key: SignalKey = self.signals.insert(sig);
        self.signals_order.push(sig_key);
        self.sig_key_by_name.insert(name.to_lowercase(), sig_key);

        sig_key
    }

    /// Deletes the Signal identified by `sig_key`, removing every reference across the database.
    pub fn delete_signal(&mut self, sig_key: SignalKey) -> Result<(), DatabaseError> {
        let removed_sig: SignalDBC =
            self.signals
                .remove(sig_key)
                .ok_or(DatabaseError::SignalMissing {
                    signal_key: sig_key,
                })?;

        let sig_name_lower: String = removed_sig.name.to_lowercase();

        self.signals_order.retain(|&k| k != sig_key);
        self.sig_key_by_name.remove(&sig_name_lower);

        self.bu_sg_rel_attributes
            .retain(|(_, sk), _| *sk != sig_key);

        // remove the Signal from the Node rx_signal and tx_signal
        for (_node_key, node) in self.nodes.iter_mut() {
            node.tx_signals.retain(|&sk| sk != sig_key);
            node.rx_signals.retain(|&sk| sk != sig_key);
        }

        // remove the Signal from the Message.signal
        for (_msg_key, message) in self.messages.iter_mut() {
            message.signals.retain(|&sk| sk != sig_key);
        }

        Ok(())
    }

    /// Associates an additional receiver node with an existing signal, keeping both sides in sync.
    pub fn add_sig_receiver_node(
        &mut self,
        sig_key: SignalKey,
        node_key: NodeKey,
    ) -> Result<(), DatabaseError> {
        let Some(signal) = self.get_sig_by_key_mut(sig_key) else {
            return Err(DatabaseError::SignalMissing {
                signal_key: sig_key,
            });
        };

        let msg_key: MessageKey = signal.message;

        // add the NodeKey to SignalDBC if not already present
        if !signal.receiver_nodes.contains(&node_key) {
            signal.receiver_nodes.push(node_key);
        }

        let Some(node) = self.get_node_by_key_mut(node_key) else {
            return Err(DatabaseError::NodeMissing { node_key });
        };

        // add the SignalKey to NodeDBC if not already present
        if !node.rx_signals.contains(&sig_key) {
            node.rx_signals.push(sig_key);
        }

        // check that the MessageDBC containing SignalKey contains NodeKey as receiver
        let Some(message) = self.get_message_by_key_mut(msg_key) else {
            return Err(DatabaseError::MessageMissing {
                message_key: msg_key,
            });
        };

        // add the NodeKey to MessageDBC if not already present
        if !message.receiver_nodes.contains(&node_key) {
            message.receiver_nodes.push(node_key);
        }

        Ok(())
    }

    /// Remove a receiver node from an existing signal, keeping both sides in sync.
    pub fn remove_sig_receiver_node(
        &mut self,
        sig_key: SignalKey,
        node_key: NodeKey,
    ) -> Result<(), DatabaseError> {
        let Some(signal) = self.get_sig_by_key_mut(sig_key) else {
            return Err(DatabaseError::SignalMissing {
                signal_key: sig_key,
            });
        };

        let msg_key: MessageKey = signal.message;

        // remove the NodeKey from SignalDBC.receiver_nodes
        signal.receiver_nodes.retain(|x| x != &node_key);

        let Some(node) = self.get_node_by_key_mut(node_key) else {
            return Err(DatabaseError::NodeMissing { node_key });
        };

        // remove the SignalKey from NodeDBC.rx_signals
        node.rx_signals.retain(|x| x != &sig_key);

        // check if the NodeKey still has some signal from the MessageDBC
        let still_receives_any_from_msg: bool = {
            let Some(node) = self.get_node_by_key(node_key) else {
                return Err(DatabaseError::NodeMissing { node_key });
            };

            node.rx_signals.iter().copied().any(|sk| {
                self.get_sig_by_key(sk)
                    .map(|s| s.message == msg_key)
                    .unwrap_or(false)
            })
        };

        // if there are no more signals from that MessageDBC, remove the Nodekey as receiver of it
        if !still_receives_any_from_msg {
            let Some(message) = self.get_message_by_key_mut(msg_key) else {
                return Err(DatabaseError::MessageMissing {
                    message_key: msg_key,
                });
            };
            message.receiver_nodes.retain(|x| x != &node_key);
        }

        Ok(())
    }

    /// Binds a signal to a message, configuring its layout and multiplexing metadata.
    pub fn add_msg_sig_relation(
        &mut self,
        sig_key: SignalKey,
        msg_key: MessageKey,
        mux_role: MuxRole,
        mux_selector: Option<MuxSelector>,
    ) -> Result<SignalKey, DatabaseError> {
        // check if the SignalDBC is already associated to a MessageDBC
        let Some(signal) = self.get_sig_by_key(sig_key) else {
            return Err(DatabaseError::SignalMissing {
                signal_key: sig_key,
            });
        };
        let bit_start: u16 = signal.bit_start;
        let bit_length: u16 = signal.bit_length;
        if !signal.message.is_null() {
            let mkey: MessageKey = signal.message;
            let associated_with = if let Some(message) = self.get_message_by_key(mkey) {
                format!("message '{}' (ID {})", message.name, message.id_hex)
            } else {
                "an unknown message".to_string()
            };
            return Err(DatabaseError::SignalAlreadyAssociated {
                signal: signal.name.clone(),
                associated_with,
            });
        }

        // check if the signal bit_start and bit_length are not too big for Message.bytes_length
        let Some(message) = self.get_message_by_key(msg_key) else {
            return Err(DatabaseError::MessageMissing {
                message_key: msg_key,
            });
        };
        let dlc: u16 = message.byte_length;
        let endianness: Endianness = signal.endian.clone();
        message_layout::check_signal_fits(dlc, bit_start, bit_length, endianness)?;

        // if SignalDBC is Multiplexed, try to guess the Multiplexor if there is only one in the message
        let inferred_switch: Option<SignalKey> = if mux_role == MuxRole::Multiplexed {
            self.get_message_by_key(msg_key).and_then(|msg| {
                if msg.mux_multiplexors.len() == 1 {
                    Some(msg.mux_multiplexors[0])
                } else {
                    None
                }
            })
        } else {
            None
        };

        // We'll need receiver_nodes later to aggregate into MessageDBC.receiver_nodes
        let msg_receivers: Vec<NodeKey> = {
            let Some(signal) = self.get_sig_by_key_mut(sig_key) else {
                return Err(DatabaseError::SignalMissing {
                    signal_key: sig_key,
                });
            };

            // update relevant signal fields
            signal.message = msg_key;
            signal.bit_start = bit_start;
            signal.bit_length = bit_length;
            signal.mux = if mux_role == MuxRole::None {
                None
            } else {
                Some(MuxInfo {
                    role: mux_role,
                    group: 0,
                    switch: inferred_switch,
                    selector: mux_selector.clone().unwrap_or_default(),
                })
            };

            signal.steps.clear();
            signal.compile_inline();

            signal.receiver_nodes.clone()
        };

        {
            let Some(message) = self.get_message_by_key_mut(msg_key) else {
                return Err(DatabaseError::MessageMissing {
                    message_key: msg_key,
                });
            };

            // add the signal within current message
            if !message.signals.contains(&sig_key) {
                message.signals.push(sig_key);
            }

            // Aggregate receivers at message level (union of all signal receivers)
            for nk in &msg_receivers {
                if !message.receiver_nodes.contains(nk) {
                    message.receiver_nodes.push(*nk);
                }
            }
        }

        // Also back-link: for each sender node of this message, mark this signal as sent
        // This keeps NodeDBC.tx_signals consistent when the transmitter is specified on BO_
        // and SG_ lines are parsed afterwards (common case without BO_TX_BU_ lines).
        let sender_nodes: Vec<NodeKey> = self
            .get_message_by_key(msg_key)
            .map(|m| m.sender_nodes.clone())
            .unwrap_or_default();
        for nk in sender_nodes {
            if let Some(node) = self.get_node_by_key_mut(nk)
                && !node.tx_signals.contains(&sig_key)
            {
                node.tx_signals.push(sig_key);
            }
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
                let dep_to_attach: Vec<(SignalKey, MuxSelector)> = {
                    let Some(msg) = self.get_message_by_key(msg_key) else {
                        return Err(DatabaseError::MessageMissingDuringMultiplexing);
                    };
                    msg.signals
                        .iter()
                        .copied()
                        .filter_map(|sk| {
                            let s: &SignalDBC = self.get_sig_by_key(sk)?;
                            let mi: &MuxInfo = s.mux.as_ref()?;
                            if mi.role == MuxRole::Multiplexed && mi.switch.is_none() {
                                Some((sk, mi.selector.clone()))
                            } else {
                                None
                            }
                        })
                        .collect()
                };

                // Update the signals and the mux_cases
                for (sk, sel) in dep_to_attach {
                    // set the Multiplexor to the signal
                    if let Some(s) = self.get_sig_by_key_mut(sk)
                        && let Some(mi) = s.mux.as_mut()
                        && mi.role == MuxRole::Multiplexed
                        && mi.switch.is_none()
                    {
                        mi.switch = Some(sig_key);
                    }

                    // Update the map of the message
                    if let Some(m) = self.get_message_by_key_mut(msg_key) {
                        let by_sel = m.mux_cases.entry(sig_key).or_default();
                        by_sel.entry(sel.clone()).or_default().push(sk);
                    }
                }
            }
            MuxRole::Multiplexed => {
                if let Some(sw) = inferred_switch
                    && let Some(m) = self.get_message_by_key_mut(msg_key)
                {
                    let by_sel = m.mux_cases.entry(sw).or_default();
                    if let Some(sel) = mux_selector.clone() {
                        by_sel.entry(sel).or_default().push(sig_key);
                    }
                }
            }
        }

        Ok(sig_key)
    }

    /// Detaches a signal from a message, reversing [`Self::add_msg_sig_relation`].
    pub fn remove_msg_sig_relation(
        &mut self,
        sig_key: SignalKey,
        msg_key: MessageKey,
    ) -> Result<(), DatabaseError> {
        // Ensure the message exists.
        self.get_message_by_key(msg_key)
            .ok_or(DatabaseError::MessageMissing {
                message_key: msg_key,
            })?;

        // Snapshot the signal state and verify that it is bound to the target message.
        let mux_snapshot: Option<MuxInfo> = {
            let signal = self
                .get_sig_by_key(sig_key)
                .ok_or(DatabaseError::SignalMissing {
                    signal_key: sig_key,
                })?;
            if signal.message.is_null() {
                return Err(DatabaseError::InconsistentState {
                    details: "Signal is not associated with any message",
                });
            }
            if signal.message != msg_key {
                let associated_with = if let Some(message) = self.get_message_by_key(signal.message)
                {
                    format!("Message '{}' (ID {})", message.name, message.id_hex)
                } else {
                    "An unknown message".to_string()
                };
                return Err(DatabaseError::SignalAlreadyAssociated {
                    signal: signal.name.clone(),
                    associated_with,
                });
            }
            signal.mux.clone()
        };

        let mut multiplexed_to_detach: Vec<SignalKey> = Vec::new();

        {
            let Some(message) = self.get_message_by_key_mut(msg_key) else {
                return Err(DatabaseError::MessageMissing {
                    message_key: msg_key,
                });
            };

            let before = message.signals.len();
            message.signals.retain(|&sk| sk != sig_key);
            if before == message.signals.len() {
                return Err(DatabaseError::InconsistentState {
                    details: "Signal not registered within the message.",
                });
            }

            if let Some(mux_info) = &mux_snapshot {
                match mux_info.role {
                    MuxRole::Multiplexor => {
                        message.mux_multiplexors.retain(|&mk| mk != sig_key);
                        if let Some(by_sel) = message.mux_cases.remove(&sig_key) {
                            for sigs in by_sel.values() {
                                multiplexed_to_detach.extend(sigs.iter().copied());
                            }
                        }
                    }
                    MuxRole::Multiplexed => {
                        if let Some(sw) = mux_info.switch
                            && let Some(by_sel) = message.mux_cases.get_mut(&sw) {
                                by_sel.retain(|_, list| {
                                    list.retain(|&sk| sk != sig_key);
                                    !list.is_empty()
                                });
                                if by_sel.is_empty() {
                                    message.mux_cases.remove(&sw);
                                }
                            }
                    }
                    MuxRole::None => {}
                }
            }
        }

        // Clear multiplexing switch references on dependents that previously pointed at this signal.
        for dep in multiplexed_to_detach {
            if let Some(sig) = self.get_sig_by_key_mut(dep)
                && let Some(mi) = sig.mux.as_mut()
                && mi.role == MuxRole::Multiplexed
            {
                mi.switch = None;
            }
        }

        // Reset the detached signal metadata.
        if let Some(signal) = self.get_sig_by_key_mut(sig_key) {
            signal.message = MessageKey::default();
            signal.mux = None;
        }

        // Remove the signal from every sender node's transmitted list.
        let sender_nodes: Vec<NodeKey> = self
            .get_message_by_key(msg_key)
            .map(|m| m.sender_nodes.clone())
            .unwrap_or_default();
        for nk in sender_nodes {
            if let Some(node) = self.get_node_by_key_mut(nk) {
                node.tx_signals.retain(|&sk| sk != sig_key);
            }
        }

        // Rebuild the receiver list for the message (union of the remaining signal receivers).
        let new_receivers: Vec<NodeKey> = if let Some(message) = self.get_message_by_key(msg_key) {
            let mut seen: HashSet<NodeKey> = HashSet::new();
            let mut ordered: Vec<NodeKey> = Vec::new();
            for &sk in &message.signals {
                if let Some(sig) = self.get_sig_by_key(sk) {
                    for &nk in &sig.receiver_nodes {
                        if seen.insert(nk) {
                            ordered.push(nk);
                        }
                    }
                }
            }
            ordered
        } else {
            Vec::new()
        };

        if let Some(message) = self.get_message_by_key_mut(msg_key) {
            message.receiver_nodes = new_receivers;
        }

        Ok(())
    }

    /// Create a new Signal from an existing one adding "_copy" to the name.
    pub fn copy_signal(&mut self, source_sig_key: SignalKey) -> Result<SignalKey, DatabaseError> {
        // check that the source node key correspond to a Node
        let (
            src_name,
            src_endian,
            src_sign,
            src_factor,
            src_offset,
            src_min,
            src_max,
            src_unit,
            src_value_table,
            src_receivers,
            bit_start,
            bit_length,
            src_comment,
            src_attrs,
        ) = {
            let s = self
                .get_sig_by_key(source_sig_key)
                .ok_or(DatabaseError::SignalMissing {
                    signal_key: source_sig_key,
                })?;
            (
                s.name.clone(),
                s.endian.clone(),
                s.sign.clone(),
                s.factor,
                s.offset,
                s.min,
                s.max,
                s.unit_of_measurement.clone(),
                s.value_table.clone(),
                s.receiver_nodes.clone(),
                s.bit_start,
                s.bit_length,
                s.comment.clone(),
                s.attributes.clone(),
            )
        }; // <-- fine borrow immutabile

        // check that new copy name does not already exist
        let mut copy_counter: u32 = 0;
        let mut new_name: String = format!("{}_copy", &src_name);
        while self.get_signal_by_name(&new_name).is_some() {
            new_name = format!("{}_copy{}", &src_name, copy_counter);
            copy_counter += 1;
        }

        let new_sig_key: SignalKey = self.add_signal(
            &new_name, src_endian, src_sign, src_factor, src_offset, src_min, src_max, &src_unit,
        );
        {
            let Some(new_sig) = self.get_sig_by_key_mut(new_sig_key) else {
                return Err(DatabaseError::InconsistentState {
                    details: "newly created signal missing",
                });
            };

            // update comments and attributes
            new_sig.comment = src_comment;
            new_sig.attributes = src_attrs;
            new_sig.value_table = src_value_table;
            new_sig.bit_length = bit_length;
            new_sig.bit_start = bit_start;

            for node_key in src_receivers {
                let _ = self.add_sig_receiver_node(new_sig_key, node_key);
            }
        }

        Ok(new_sig_key)
    }

    /// Looks up the `SignalKey` for a case-insensitive signal name.
    pub fn get_sig_key_by_name(&self, name: &str) -> Option<SignalKey> {
        self.sig_key_by_name.get(&name.to_lowercase()).copied()
    }

    /// Returns an immutable reference to a signal given its key.
    pub fn get_sig_by_key(&self, key: SignalKey) -> Option<&SignalDBC> {
        self.signals.get(key)
    }

    /// Returns a mutable reference to a signal given its key.
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

    // -------------- Immutable Iterators ---------------
    /// Iterator according to the orders (defualt order is name based)
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

    // -------------- Mutable Closures ---------------
    /// Closure to edit all NodeDBC
    pub fn for_each_node_mut(&mut self, mut f: impl FnMut(&mut NodeDBC)) {
        let keys = self.nodes_order.clone(); // evitiamo borrow lungo su nodes_order
        for k in keys {
            if let Some(node) = self.nodes.get_mut(k) {
                f(node);
            }
        }
    }

    /// Closure to edit all MessageDBC
    pub fn for_each_message_mut(&mut self, mut f: impl FnMut(&mut MessageDBC)) {
        let keys = self.messages_order.clone();
        for k in keys {
            if let Some(msg) = self.messages.get_mut(k) {
                f(msg);
            }
        }
    }

    /// Closure to edit all SignalDBC
    pub fn for_each_signal_mut(&mut self, mut f: impl FnMut(&mut SignalDBC)) {
        let keys = self.signals_order.clone();
        for k in keys {
            if let Some(sig) = self.signals.get_mut(k) {
                f(sig);
            }
        }
    }

    // -------------- Sorting ---------------
    /// Sort nodes_by_name case insensitive
    pub fn sort_db_nodes_by_name(&mut self) {
        self.nodes_order
            .sort_by_cached_key(|&k| self.nodes.get(k).map(|n| n.name.to_ascii_lowercase()));
    }

    /// Sort messages_by_name case insensitive
    pub fn sort_db_messages_by_name(&mut self) {
        self.messages_order
            .sort_by_cached_key(|&k| self.messages.get(k).map(|m| m.name.to_ascii_lowercase()));
    }

    /// Sort signals_by_name case insensitive
    pub fn sort_db_signals_by_name(&mut self) {
        self.signals_order
            .sort_by_cached_key(|&k| self.signals.get(k).map(|s| s.name.to_ascii_lowercase()));
    }

    /// Sort `messages_sent`, `tx_signals` and `rx_signals` inside the specific given NodeDBC
    /// by the target names (ASCII case-insensitive).
    pub fn sort_node_fields(&mut self, node_key: NodeKey) {
        // Compute the new order on immutable borrows
        let (sorted_msgs, sorted_sigs_sent, sorted_sigs_received) = {
            let Some(node) = self.get_node_by_key(node_key) else {
                return;
            };

            // messages_sent -> by MessageDBC.name
            let mut ms: Vec<MessageKey> = node.messages_sent.clone();
            ms.sort_by_cached_key(|&mk| {
                self.get_message_by_key(mk)
                    .map(|m| m.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            // tx_signals -> by SignalDBC.name
            let mut sr1: Vec<SignalKey> = node.tx_signals.clone();
            sr1.sort_by_cached_key(|&sk| {
                self.get_sig_by_key(sk)
                    .map(|s| s.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            // rx_signals -> by SignalDBC.name
            let mut sr2: Vec<SignalKey> = node.rx_signals.clone();
            sr2.sort_by_cached_key(|&sk| {
                self.get_sig_by_key(sk)
                    .map(|s| s.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            (ms, sr1, sr2)
        };

        // Write back with a mutable borrow
        if let Some(node) = self.get_node_by_key_mut(node_key) {
            node.messages_sent = sorted_msgs;
            node.tx_signals = sorted_sigs_sent;
            node.rx_signals = sorted_sigs_received;
        }
    }

    /// Sort `sender_nodes` and `signals` inside the specific given MessageDBC
    /// by the target names (ASCII case-insensitive).
    pub fn sort_message_fields(&mut self, msg_key: MessageKey) {
        let (sorted_senders, sorted_sigs, sorted_receivers) = {
            let Some(msg) = self.get_message_by_key(msg_key) else {
                return;
            };

            // sender_nodes -> by NodeDBC.name
            let mut ns: Vec<NodeKey> = msg.sender_nodes.clone();
            ns.sort_by_cached_key(|&nk| {
                self.get_node_by_key(nk)
                    .map(|n| n.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            // receiver_nodes -> by NodeDBC.name
            let mut rn: Vec<NodeKey> = msg.receiver_nodes.clone();
            rn.sort_by_cached_key(|&nk| {
                self.get_node_by_key(nk)
                    .map(|n| n.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            // signals -> by SignalDBC.name
            let mut ss: Vec<SignalKey> = msg.signals.clone();
            ss.sort_by_cached_key(|&sk| {
                self.get_sig_by_key(sk)
                    .map(|s| s.name.to_ascii_lowercase())
                    .unwrap_or_default()
            });

            (ns, ss, rn)
        };

        if let Some(msg) = self.get_message_by_key_mut(msg_key) {
            msg.sender_nodes = sorted_senders;
            msg.signals = sorted_sigs;
            msg.receiver_nodes = sorted_receivers;
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
    /// - `tx_signals`  by the target MessageDBC.name (ASCII case-insensitive)
    /// - `rx_signals`   by the target SignalDBC.name  (ASCII case-insensitive)
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

                // tx_signals -> sort by signal name (case-insensitive)
                let mut sr1: Vec<SignalKey> = node.tx_signals.clone();
                sr1.sort_by_cached_key(|&sk| {
                    let (missing, name) = match self.get_sig_by_key(sk) {
                        Some(s) => (false, s.name.to_ascii_lowercase()),
                        None => (true, String::new()),
                    };
                    (missing, name, sk)
                });

                // rx_signals -> sort by signal name (case-insensitive)
                let mut sr2: Vec<SignalKey> = node.rx_signals.clone();
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
                    tx_signals: sr1,
                    rx_signals: sr2,
                }
            })
            .collect();

        // Apply the plans with mutable borrows.
        for p in plans {
            if let Some(node) = self.get_node_by_key_mut(p.nk) {
                node.messages_sent = p.messages_sent;
                node.tx_signals = p.tx_signals;
                node.rx_signals = p.rx_signals;
            }
        }
    }

    /// For ALL MessageDBC entries, sort:
    /// - `sender_nodes` by NodeDBC.name    (ASCII case-insensitive)
    /// - `signals`      by SignalDBC.name  (ASCII case-insensitive)
    ///
    /// Missing/invalid keys are pushed to the end; ties are broken by the key.
    pub fn sort_all_message_fields(&mut self) {
        let plans: Vec<MessageFieldPlan> = self
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

                // receiver_nodes -> sort by node name (case-insensitive)
                let mut rn = msg.receiver_nodes.clone();
                rn.sort_by_cached_key(|&nk| {
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

                (mk, ns, ss, rn)
            })
            .collect();

        for (mk, ns, ss, rn) in plans {
            if let Some(msg) = self.get_message_by_key_mut(mk) {
                msg.sender_nodes = ns;
                msg.signals = ss;
                msg.receiver_nodes = rn;
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

    /// Resets the entire database to an empty state (drops nodes, messages, signals, and metadata).
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
    /// Returns a display-friendly label (allocates a new `String`).
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
    tx_signals: Vec<SignalKey>,
    rx_signals: Vec<SignalKey>,
}

/// Type alias to simplify clippy::type_complexity for message sorting plans.
type MessageFieldPlan = (MessageKey, Vec<NodeKey>, Vec<SignalKey>, Vec<NodeKey>);

const CAN_EFF_MASK: u32 = 0x1FFF_FFFF; // 29 bit
const CAN_SFF_MASK: u32 = 0x0000_07FF; // 11 bit

#[inline]
pub fn id_to_hex(id: u32) -> String {
    if id <= CAN_SFF_MASK {
        format!("0x{:03X}", id)
    } else {
        format!("0x{:08X}", id & CAN_EFF_MASK)
    }
}
