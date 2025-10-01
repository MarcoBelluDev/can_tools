/// Metadata and payload carried by a
/// [`CanFrame`](crate::asc::types::canframe::CanFrame).
///
/// There is typically one `MessageLog` per `CanFrame`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MessageLog {
    /// Logger channel index (typically 1-based).
    pub channel: u8,

    /// Raw payload length token.
    pub byte_length: u16,

    /// `"CAN"` or `"CAN FD"` depending on payload size.
    pub protocol: String,

    /// Raw identifier token as seen in the log (may include format suffixes).
    pub id: String,

    /// Empty if unknown (missing DB).
    pub name: String,

    /// Empty if unknown (missing DB).
    pub sender_node: String,

    /// Payload bytes as hex pairs separated by spaces.
    pub data: String,

    /// Optional comment (empty if unknown).
    pub comment: String,

    /// Indices into `CanLog.signals` for all `SignalLog` present in this message.
    pub signals: Vec<usize>,
}

impl MessageLog {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        self.channel = 0;
        self.byte_length = 0;
        self.protocol.clear();
        self.id.clear();
        self.name.clear();
        self.sender_node.clear();
        self.data.clear();
        self.comment.clear();
        self.signals.clear();
    }
}
