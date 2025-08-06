use crate::models::node::Node;
use crate::models::signal::Signal;

// BO_ <ID> <MESSAGE_NAME> : <BYTES_LENGHT> <SENDER_NODE>
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
    pub fn get_signal_by_name(&self, name: &str) -> Option<&Signal> {
        self.signals
            .iter()
            .find(|sig| sig.name.eq_ignore_ascii_case(name))
    }

    pub fn get_signal_by_name_mut(&mut self, name: &str) -> Option<&mut Signal> {
        self.signals
            .iter_mut()
            .find(|sig| sig.name.eq_ignore_ascii_case(name))
    }

    pub fn get_sender_nodes_by_name(&self, name: &str) -> Option<&Node> {
        self.sender_nodes
            .iter()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

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
