use std::collections::HashMap;

use crate::models::node::Node;

// SG_ <name> : <bit_start>|<bit_lengths>@<endianness><signedness> (<scale>,<factor>) [<min>|<max>] "<units>" <receiver nodes...>
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Signal {
    pub name: String,
    pub bit_start: usize,
    pub bit_length: usize,
    pub endian: usize, // 1 = little endian (Intel), 0 = big endian (Motorola)
    pub sign: usize,
    pub factor: f64,
    pub offset: f64,
    pub min: f64,
    pub max: f64,
    pub unit_of_measurement: String,
    pub receiver_nodes: Vec<Node>,
    pub comment: String,                   // CM_ SG_
    pub value_table: HashMap<i32, String>, // VAL_ <message_id> <signal_name> <val1> "<descr1>" <val2> "<descr2>" ... ;
}
impl Signal {
    pub fn get_receiver_nodes_by_name(&self, name: &str) -> Option<&Node> {
        self.receiver_nodes
            .iter()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    pub fn get_receiver_nodes_by_name_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.receiver_nodes
            .iter_mut()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn build_test_signal() -> Signal {
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
            receiver_nodes: vec![
                Node {
                    name: "Gateway".to_string(),
                    comment: "Comment on gateway node".to_string(),
                },
                Node {
                    name: "Motor".to_string(),
                    comment: "Comment on motor ecu".to_string(),
                },
            ],
            comment: "Vehicle speed".into(),
            value_table: HashMap::new(),
        }
    }

    #[test]
    fn test_get_receiver_nodes_by_name() {
        let sig: Signal = build_test_signal();

        // Exact search
        let node: Option<&Node> = sig.get_receiver_nodes_by_name("Gateway");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Gateway");
        assert_eq!(node.unwrap().comment, "Comment on gateway node");

        // Insensitive search
        let node: Option<&Node> = sig.get_receiver_nodes_by_name("motor");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Motor");

        // Signal not existing
        assert!(sig.get_receiver_nodes_by_name("FakeECU").is_none());
    }

    #[test]
    fn test_get_receiver_nodes_by_name_mut() {
        let mut sig: Signal = build_test_signal();

        // Exact search
        let node: Option<&mut Node> = sig.get_receiver_nodes_by_name_mut("Gateway");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Gateway");

        // Insensitive search
        let node: Option<&mut Node> = sig.get_receiver_nodes_by_name_mut("motor");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Motor");

        // Signal not existing
        assert!(sig.get_receiver_nodes_by_name_mut("FakeECU").is_none());
    }
}
