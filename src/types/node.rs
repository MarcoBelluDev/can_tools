use crate::types::{
    attributes::AttributeValue,
    database::{CanMessageKey, CanSignalKey},
};
use std::collections::BTreeMap;

/// Node/ECU defined in the database.
#[derive(Default, Clone, PartialEq)]
pub struct CanNode {
    /// Node/ECU name.
    pub name: String,
    /// Associated comment.
    pub comment: String,
    /// Messages transmitted by this node.
    pub messages_sent: Vec<CanMessageKey>,
    /// Signals this node transmits (aggregated from the messages it sends).
    pub tx_signals: Vec<CanSignalKey>,
    /// Signals this node receives.
    pub rx_signals: Vec<CanSignalKey>,

    // --- Attributes ---
    pub attributes: BTreeMap<String, AttributeValue>,
}

impl CanNode {
    /// Resets all fields to their default values.
    ///
    /// Useful for reusing an instance without reallocating backing vectors.
    pub fn clear(&mut self) {
        *self = CanNode::default();
    }
}
