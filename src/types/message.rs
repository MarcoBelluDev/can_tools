use crate::types::{
    attributes::AttributeValue,
    database::{CanDatabase, CanNodeKey, CanSignalKey},
    signal::CanSignal,
};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

/// CAN message defined in the database (DBC/ARXML).
///
/// Maintains the numeric ID (`id`), the normalized hexadecimal ID (`id_hex`),
/// the `name`, payload length (`byte_length`), and metadata such as `msgtype`, `cycle_time`,
/// the transmitting nodes (`sender_nodes`), and the list of composing signals (`signals`).
#[derive(Default, Clone, PartialEq)]
pub struct CanMessage {
    /// ID Format (Standard or Extended)
    pub id_format: IdFormat,
    /// Numeric CAN ID (base 10).
    pub id: u32,
    /// **Normalized** hexadecimal CAN ID (`"0x..."`, uppercase).
    pub id_hex: String,
    /// Message name.
    pub name: String,
    /// Payload length in bytes.
    pub byte_length: u16,
    /// Message type string (free-form from the DBC, defaults to `"CAN"` / `"CAN FD"` based on payload length).
    pub msgtype: String,
    /// Transmitting nodes (ECUs) for this message.
    pub sender_nodes: Vec<CanNodeKey>,
    /// Receiver nodes (ECUs) aggregated from all signals in this message.
    pub receiver_nodes: Vec<CanNodeKey>,
    /// Signals that belong to this message.
    pub signals: Vec<CanSignalKey>,
    /// Associated comment (DBC `CM_ BO_` section).
    pub comment: String,
    /// List of multiplexor switch signals (primary first). Empty if none.
    pub mux_multiplexors: Vec<CanSignalKey>,

    // --- Message Attribute Entry ---
    pub attributes: BTreeMap<String, AttributeValue>,

    /// Fast lookup: for each Multiplexor -> for each selector -> signals gated by that selector.
    ///
    /// Example: mux_cases\[Motor_MUX\]\[Value(0)\] = [Motor_status, Motor_Direction, ...]
    pub mux_cases: HashMap<CanSignalKey, HashMap<MuxSelector, Vec<CanSignalKey>>>,
}

impl CanMessage {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        *self = CanMessage::default();
    }

    /// Convenience iterator over the `CanSignal`s belonging to this message.
    pub fn signals<'a>(&'a self, db: &'a CanDatabase) -> impl Iterator<Item = &'a CanSignal> + 'a {
        self.signals
            .iter()
            .filter_map(move |&key| db.get_sig_by_key(key))
    }
}

/// CAN identifier format (standard 11-bit or extended 29-bit).
#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub enum IdFormat {
    #[default]
    Standard,
    Extended,
}

impl IdFormat {
    /// Returns a human-readable name for this CAN ID format.
    pub fn to_str(&self) -> String {
        match self {
            IdFormat::Standard => "Standard".to_string(),
            IdFormat::Extended => "Extended".to_string(),
        }
    }
}

/// Role a signal plays in multiplexing.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum MuxRole {
    /// Not multiplexed (always present).
    #[default]
    None,
    /// This signal is the multiplexer switch (marked as `M` in DBC).
    Multiplexor,
    /// This signal is gated by a multiplexer value (marked as `mX`).
    Multiplexed,
}

impl fmt::Display for MuxRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MuxRole::None => f.write_str("None"),
            MuxRole::Multiplexor => f.write_str("Multiplexor"),
            MuxRole::Multiplexed => f.write_str("Multiplexed"),
        }
    }
}

/// Selector for multiplexed signals: either a single value or a closed range.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MuxSelector {
    /// Active only when the switch == value.
    Value(u32),
    /// Active only when min <= switch <= max.
    Range { min: u32, max: u32 },
}

impl Default for MuxSelector {
    fn default() -> Self {
        // Default is a no-op value; only meaningful when role == Multiplexed.
        MuxSelector::Value(0)
    }
}

impl fmt::Display for MuxSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MuxSelector::Value(v) => write!(f, "Value({v})"),
            MuxSelector::Range { min, max } => write!(f, "Range({min}..={max})"),
        }
    }
}

/// Message send behavior (as used by some DBC attributes like `GenMsgSendType`).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum GenMsgSendType {
    Cyclic,   // 0
    NotUsed,  // da 0 a 6
    IfActive, // 7
    #[default]
    NoMsgSendType, // 8
}
