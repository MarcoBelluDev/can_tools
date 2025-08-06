use std::collections::HashMap;

use crate::models::message::Message;
use crate::models::node::Node;
use crate::models::signal::Signal;

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Database {
    pub version: String,        // VERSION
    pub bit_timing: String,     // BS_
    pub nodes: Vec<Node>,       // BU_
    pub messages: Vec<Message>, // BO_
}

impl Database {
    pub fn get_message_by_id(&self, id: u64) -> Option<&Message> {
        self.messages.iter().find(|msg| msg.id == id)
    }

    pub fn get_message_by_id_mut(&mut self, id: u64) -> Option<&mut Message> {
        self.messages.iter_mut().find(|msg| msg.id == id)
    }

    pub fn get_message_by_id_hex(&self, id_hex: &str) -> Option<&Message> {
        self.messages
            .iter()
            .find(|msg| msg.id_hex.eq_ignore_ascii_case(id_hex))
    }

    pub fn get_message_by_id_hex_mut(&mut self, id_hex: &str) -> Option<&mut Message> {
        self.messages
            .iter_mut()
            .find(|msg| msg.id_hex.eq_ignore_ascii_case(id_hex))
    }

    pub fn get_message_by_name(&self, name: &str) -> Option<&Message> {
        self.messages
            .iter()
            .find(|msg| msg.name.eq_ignore_ascii_case(name))
    }

    pub fn get_message_by_name_mut(&mut self, name: &str) -> Option<&mut Message> {
        self.messages
            .iter_mut()
            .find(|msg| msg.name.eq_ignore_ascii_case(name))
    }

    pub fn get_nodes_by_name(&self, name: &str) -> Option<&Node> {
        self.nodes
            .iter()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    pub fn get_nodes_by_name_mut(&mut self, name: &str) -> Option<&mut Node> {
        self.nodes
            .iter_mut()
            .find(|node| node.name.eq_ignore_ascii_case(name))
    }

    pub(crate) fn parse_version(&mut self, line: &str) {
        // Example: VERSION "1.0"
        self.version = line
            .to_lowercase()
            .replace("version", "") // delete version text
            .trim() // delete whitespaces
            .trim_matches('"') // delete "
            .to_string() // convert in string
    }

    pub(crate) fn parse_bit_timing(&mut self, line: &str) {
        // Example: BS_: 125000

        self.bit_timing = line
            .to_lowercase()
            .replace("bs_:", "") // delete "bs_"
            .trim() // delete whitespaces
            .to_string();
    }

    pub(crate) fn parse_nodes(&mut self, line: &str) {
        // Example: BU_: ECU1 ECU2 ECU3 ECU4 etc...

        // Split the lines in part dividere by whitespaces
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Skip "BU_:"
        for part in parts.iter().skip(1) {
            self.nodes.push(Node {
                name: part.to_string(),
                comment: "".to_string(), // initialize empty comment
            });
        }
    }

    pub(crate) fn parse_messages(&mut self, line: &str) {
        // BO_ <ID> <MESSAGE_NAME> : <BYTES_LENGHT> <SENDER_NODE>
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 5 {
            // Too short line are not considered.
            return;
        }

        let id: u64 = parts[1].parse::<u64>().unwrap_or(0); // decimal id
        let id_hex: String = format!("0x{:X}", id); // hexadecimal id
        let name: String = parts[2].trim_end_matches(':').to_string();
        let byte_length: usize = parts[3].parse::<usize>().unwrap_or(0);
        let sender_nodes: Vec<Node> = vec![Node {
            name: parts[4].to_string(),
            comment: "".to_string(),
        }];

        let msg: Message = Message {
            id,
            id_hex,
            name,
            byte_length,
            sender_nodes,
            signals: Vec::new(),
            comment: String::new(),
        };

        self.messages.push(msg);
    }

    pub(crate) fn parse_signal(&mut self, line: &str) {
        // SG_ <name> : <bit_start>|<bit_lengths>@<endianness><signedness> (<scale>,<factor>) [<min>|<max>] "<units>" <receiver nodes...>
        if self.messages.is_empty() {
            return;
        }

        // remove whitespace at end and beginning
        let line: &str = line.trim_start();

        // Split line in two part: before ":" and after
        let mut split_colon = line.splitn(2, ':');
        let left = split_colon.next().unwrap().trim();
        let right = split_colon.next().unwrap_or("").trim();

        // Signal Name
        let name: String = left.split_whitespace().nth(1).unwrap_or("").to_string();

        // Bit start / length / endian / sign
        let mut right_parts = right.split_whitespace();
        let bit_info = right_parts.next().unwrap_or(""); // "63|1@1+"
        let mut bit_and_rest = bit_info.split('@');
        let bit_pos_len = bit_and_rest.next().unwrap_or(""); // "63|1"
        let endian_sign = bit_and_rest.next().unwrap_or(""); // "1+"

        let mut pos_len_parts = bit_pos_len.split('|');
        let bit_start = pos_len_parts
            .next()
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap_or(0);
        let bit_length = pos_len_parts
            .next()
            .unwrap_or("0")
            .parse::<usize>()
            .unwrap_or(0);

        let endian = endian_sign
            .chars()
            .nth(0)
            .unwrap_or('1')
            .to_digit(10)
            .unwrap_or(1) as usize;
        let sign = if endian_sign.contains('-') { 1 } else { 0 };

        // Scale and offset
        let factor_offset_raw: &str = right_parts
            .next()
            .unwrap_or("(1,0)")
            .trim_matches(|c| c == '(' || c == ')');
        let mut so_parts = factor_offset_raw.split(',');
        let factor: f64 = so_parts.next().unwrap_or("1").parse::<f64>().unwrap_or(1.0);
        let offset: f64 = so_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

        // Min and max
        let min_max_raw: &str = right_parts
            .next()
            .unwrap_or("[0|0]")
            .trim_matches(|c| c == '[' || c == ']');
        let mut mm_parts = min_max_raw.split('|');
        let min: f64 = mm_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let max: f64 = mm_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

        // Measurement Unit
        let unit: String = right_parts
            .next()
            .unwrap_or("")
            .trim_matches('"')
            .to_string();

        // Receiver nodes (possono essere separati da virgole)
        let receivers_str = right_parts.collect::<Vec<&str>>().join(" ");
        let receivers: Vec<Node> = receivers_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Node {
                name: s.trim().to_string(),
                comment: "".to_string(),
            })
            .collect();

        // Generate the Signal
        let signal: Signal = Signal {
            name,
            bit_start,
            bit_length,
            endian,
            sign,
            factor,
            offset,
            min,
            max,
            unit_of_measurement: unit,
            receiver_nodes: receivers,
            comment: String::new(),
            value_table: Default::default(),
        };

        // Add to last message
        if let Some(last_msg) = self.messages.last_mut() {
            last_msg.signals.push(signal);
        }
    }

    pub(crate) fn parse_add_nodes(&mut self, line: &str) {
        // remove "BO_TX_BU_"
        let content = line.trim_start_matches("BO_TX_BU_").trim();

        // Split su ":" → prima parte ID, seconda parte lista nodi
        let mut parts = content.splitn(2, ':');
        let id_str = parts.next().unwrap_or("").trim();
        let nodes_str = parts.next().unwrap_or("").trim().trim_end_matches(';');

        // Convert ID in u64
        let id: u64 = match id_str.parse() {
            Ok(v) => v,
            Err(_) => return, // se non è un numero valido, esce
        };

        // Find message by id
        if let Some(msg) = self.get_message_by_id_mut(id) {
            // Split nodes by comma ,
            for node_name in nodes_str
                .split(',')
                .map(|n| n.trim())
                .filter(|n| !n.is_empty())
            {
                // check if already present
                if !msg.sender_nodes.iter().any(|n| n.name == node_name) {
                    msg.sender_nodes.push(Node {
                        name: node_name.to_string(),
                        comment: "".to_string(),
                    });
                }
            }
        }
    }

    pub(crate) fn parse_message_comments(&mut self, line: &str) {
        // Example line: CM_ BO_ 2549880610 "Testo del commento";

        // Split line into parts
        let mut parts = line.split_whitespace();
        parts.next(); // skip CM_
        parts.next(); // skip BO_

        let id_str = match parts.next() {
            Some(id) => id,
            None => return,
        };

        let id: u64 = match id_str.parse() {
            Ok(v) => v,
            Err(_) => return,
        };

        // check if we have a message with that ID
        let msg = match self.get_message_by_id_mut(id) {
            Some(m) => m,
            None => return,
        };

        // Take the comment within " "
        let line = line.trim_end_matches(';').trim();
        let first_quote = match line.find('"') {
            Some(pos) => pos,
            None => return,
        };
        let last_quote = match line.rfind('"') {
            Some(pos) if pos > first_quote => pos,
            _ => return,
        };

        let comment = &line[first_quote + 1..last_quote];

        // push the comment into the message
        msg.comment = comment.to_string();
    }

    pub(crate) fn parse_signal_comments(&mut self, text: &str) {
        // Example: CM_ SG_ 1635 SignalName "Comment..."

        let mut parts = text.split_whitespace();
        parts.next(); // skip CM_
        parts.next(); // skip SG_

        let id_str = match parts.next() {
            Some(id) => id,
            None => return,
        };

        let id: u64 = match id_str.parse() {
            Ok(v) => v,
            Err(_) => return,
        };

        let signal_name = match parts.next() {
            Some(name) => name,
            None => return,
        };

        // Find Message
        let msg: &mut Message = match self.get_message_by_id_mut(id) {
            Some(m) => m,
            None => return,
        };

        // Find Signal
        let sig: &mut Signal = match msg.get_signal_by_name_mut(signal_name) {
            Some(s) => s,
            None => return,
        };

        // Extract the comment
        let first_quote: usize = match text.find('"') {
            Some(pos) => pos,
            None => return,
        };
        let last_quote: usize = match text.rfind('"') {
            Some(pos) if pos > first_quote => pos,
            _ => return,
        };

        let comment = &text[first_quote + 1..last_quote];

        // Push comment into the signal, remove initial whitespaces in lines
        sig.comment = comment
            .lines()
            .map(|l| l.trim_start())
            .collect::<Vec<_>>()
            .join("\n");
    }

    pub(crate) fn parse_node_comments(&mut self, text: &str) {
        // CM_ BU_ NodeName "Comment..."
        
        let mut parts = text.split_whitespace();
        parts.next(); // skip CM_
        parts.next(); // skip BU_

        let node_name = match parts.next() {
            Some(name) => name,
            None => return,
        };

        // Find quotes ""
        let first_quote = match text.find('"') {
            Some(pos) => pos,
            None => return,
        };
        let last_quote = match text.rfind('"') {
            Some(pos) if pos > first_quote => pos,
            _ => return,
        };

        // Take comment and normalize whitespaces
        let comment = text[first_quote + 1..last_quote]
            .lines()
            .map(|l| l.trim_start())
            .collect::<Vec<_>>()
            .join("\n");

        // Update in Database.nodes
        if let Some(node) = self.get_nodes_by_name_mut(node_name) {
            node.comment = comment.clone();
        }

        for msg in &mut self.messages {
            // // Update in sender_nodes for all Messages
            if let Some(sender) = msg.get_sender_nodes_by_name_mut(node_name) {
                sender.comment = comment.clone();
            }

            // Update in receiver_nodes for all Signals
            for sig in &mut msg.signals {
                if let Some(receiver) = sig.get_receiver_nodes_by_name_mut(node_name) {
                    receiver.comment = comment.clone();
                }
            }
        }
    }

    pub(crate) fn parse_value_table(&mut self, line: &str) {
        // remove whitespace at end and beginning
        let line: &str = line.trim_start();

        // Example: VAL_ <message_id> <signal_name> <val1> "<descr1>" <val2> "<descr2>" ... ;
        let mut parts = line.split_whitespace();

        // Skip "VAL_"
        parts.next();

        // Message ID come numero decimale
        let message_id: u64 = parts.next().unwrap_or("").parse::<u64>().unwrap_or(0);

        // Signal Name
        let signal_name: String = parts.next().unwrap_or("").to_string();

        // Rest of row contains couples: <value> "<description>"
        let mut remaining: String = parts.collect::<Vec<_>>().join(" ");

        // Remove final ";"
        remaining = remaining.trim_end_matches(';').trim().to_string();

        // Parsing couples: <value> "<description>"
        let mut value_table: HashMap<i32, String> = HashMap::new();
        let mut tokens = remaining.split('"').map(|s| s.trim());

        while let Some(before) = tokens.next() {
            let before: &str = before.trim();
            if before.is_empty() {
                continue;
            }
            if let Some(num_str) = before.split_whitespace().last() {
                if let Ok(val) = num_str.parse::<i32>() {
                    if let Some(desc) = tokens.next() {
                        value_table.insert(val, desc.to_string());
                    }
                }
            }
        }

        // Add value table to right message and signal
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            if let Some(signal) = msg.signals.iter_mut().find(|s| s.name == signal_name) {
                signal.value_table = value_table;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let mut db: Database = Database::default();
        // Example Line
        db.parse_version(r#"VERSION "1.0.2""#);
        assert_eq!(db.version, "1.0.2");
    }

    #[test]
    fn test_parse_bit_timing() {
        let mut db: Database = Database::default();
        // Example Line
        db.parse_bit_timing("BS_: 125000");
        assert_eq!(db.bit_timing, "125000");
    }

    #[test]
    fn test_parse_nodes() {
        let mut db: Database = Database::default();
        // Example Line
        db.parse_nodes("BU_: Motor Infotainment Gateway");
        assert_eq!(
            db.nodes,
            vec![
                Node {
                    name: "Motor".to_string(),
                    comment: "".to_string(),
                },
                Node {
                    name: "Infotainment".to_string(),
                    comment: "".to_string(),
                },
                Node {
                    name: "Gateway".to_string(),
                    comment: "".to_string(),
                }
            ]
        );
    }

    #[test]
    fn test_parse_messages() {
        let mut db: Database = Database::default();

        // Example Line
        db.parse_messages("BO_ 960 Key_Status: 4 BCM");

        // Only one message must be added
        assert_eq!(db.messages.len(), 1);

        let msg = &db.messages[0];
        assert_eq!(msg.id, 960);
        assert_eq!(msg.id_hex, "0x3C0");
        assert_eq!(msg.name, "Key_Status");
        assert_eq!(msg.byte_length, 4);
        assert_eq!(msg.sender_nodes[0].name, "BCM");
        assert!(msg.signals.is_empty());
        assert!(msg.comment.is_empty());
    }

    #[test]
    fn test_parse_signal() {
        let mut db: Database = Database::default();

        // Add example message to connect the signal to
        db.parse_messages("BO_ 960 Key_Status: 4 BCM");

        // Example Line
        db.parse_signal(r#"SG_ Engine_Speed : 48|8@1+ (1,0) [0|255] "km/h" Infotainment"#);

        // There must be only 1 message
        assert_eq!(db.messages.len(), 1);

        // There must be only one signal inside the message
        let msg = &db.messages[0];
        assert_eq!(msg.signals.len(), 1);

        // check on the signal
        let sig = &msg.signals[0];
        assert_eq!(sig.name, "Engine_Speed");
        assert_eq!(sig.bit_start, 48);
        assert_eq!(sig.bit_length, 8);
        assert_eq!(sig.endian, 1);
        assert_eq!(sig.sign, 0);
        assert_eq!(sig.factor, 1.0);
        assert_eq!(sig.offset, 0.0);
        assert_eq!(sig.min, 0.0);
        assert_eq!(sig.max, 255.0);
        assert_eq!(sig.unit_of_measurement, "km/h");

        // Receiver nodes
        assert_eq!(sig.receiver_nodes.len(), 1);
        assert_eq!(sig.receiver_nodes[0].name, "Infotainment");

        // Empty Value Table
        assert!(sig.value_table.is_empty());
    }

    #[test]
    fn test_parse_add_nodes() {
        // Crea un Database con un messaggio esistente
        let mut db: Database = Database::default();
        db.messages.push(Message {
            id: 2549940736,
            id_hex: String::from("0x98012340"),
            name: String::from("TestMessage"),
            byte_length: 8,
            sender_nodes: vec![Node {
                name: "Motor".to_string(),
                comment: "".to_string(),
            }],
            signals: vec![],
            comment: String::new(),
        });

        // Example Line
        db.parse_add_nodes("BO_TX_BU_ 2549940736 : Infotainment,Gateway;");

        // Check all expected nodes are present
        let msg: &Message = db.get_message_by_id(2549940736).unwrap();
        assert_eq!(msg.sender_nodes.len(), 3);
        assert!(msg.sender_nodes.iter().any(|n| n.name == "Motor"));
        assert!(msg.sender_nodes.iter().any(|n| n.name == "Infotainment"));
        assert!(msg.sender_nodes.iter().any(|n| n.name == "Gateway"));

        // Add again the same lane and check there are no duplicates
        db.parse_add_nodes("BO_TX_BU_ 2549940736 : Infotainment,Gateway;");
        let msg: &Message = db.get_message_by_id(2549940736).unwrap();
        assert_eq!(msg.sender_nodes.len(), 3);
    }

    #[test]
    fn test_parse_message_comments_id_found() {
        // Prepare a db
        let mut db: Database = Database::default();
        db.messages.push(Message {
            id: 2549880610,
            id_hex: format!("{:X}", 2549880610u64),
            name: "TestMessage".to_string(),
            byte_length: 8,
            sender_nodes: vec![],
            signals: vec![],
            comment: String::new(),
        });

        // Example Line
        let line = r#"CM_ BO_ 2549880610 "Example comment";"#;
        db.parse_message_comments(line);

        // Check comment
        assert_eq!(db.messages[0].comment, "Example comment");
    }

    #[test]
    fn test_parse_message_comments_id_not_found() {
        // Empty Database
        let mut db: Database = Database::default();

        // Example Line
        let line = r#"CM_ BO_ 999999 "Questo non verrà mai assegnato";"#;
        db.parse_message_comments(line);

        // Message not found -> No comment assigned
        assert!(db.messages.is_empty());
    }

    #[test]
    fn test_parse_signal_comments_id_found() {
        // Prepare a db and a message to test
        let mut db: Database = Database::default();
        let msg: Message = Message {
            id: 1635u64,
            id_hex: format!("{:X}", 1635u64),
            name: "Gateway_01".to_string(),
            byte_length: 8,
            sender_nodes: vec![],
            signals: vec![Signal {
                name: "NetStatus".to_string(),
                ..Default::default()
            }],
            comment: String::new(),
        };
        db.messages.push(msg);

        // Example Line (con rimozione indentazione)
        let line = r#"CM_ SG_ 1635 NetStatus "Example of
                multiline
                comment."; "#;

        let expected_output = r#"Example of
                multiline
                comment."#;

        // Remove initial space of each line
        let expected_output = expected_output
            .lines()
            .map(|l| l.trim_start())
            .collect::<Vec<_>>()
            .join("\n");

        println!("line = {}", line);

        // parse the comment
        db.parse_signal_comments(&line);

        // Verifica che il commento sia stato assegnato correttamente
        let msg: &mut Message = db.get_message_by_id_mut(1635u64).unwrap();
        let sig: &Signal = msg.get_signal_by_name("NetStatus").unwrap();
        assert_eq!(sig.comment, expected_output);
    }

    #[test]
    fn test_parse_signal_comments_id_not_found() {
        // Database vuoto
        let mut db: Database = Database::default();

        // Riga che fa riferimento a un ID inesistente
        let line: &'static str = r#"CM_ SG_ 9999 SomeSignal "This comment will not be assigned";"#;

        // Parsing
        db.parse_signal_comments(line);

        // Nessun messaggio aggiunto
        assert!(db.messages.is_empty());
    }

     #[test]
    fn test_parse_node_comments_updates_all_places() {
        // --- Setup database ---
        let mut db: Database = Database::default();

        // Database Node
        db.nodes.push(Node {
            name: "Gateway".to_string(),
            comment: String::new(),
        });

        // Message with Sender Node
        let mut msg: Message = Message {
            id: 1000,
            id_hex: "0x3E8".to_string(),
            name: "TestMessage".to_string(),
            byte_length: 8,
            sender_nodes: vec![Node {
                name: "Gateway".to_string(),
                comment: String::new(),
            }],
            signals: vec![],
            comment: String::new(),
        };

        // Signal with receiver Node
        let sig: Signal = Signal {
            name: "TestSignal".to_string(),
            receiver_nodes: vec![Node {
                name: "Gateway".to_string(),
                comment: String::new(),
            }],
            ..Default::default()
        };
        msg.signals.push(sig);

        db.messages.push(msg);

        // Example input
        let input = r#"CM_ BU_ Gateway "Node comment line 1
        line 2";"#;

        // --- Esegui ---
        db.parse_node_comments(input);

        // --- Expected comment normalizzato ---
        let expected_comment: &'static str = "Node comment line 1\nline 2";

        // Check on Database node
        assert_eq!(
            db.get_nodes_by_name_mut("Gateway").unwrap().comment,
            expected_comment
        );

        // Check on Message node
        let sender_comment = db.messages[0]
            .get_sender_nodes_by_name_mut("Gateway")
            .unwrap()
            .comment
            .clone();
        assert_eq!(sender_comment, expected_comment);

        // check on Signal Node
        let receiver_comment = db.messages[0].signals[0]
            .get_receiver_nodes_by_name_mut("Gateway")
            .unwrap()
            .comment
            .clone();
        assert_eq!(receiver_comment, expected_comment);
    }

    #[test]
    fn test_parse_value_table() {
        let mut db: Database = Database::default();

        // Add a message
        db.parse_messages("BO_ 960 Key_Status: 4 BCM");

        // Add a signal
        db.parse_signal(r#"SG_ Engine_Speed : 48|8@1+ (1,0) [0|255] "km/h" Infotainment"#);

        // Example Line
        db.parse_value_table(r#"VAL_ 960 Engine_Speed 0 "Off" 1 "On" 255 "Error";"#);

        // Check message presence
        assert_eq!(db.messages.len(), 1);

        // Check signal presence
        let msg = &db.messages[0];
        assert_eq!(msg.signals.len(), 1);

        // Check value table
        let sig = &msg.signals[0];
        assert_eq!(sig.value_table.len(), 3);
        assert_eq!(sig.value_table.get(&0), Some(&"Off".to_string()));
        assert_eq!(sig.value_table.get(&1), Some(&"On".to_string()));
        assert_eq!(sig.value_table.get(&255), Some(&"Error".to_string()));
    }

    fn build_test_db() -> Database {
        Database {
            version: "1.0".into(),
            bit_timing: "BS_".into(),
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
                    byte_length: 8,
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
}
