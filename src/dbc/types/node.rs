use crate::dbc::types::{
    attributes::AttributeValue,
    database::{MessageKey, SignalKey},
};
use std::collections::BTreeMap;

/// Node/ECU defined in the database.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct NodeDBC {
    /// Node/ECU name.
    pub name: String,
    /// Associated comment
    pub comment: String,
    /// Messages transmitted by this node.
    pub messages_sent: Vec<MessageKey>,
    /// Signals read by this node
    pub signals_sent: Vec<SignalKey>,
    /// Signals read by this node
    pub signals_read: Vec<SignalKey>,

    // --- Attributes ---
    pub attributes: BTreeMap<String, AttributeValue>,
}

impl NodeDBC {
    /// Resets all fields to their default values.
    /// Clear the database
    pub fn clear(&mut self) {
        *self = NodeDBC::default();
    }
}
