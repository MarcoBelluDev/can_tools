use crate::types::message::Message;
use crate::types::node::Node;

/// Represents an in-memory representation of a parsed DBC file.
///
/// The `Database` struct holds the main elements and metadata extracted from a DBC file, such as:
/// - The database name (`BA_ "DBName"`).
/// - The bus type used (e.g., CAN or CAN FD) (`BA_ "BusType"`).
/// - The baudrate of the CAN network (`BA_ "Baudrate"`).
/// - The CANFD baudrate of the CAN network (`BA_ "BaudrateCANFD"`).
/// - The DBC version (`VERSION` line).
/// - The list of CAN nodes (`BU_` lines).
/// - The list of CAN messages (`BO_` lines), including their signals.
///
/// This struct also provides parsing methods for interpreting DBC lines and converting them
/// into structured Rust data.
///
/// # Fields
/// - `name`: The database name (parsed from the `BA_ "DBName"` attribute).
/// - `bustype`: The type of bus (e.g., "CAN") (parsed from the `BA_ "BusType"` attribute).
/// - `baudrate`: The network baudrate in bits per second (parsed from the `BA_ "Baudrate"` attribute).
/// - `version`: The DBC file version string (parsed from the `VERSION` line).
/// - `nodes`: The list of CAN nodes in the network (parsed from `BU_` lines).
/// - `messages`: The list of CAN messages in the network (parsed from `BO_` lines).
///
/// # Example
/// ```
/// use can_tools::types::database::Database;
///
/// let db = Database::default();
/// assert_eq!(db.name, "");
/// assert!(db.messages.is_empty());
/// ```
#[derive(Default, Clone, PartialEq, Debug)]
pub struct Database {
    pub name: String,           
    pub bustype: String,        
    pub baudrate: usize,       
    pub baudrate_canfd: usize,  
    pub version: String,        
    pub nodes: Vec<Node>,       
    pub messages: Vec<Message>, 
}

impl Database {
    // ---- Public accessors ----

    /// Returns an immutable reference to a `Message` by its numeric CAN ID.
    ///
    /// # Parameters
    /// - `id`: Numeric CAN ID of the message to search for.
    ///
    /// # Returns
    /// - `Some(&Message)` if a message with the given ID exists.
    /// - `None` otherwise.
    pub fn get_message_by_id(&self, id: u64) -> Option<&Message> {
        self.messages.iter().find(|msg| msg.id == id)
    }

    /// Returns a mutable reference to a `Message` by its numeric CAN ID.
    ///
    /// # Parameters
    /// - `id`: Numeric CAN ID of the message to search for.
    ///
    /// # Returns
    /// - `Some(&mut Message)` if a message with the given ID exists.
    /// - `None` otherwise.
    pub fn get_message_by_id_mut(&mut self, id: u64) -> Option<&mut Message> {
        self.messages.iter_mut().find(|msg| msg.id == id)
    }

    /// Returns an immutable reference to a `Message` by its hexadecimal CAN ID.
    ///
    /// The comparison is **case-insensitive**.
    ///
    /// # Parameters
    /// - `id_hex`: Hexadecimal CAN ID string (e.g., `"0x123"`).
    ///
    /// # Returns
    /// - `Some(&Message)` if a message with the given hex ID exists.
    /// - `None` otherwise.
    pub fn get_message_by_id_hex(&self, id_hex: &str) -> Option<&Message> {
        self.messages
            .iter()
            .find(|msg| msg.id_hex.eq_ignore_ascii_case(id_hex))
    }

    /// Returns a mutable reference to a `Message` by its hexadecimal CAN ID.
    ///
    /// The comparison is **case-insensitive**.
    ///
    /// # Parameters
    /// - `id_hex`: Hexadecimal CAN ID string (e.g., `"0x123"`).
    ///
    /// # Returns
    /// - `Some(&mut Message)` if a message with the given hex ID exists.
    /// - `None` otherwise.
    pub fn get_message_by_id_hex_mut(&mut self, id_hex: &str) -> Option<&mut Message> {
        self.messages
            .iter_mut()
            .find(|msg| msg.id_hex.eq_ignore_ascii_case(id_hex))
    }

    /// Returns an immutable reference to a `Message` by its name.
    ///
    /// The comparison is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the message to search for.
    ///
    /// # Returns
    /// - `Some(&Message)` if a message with the given name exists.
    /// - `None` otherwise.
    pub fn get_message_by_name(&self, name: &str) -> Option<&Message> {
        self.messages
            .iter()
            .find(|msg| msg.name.eq_ignore_ascii_case(name))
    }

    /// Returns a mutable reference to a `Message` by its name.
    ///
    /// The comparison is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the message to search for.
    ///
    /// # Returns
    /// - `Some(&mut Message)` if a message with the given name exists.
    /// - `None` otherwise.
    pub fn get_message_by_name_mut(&mut self, name: &str) -> Option<&mut Message> {
        self.messages
            .iter_mut()
            .find(|msg| msg.name.eq_ignore_ascii_case(name))
    }

    /// Returns an immutable reference to a `Node` by its name.
    ///
    /// The comparison is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the node to search for.
    ///
    /// # Returns
    /// - `Some(&Node)` if a node with the given name exists.
    /// - `None` otherwise.
    pub fn get_nodes_by_name(&self, name: &str) -> Option<&Node> {
        self.nodes
            .iter()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    /// Returns a mutable reference to a `Node` by its name.
    ///
    /// The comparison is **case-insensitive**.
    ///
    /// # Parameters
    /// - `name`: The name of the node to search for.
    ///
    /// # Returns
    /// - `Some(&mut Node)` if a node with the given name exists.
    /// - `None` otherwise.
    pub fn get_nodes_by_name_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.nodes
            .iter_mut()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    /// Clears all metadata, nodes, and messages from this `Database`.
    ///
    /// This method resets string fields to empty strings and numeric fields to `0`,
    /// and empties the `nodes` and `messages` vectors.
    ///
    /// # Effects
    /// - `name`, `bustype`, `version` → `""`
    /// - `baudrate`, `baudrate_canfd` → `0`
    /// - `nodes`, `messages` → emptied (via `Vec::default`)
    pub fn clear(&mut self) {
        self.name.clear();
        self.bustype.clear();
        self.baudrate = 0;
        self.baudrate_canfd = 0;
        self.version.clear();
        self.nodes = Vec::default();
        self.messages = Vec::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_db() -> Database {
        Database {
            name: "TestCAN".to_string(),
            bustype: "CAN".to_string(),
            baudrate: 500000,
            baudrate_canfd: 2000000,
            version: "1.0".into(),
            nodes: vec![
                Node {
                    name: "Motor".to_string(),
                    comment: "Test comment".to_string(),
                },
                Node {
                    name: "Gateway".to_string(),
                    comment: "Test comment 2".to_string(),
                },
            ],
            messages: vec![
                Message {
                    id: 100,
                    id_hex: "0x64".into(),
                    name: "Motor_01".into(),
                    byte_length: 16,
                    msgtype: "CAN FD".to_string(),
                    sender_nodes: vec![Node {
                        name: "Motor".into(),
                        comment: "".to_string(),
                    }],
                    signals: vec![],
                    comment: "Test comment".into(),
                },
                Message {
                    id: 200,
                    id_hex: "0xC8".into(),
                    name: "Game_01".into(),
                    byte_length: 4,
                    msgtype: "CAN".to_string(),
                    sender_nodes: vec![Node {
                        name: "Infotainment".into(),
                        comment: "".to_string(),
                    }],
                    signals: vec![],
                    comment: "Another comment".into(),
                },
            ],
        }
    }

    #[test]
    fn test_get_message_by_id() {
        let db = build_test_db();
        let msg = db.get_message_by_id(100);
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().name, "Motor_01");

        // ID inesistente
        assert!(db.get_message_by_id(999).is_none());
    }

    #[test]
    fn test_get_message_by_id_mut() {
        let mut db = build_test_db();
        let msg = db.get_message_by_id_mut(100);
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().name, "Motor_01");

        // ID inesistente
        assert!(db.get_message_by_id_mut(999).is_none());
    }

    #[test]
    fn test_get_message_by_id_hex() {
        let db = build_test_db();
        let msg = db.get_message_by_id_hex("0xC8");
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id, 200);

        // Case insensitive
        let msg_lower = db.get_message_by_id_hex("0xc8");
        assert!(msg_lower.is_some());
        assert_eq!(msg_lower.unwrap().id, 200);

        // ID HEX inesistente
        assert!(db.get_message_by_id_hex("0xFFFF").is_none());
    }

    #[test]
    fn test_get_message_by_id_hex_mut() {
        let mut db = build_test_db();
        let msg = db.get_message_by_id_hex_mut("0xC8");
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id, 200);

        // Case insensitive
        let msg_lower = db.get_message_by_id_hex_mut("0xc8");
        assert!(msg_lower.is_some());
        assert_eq!(msg_lower.unwrap().id, 200);

        // ID HEX inesistente
        assert!(db.get_message_by_id_hex_mut("0xFFFF").is_none());
    }

    #[test]
    fn test_get_message_by_name() {
        let db = build_test_db();
        let msg = db.get_message_by_name("Motor_01");
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id, 100);

        // Case insensitive
        let msg_lower = db.get_message_by_name("motor_01");
        assert!(msg_lower.is_some());
        assert_eq!(msg_lower.unwrap().id, 100);

        // Nome inesistente
        assert!(db.get_message_by_name("UnknownName").is_none());
    }

    #[test]
    fn test_get_message_by_name_mut() {
        let mut db = build_test_db();
        let msg = db.get_message_by_name_mut("Motor_01");
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id, 100);

        // Case insensitive
        let msg_lower = db.get_message_by_name_mut("motor_01");
        assert!(msg_lower.is_some());
        assert_eq!(msg_lower.unwrap().id, 100);

        // Nome inesistente
        assert!(db.get_message_by_name_mut("UnknownName").is_none());
    }

    #[test]
    fn test_get_nodes_by_name() {
        let db: Database = build_test_db();

        // Exact search
        let node: Option<&Node> = db.get_nodes_by_name("Motor");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Motor");
        assert_eq!(node.unwrap().comment, "Test comment");

        // Insensitive search
        let node: Option<&Node> = db.get_nodes_by_name("gateway");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Gateway");

        // Signal not existing
        assert!(db.get_nodes_by_name("FakeECU").is_none());
    }

    #[test]
    fn test_get_nodes_by_name_mut() {
        let mut db: Database = build_test_db();

        // Exact search
        let node: Option<&mut Node> = db.get_nodes_by_name_mut("Gateway");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Gateway");

        // Insensitive search
        let node: Option<&mut Node> = db.get_nodes_by_name_mut("motor");
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Motor");

        // Signal not existing
        assert!(db.get_nodes_by_name_mut("FakeECU").is_none());
    }

    #[test]
    fn test_clear() {
        let mut db: Database = build_test_db();

        // Check that everything is back to default value
        db.clear();
        assert_eq!(db, Database::default());
    }
}
