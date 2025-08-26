use crate::{SignalKey, NodeKey, Database, SignalDB};

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

// --- helpers ---

/// Normalizes a hexadecimal ID string.
///
/// Converts variants such as `"12DD54E3x"`, `"0x12dd54e3"`, `"12dd54e3"`
/// into the canonical form `"0x12DD54E3"`.
pub(crate) fn normalize_id_hex(s: &str) -> String {
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

#[derive(Default, Clone, PartialEq, Debug)]
pub enum IdFormat {
    #[default]
    Standard,
    Extended,
}

impl IdFormat {
    pub fn to_str(&self) -> String {
        match self {
            IdFormat::Standard => {
                "Standard".to_string()
            },
            IdFormat::Extended=> {
                "Extended".to_string()
            },
        }
    }
}