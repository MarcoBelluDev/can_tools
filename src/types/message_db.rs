use crate::{Database, NodeKey, SignalDB, SignalKey};
use std::collections::HashMap;

/// CAN message defined in the database (DBC/ARXML).
///
/// Maintains the numeric ID (`id`), the normalized hexadecimal ID (`id_hex`),
/// the `name`, payload length (`byte_length`), and metadata such as `msgtype`, `cycle_time`,
/// the transmitting nodes (`sender_nodes`), and the list of composing signals (`signals`).
#[derive(Default, Clone, PartialEq, Debug)]
pub struct MessageDB {
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
    /// List of multiplexer switch signals (primary first). Empty if none.
    pub mux_multiplexors: Vec<SignalKey>,

    /// Fast lookup: for each Multiplexer -> for each selector -> signals gated by that selector.
    ///
    /// Example: mux_cases[Motor_MUX][Value(0)] = [Motor_status, Motor_Direction, ...]
    pub mux_cases: HashMap<SignalKey, HashMap<MuxSelector, Vec<SignalKey>>>,
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
        self.signals
            .iter()
            .filter_map(move |&key| db.get_sig_by_key(key))
    }
}

#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub enum IdFormat {
    #[default]
    Standard,
    Extended,
}

impl IdFormat {
    pub fn to_str(&self) -> String {
        match self {
            IdFormat::Standard => "Standard".to_string(),
            IdFormat::Extended => "Extended".to_string(),
        }
    }
}

/// What role (if any) a signal plays in multiplexing.
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

/// A selector for multiplexed signals: either a single value or a closed range.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MuxSelector {
    /// Active only when the switch == value.
    Value(u32),
    /// Active only when min <= switch <= max.
    Range { min: u32, max: u32 },
}

/// Multiplexing metadata attached to a signal.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MuxInfo {
    /// Role of this signal in multiplexing.
    pub role: MuxRole,
    /// Optional group index (extended multiplexing). `0` if unused.
    pub group: u8,
    /// For `Dependent` signals, the switch controlling it. `None` otherwise.
    pub switch: Option<SignalKey>,
    /// For `Dependent` signals, the allowed selectors (values/ranges). Empty otherwise.
    pub selectors: Vec<MuxSelector>,
}

impl MuxInfo {
    pub fn role_to_string(&self) -> String {
        match self.role {
            MuxRole::None => "None".to_string(),
            MuxRole::Multiplexed => "Multiplexed".to_string(),
            MuxRole::Multiplexor => "Multiplexor".to_string(),
        }
    }
}
