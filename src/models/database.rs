use std::collections::HashMap;

use crate::models::message::Message;
use crate::models::node::Node;
use crate::models::signal::Signal;

#[derive(Default, Clone)]
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

    pub fn get_message_by_id_hex(&self, id_hex: &str) -> Option<&Message> {
        self.messages
            .iter()
            .find(|msg| msg.id_hex.eq_ignore_ascii_case(id_hex))
    }

    pub fn get_message_by_name(&self, name: &str) -> Option<&Message> {
        self.messages
            .iter()
            .find(|msg| msg.name.eq_ignore_ascii_case(name))
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
        let sender_node: String = parts[4].to_string();

        let msg: Message = Message {
            id,
            id_hex,
            name,
            byte_length,
            sender_node,
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
                    name: "Motor".to_string()
                },
                Node {
                    name: "Infotainment".to_string()
                },
                Node {
                    name: "Gateway".to_string()
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
        assert_eq!(msg.id, 960); // decimal ID
        assert_eq!(msg.id_hex, "0x3C0"); // hex ID
        assert_eq!(msg.name, "Key_Status"); // nome senza ":"
        assert_eq!(msg.byte_length, 4); // lunghezza in byte
        assert_eq!(msg.sender_node, "BCM"); // sender node
        assert!(msg.signals.is_empty()); // inizialmente vuoto
        assert!(msg.comment.is_empty()); // nessun commento iniziale
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
            nodes: vec![],
            messages: vec![
                Message {
                    id: 100,
                    id_hex: "0x64".into(),
                    name: "Motor_01".into(),
                    byte_length: 8,
                    sender_node: "Motor".into(),
                    signals: vec![],
                    comment: "Test comment".into(),
                },
                Message {
                    id: 200,
                    id_hex: "0xC8".into(),
                    name: "Game_01".into(),
                    byte_length: 4,
                    sender_node: "Infotainment".into(),
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
}
