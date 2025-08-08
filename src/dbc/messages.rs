use crate::types::database::Database;
use crate::types::message::Message;
use crate::types::node::Node;

const CAN_SFF_MASK: u64 = 0x7FF;        // 11 bit
const CAN_EFF_MASK: u64 = 0x1FFF_FFFF;  // 29 bit
const CAN_EFF_FLAG: u64 = 0x8000_0000;  // flag "extended" stile SocketCAN

pub(crate) fn fct(db: &mut Database, line: &str) {
    // BO_ <ID> <MESSAGE_NAME> : <BYTES_LENGHT> <SENDER_NODE>
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 5 {
        // Too short line are not considered.
        return;
    }

    let id: u64 = parts[1].parse::<u64>().unwrap_or(0); // decimal id
    let id_hex: String = id_to_hex(id); // hexadecimal id
    let name: String = parts[2].trim_end_matches(':').to_string();
    let byte_length: usize = parts[3].parse::<usize>().unwrap_or(0);
    let msgtype: String = if byte_length <= 8 {
        "CAN".to_string()
    } else {
        "CAN FD".to_string()
    };
    let sender_nodes: Vec<Node> = vec![Node {
        name: parts[4].to_string(),
        comment: "".to_string(),
    }];

    let msg: Message = Message {
        id,
        id_hex,
        name,
        byte_length,
        msgtype,
        sender_nodes,
        signals: Vec::new(),
        comment: String::new(),
    };

    db.messages.push(msg);
}

// to hex considering Extended ID
pub(crate) fn id_to_hex(raw: u64) -> String {
    let r: u64 = raw as u64;
    // remove flag, keep only 29 bit
    let id29: u64 = r & CAN_EFF_MASK;
    // Extended if there is specific flag OR it is bigger then 11 bit
    let is_ext: bool = (r & CAN_EFF_FLAG) != 0 || id29 > CAN_SFF_MASK;
    let id_hex: u64 = if is_ext { id29 } else { id29 & CAN_SFF_MASK };
    format!("0x{:X}", id_hex)
}

pub(crate) fn tx_nodes(db: &mut Database, line: &str) {
    // remove "BO_TX_BU_"
    let content = line.trim_start_matches("BO_TX_BU_").trim();

    // Split by ":" → first part is ID, second part is Node list
    let mut parts = content.splitn(2, ':');
    let id_str = parts.next().unwrap_or("").trim();
    let nodes_str = parts.next().unwrap_or("").trim().trim_end_matches(';');

    // Convert ID in u64
    let id: u64 = match id_str.parse() {
        Ok(v) => v,
        Err(_) => return, // return if not a valid number
    };

    // Find message by id
    if let Some(msg) = db.get_message_by_id_mut(id) {
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

pub(crate) fn comments(db: &mut Database, line: &str) {
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
    let msg = match db.get_message_by_id_mut(id) {
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbc_parse_messages() {
        let mut db: Database = Database::default();

        // Example Line
        fct(&mut db, "BO_ 960 Key_Status: 4 BCM");

        // Only one message must be added
        assert_eq!(db.messages.len(), 1);

        let msg: &Message = &db.messages[0];
        assert_eq!(msg.id, 960);
        assert_eq!(msg.id_hex, "0x3C0");
        assert_eq!(msg.name, "Key_Status");
        assert_eq!(msg.byte_length, 4);
        assert_eq!(msg.sender_nodes[0].name, "BCM");
        assert!(msg.signals.is_empty());
        assert!(msg.comment.is_empty());
    }

    #[test]
    fn test_tx_nodes() {
        // generate test Database with one Message
        let mut db: Database = Database::default();
        db.messages.push(Message {
            id: 2549940736,
            id_hex: String::from("0x98012340"),
            name: String::from("TestMessage"),
            byte_length: 8,
            msgtype: "CAN".to_string(),
            sender_nodes: vec![Node {
                name: "Motor".to_string(),
                comment: "".to_string(),
            }],
            signals: vec![],
            comment: String::new(),
        });

        // Example Line
        tx_nodes(&mut db, "BO_TX_BU_ 2549940736 : Infotainment,Gateway;");

        // Check all expected nodes are present
        let msg: &Message = db.get_message_by_id(2549940736).unwrap();
        assert_eq!(msg.sender_nodes.len(), 3);
        assert!(msg.sender_nodes.iter().any(|n| n.name == "Motor"));
        assert!(msg.sender_nodes.iter().any(|n| n.name == "Infotainment"));
        assert!(msg.sender_nodes.iter().any(|n| n.name == "Gateway"));

        // Add again the same lane and check there are no duplicates
        tx_nodes(&mut db, "BO_TX_BU_ 2549940736 : Infotainment,Gateway;");
        let msg: &Message = db.get_message_by_id(2549940736).unwrap();
        assert_eq!(msg.sender_nodes.len(), 3);
    }

    #[test]
    fn test_comments_id_found() {
        // Prepare a db
        let mut db: Database = Database::default();
        db.messages.push(Message {
            id: 2549880610,
            id_hex: format!("{:X}", 2549880610u64),
            name: "TestMessage".to_string(),
            byte_length: 16,
            msgtype: "CAN FD".to_string(),
            sender_nodes: vec![],
            signals: vec![],
            comment: String::new(),
        });

        // Example Line
        let line = r#"CM_ BO_ 2549880610 "Example comment";"#;
        comments(&mut db, line);

        // Check comment
        assert_eq!(db.messages[0].comment, "Example comment");
    }

    #[test]
    fn test_comments_id_not_found() {
        // Empty Database
        let mut db: Database = Database::default();

        // Example Line
        let line = r#"CM_ BO_ 999999 "Questo non verrà mai assegnato";"#;
        comments(&mut db, line);

        // Message not found -> No comment assigned
        assert!(db.messages.is_empty());
    }
}