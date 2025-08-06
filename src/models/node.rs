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
/// use can_tools::models::node::Node;
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
