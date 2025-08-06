use crate::models::node::Node;
use crate::models::signal::Signal;

/// Represents a generic CAN message parsed from a DBC file.
///
/// A `Message` contains metadata such as its CAN ID (both decimal and hexadecimal),
/// the message name, its byte length, the sender nodes, the list of signals
/// it contains, and an optional comment.
///
/// # Fields
/// - `id`: The numeric CAN ID of the message.
/// - `id_hex`: The CAN ID in hexadecimal string format (e.g., `"0x123"`).
/// - `name`: The message name as defined in the DBC file.
/// - `byte_length`: The length of the CAN frame payload in bytes.
/// - `sender_nodes`: The list of nodes that can send this message.
/// - `signals`: The list of signals (`Signal`) contained in this message.
/// - `comment`: An optional comment or description for the message.
///
/// # Example
/// ```
/// use can_tools::models::message::Message;
/// let msg = Message::default();
/// assert!(msg.signals.is_empty());
/// ```
///
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Message {
    pub id: u64,
    pub id_hex: String,
    pub name: String,
    pub byte_length: usize,
    pub sender_nodes: Vec<Node>,
    pub signals: Vec<Signal>, // SG_
    pub comment: String,      // CM_ BO_
}

impl Message {
    /// Returns an immutable reference to a `Signal` by its name.
    ///
    /// The search is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the signal to search for.
    ///
    /// # Returns
    /// - `Some(&Signal)` if a matching signal is found.
    /// - `None` if no matching signal exists.
    ///
    /// # Example
    /// ```
    /// # use can_tools::models::message::Message;
    /// let msg = Message::default();
    /// assert!(msg.get_signal_by_name("Speed").is_none());
    /// ```
    pub fn get_signal_by_name(&self, name: &str) -> Option<&Signal> {
        self.signals
            .iter()
            .find(|sig| sig.name.eq_ignore_ascii_case(name))
    }

    /// Returns a mutable reference to a `Signal` by its name.
    ///
    /// The search is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the signal to search for.
    ///
    /// # Returns
    /// - `Some(&mut Signal)` if a matching signal is found.
    /// - `None` if no matching signal exists.
    ///
    /// # Example
    /// ```
    /// # use can_tools::models::message::Message;
    /// let mut msg = Message::default();
    /// if let Some(sig) = msg.get_signal_by_name_mut("RPM") {
    ///     sig.factor = 2.0;
    /// }
    /// ```
    pub fn get_signal_by_name_mut(&mut self, name: &str) -> Option<&mut Signal> {
        self.signals
            .iter_mut()
            .find(|sig| sig.name.eq_ignore_ascii_case(name))
    }

    /// Returns an immutable reference to a sender `Node` by its name.
    ///
    /// The search is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the sender node to search for.
    ///
    /// # Returns
    /// - `Some(&Node)` if a matching node is found.
    /// - `None` if no matching node exists.
    ///
    /// # Example
    /// ```
    /// # use can_tools::models::message::Message;
    /// let msg = Message::default();
    /// assert!(msg.get_sender_nodes_by_name("Gateway").is_none());
    /// ```
    pub fn get_sender_nodes_by_name(&self, name: &str) -> Option<&Node> {
        self.sender_nodes
            .iter()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    /// Returns a mutable reference to a sender `Node` by its name.
    ///
    /// The search is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the sender node to search for.
    ///
    /// # Returns
    /// - `Some(&mut Node)` if a matching node is found.
    /// - `None` if no matching node exists.
    ///
    /// # Example
    /// ```
    /// # use can_tools::models::message::Message;
    /// let mut msg = Message::default();
    /// if let Some(node) = msg.get_sender_nodes_by_name_mut("Motor") {
    ///     node.comment = "Updated comment".to_string();
    /// }
    /// ```
    pub fn get_sender_nodes_by_name_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.sender_nodes
            .iter_mut()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn build_test_message() -> Message {
        Message {
            id: 100,
            id_hex: "0x64".into(),
            name: "Motor_01".into(),
            byte_length: 8,
            sender_nodes: vec![
                Node {
                    name: "Motor".to_string(),
                    comment: "Test comment".to_string(),
                },
                Node {
                    name: "Gateway".to_string(),
                    comment: "Test comment 2".to_string(),
                },
            ],
            signals: vec![
                Signal {
                    name: "Speed".into(),
                    bit_start: 0,
                    bit_length: 16,
                    endian: 1,
                    sign: 0,
                    factor: 1.0,
                    offset: 0.0,
                    min: 0.0,
                    max: 250.0,
                    unit_of_measurement: "km/h".into(),
                    receiver_nodes: vec![],
                    comment: "Vehicle speed".into(),
                    value_table: HashMap::new(),
                },
                Signal {
                    name: "RPM".into(),
                    bit_start: 16,
                    bit_length: 16,
                    endian: 1,
                    sign: 0,
                    factor: 0.25,
                    offset: 0.0,
                    min: 0.0,
                    max: 8000.0,
                    unit_of_measurement: "rpm".into(),
                    receiver_nodes: vec![],
                    comment: "Engine RPM".into(),
                    value_table: HashMap::new(),
                },
            ],
            comment: "Test comment".into(),
        }
    }

    #[test]
    fn test_get_signal_by_name() {
        let msg = build_test_message();

        // Exact search
        let sig = msg.get_signal_by_name("Speed");
        assert!(sig.is_some());
        assert_eq!(sig.unwrap().unit_of_measurement, "km/h");

        // Insensitive search
        let sig_lower = msg.get_signal_by_name("rpm");
        assert!(sig_lower.is_some());
        assert_eq!(sig_lower.unwrap().factor, 0.25);

        // Signal not existing
        assert!(msg.get_signal_by_name("FuelLevel").is_none());
    }

    #[test]
    fn test_get_signal_by_name_mut() {
        let mut msg: Message = build_test_message();

        // Exact search
        let sig = msg.get_signal_by_name_mut("Speed");
        assert!(sig.is_some());
        assert_eq!(sig.unwrap().unit_of_measurement, "km/h");

        // Insensitive search
        let sig_lower = msg.get_signal_by_name_mut("rpm");
        assert!(sig_lower.is_some());
        assert_eq!(sig_lower.unwrap().factor, 0.25);

        // Signal not existing
        assert!(msg.get_signal_by_name("FuelLevel").is_none());
    }

    #[test]
    fn test_get_sender_nodes_by_name() {
        let msg: Message = build_test_message();

        // Exact search
        let node: Option<&Node> = msg.get_sender_nodes_by_name("Motor");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Motor");
        assert_eq!(node.unwrap().comment, "Test comment");

        // Insensitive search
        let node: Option<&Node> = msg.get_sender_nodes_by_name("gateway");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Gateway");

        // Signal not existing
        assert!(msg.get_sender_nodes_by_name("FakeECU").is_none());
    }

    #[test]
    fn test_get_sender_nodes_by_name_mut() {
        let mut msg: Message = build_test_message();

        // Exact search
        let node: Option<&mut Node> = msg.get_sender_nodes_by_name_mut("Gateway");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Gateway");

        // Insensitive search
        let node: Option<&mut Node> = msg.get_sender_nodes_by_name_mut("motor");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Motor");

        // Signal not existing
        assert!(msg.get_sender_nodes_by_name_mut("FakeECU").is_none());
    }
}
