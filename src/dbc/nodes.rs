use crate::types::database::Database;
use crate::types::node::Node;

pub(crate) fn fct(db: &mut Database, line: &str) {
    // Example: BU_: ECU1 ECU2 ECU3 ECU4 etc...

    // Split the lines in part dividere by whitespaces
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Skip "BU_:"
    for part in parts.iter().skip(1) {
        db.nodes.push(Node {
            name: part.to_string(),
            comment: "".to_string(), // initialize empty comment
        });
    }
}

pub(crate) fn comments(db: &mut Database, line: &str) {
    // CM_ BU_ NodeName "Comment..."

    let mut parts = line.split_whitespace();
    parts.next(); // skip CM_
    parts.next(); // skip BU_

    let node_name = match parts.next() {
        Some(name) => name,
        None => return,
    };

    // Find quotes ""
    let first_quote = match line.find('"') {
        Some(pos) => pos,
        None => return,
    };
    let last_quote = match line.rfind('"') {
        Some(pos) if pos > first_quote => pos,
        _ => return,
    };

    // Take comment and normalize whitespaces
    let comment = line[first_quote + 1..last_quote]
        .lines()
        .map(|l| l.trim_start())
        .collect::<Vec<_>>()
        .join("\n");

    // Update in Database.nodes
    if let Some(node) = db.get_nodes_by_name_mut(node_name) {
        node.comment = comment.clone();
    }

    for msg in &mut db.messages {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fct() {
        let mut db: Database = Database::default();
        // Example Line
        fct(&mut db, "BU_: Motor Infotainment Gateway");
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
    fn test_comments() {
        use crate::types::message::Message;
        use crate::types::signal::Signal;

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
            msgtype: "CAN".to_string(),
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

        // --- Execute ---
        comments(&mut db, input);

        // --- Expected comment ---
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
}