/// Represents a node (ECU) in a CAN network as defined in a DBC file.
///
/// A `Node` identifies a physical or logical unit in the CAN system
/// that can transmit or receive messages.  
/// It typically corresponds to an ECU (Electronic Control Unit) or a
/// software module within the vehicle's network.
///
/// # Fields
/// - `name`: The unique name of the node (ECU identifier).
/// - `comment`: Optional descriptive text providing additional information about the node.
///
/// # Example
/// ```
/// use can_tools::types::node::Node;
///
/// let node = Node {
///     name: "Motor".to_string(),
///     comment: "Controls engine-related functions".to_string(),
/// };
///
/// assert_eq!(node.name, "Motor");
/// ```
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Node {
    pub name: String,
    pub comment: String,
}

impl Node {
    /// Clears all metadata from this `Node`.
    ///
    /// This method resets string fields to empty strings 
    ///
    /// # Effects
    /// - `name`, `comment` → `""`
    pub fn clear(&mut self) {
        self.name.clear();
        self.comment.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_node() -> Node {
        Node {
            name: "Motor ECU".to_string(),
            comment: "Comment about the Motor ECU.".to_string(),
        }
    }

    #[test]
    fn test_clear() {
        let mut node: Node = build_test_node();

        // Check that everything is back to default value
        node.clear();
        assert_eq!(node, Node::default());
    }
}
