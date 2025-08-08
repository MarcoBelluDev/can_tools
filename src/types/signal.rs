use crate::types::node::Node;
use std::collections::HashMap;

/// Represents a signal within a CAN message as defined in a DBC file.
///
/// A `Signal` describes how a specific piece of data is encoded within a CAN frame.
/// It includes information such as bit position, bit length, endianness, scaling,
/// valid range, and associated receiver nodes.
///
/// # Fields
/// - `name`: The name of the signal.
/// - `bit_start`: The starting bit position of the signal within the CAN frame.
/// - `bit_length`: The number of bits used to encode the signal.
/// - `endian`: Endianness of the signal:
///   - `1` = little-endian (Intel format)
///   - `0` = big-endian (Motorola format)
/// - `sign`: Whether the signal is signed (`1`) or unsigned (`0`).
/// - `factor`: Scaling factor applied to the raw value.
/// - `offset`: Offset added to the scaled value.
/// - `min`: Minimum valid physical value for the signal.
/// - `max`: Maximum valid physical value for the signal.
/// - `unit_of_measurement`: The unit in which the signal is expressed (e.g., `"km/h"`).
/// - `receiver_nodes`: The list of nodes that receive this signal.
/// - `comment`: Optional descriptive text about the signal.
/// - `value_table`: Mapping between raw integer values and their string representations.
///
/// # Example
/// ```
/// use can_tools::types::signal::Signal;
/// let sig = Signal::default();
/// assert!(sig.receiver_nodes.is_empty());
/// ```
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Signal {
    pub name: String,
    pub bit_start: usize,
    pub bit_length: usize,
    /// Endianness:
    /// - `1` = little-endian (Intel)
    /// - `0` = big-endian (Motorola)
    pub endian: usize,
    pub sign: usize,
    pub factor: f64,
    pub offset: f64,
    pub min: f64,
    pub max: f64,
    pub unit_of_measurement: String,
    pub receiver_nodes: Vec<Node>,
    pub comment: String,
    pub value_table: HashMap<i32, String>,
}
impl Signal {
    /// Returns an immutable reference to a receiver `Node` by its name.
    ///
    /// The search is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the receiver node to search for.
    ///
    /// # Returns
    /// - `Some(&Node)` if a matching receiver node is found.
    /// - `None` if no matching node exists.
    ///
    /// # Example
    /// ```
    /// # use can_tools::types::signal::Signal;
    /// # use can_tools::types::node::Node;
    /// let sig = Signal::default();
    /// assert!(sig.get_receiver_nodes_by_name("Gateway").is_none());
    /// ```
    pub fn get_receiver_nodes_by_name(&self, name: &str) -> Option<&Node> {
        self.receiver_nodes
            .iter()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    /// Returns a mutable reference to a receiver `Node` by its name.
    ///
    /// The search is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the receiver node to search for.
    ///
    /// # Returns
    /// - `Some(&mut Node)` if a matching receiver node is found.
    /// - `None` if no matching node exists.
    ///
    /// # Example
    /// ```
    /// # use can_tools::types::signal::Signal;
    /// # use can_tools::types::node::Node;
    /// let mut sig = Signal::default();
    /// if let Some(node) = sig.get_receiver_nodes_by_name_mut("Motor") {
    ///     node.comment = "Updated comment".to_string();
    /// }
    /// ```
    pub fn get_receiver_nodes_by_name_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.receiver_nodes
            .iter_mut()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    /// Clears all metadata, receiver nodes, and value table from this `Signal`.
    ///
    /// This method resets string fields to empty strings and numeric fields to `0`,
    /// empties the `receiver_nodes` vectors,
    /// and empties the `value_table` HashMap.
    ///
    /// # Effects
    /// - `name`, `unit_of_measurement`, `comment` → `""`
    /// - `bit_start`, `bit_length` and all other numeric parameters → `0`
    /// - `receiver_nodes` → emptied (via `Vec::default`)
    /// - `value_table` → emptied (via `HashMap::default`)
    pub fn clear(&mut self) {
        self.name.clear();
        self.bit_start = 0;
        self.bit_length = 0;
        self.endian = 0;
        self.sign = 0;
        self.factor = 0.0;
        self.offset = 0.0;
        self.min = 0.0;
        self.max = 0.0;
        self.unit_of_measurement.clear();
        self.receiver_nodes = Vec::default();
        self.comment.clear();
        self.value_table = HashMap::default();
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

    #[test]
    fn test_clear() {
        let mut sig: Signal = build_test_signal();

        // Check that everything is back to default value
        sig.clear();
        assert_eq!(sig, Signal::default());
    }
}
