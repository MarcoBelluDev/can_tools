use crate::types::database::{DatabaseDBC, NodeKey};

/// Parse `BO_TX_BU_` lines assigning transmit-capable nodes to a message.
/// Example: `BO_TX_BU_ 123 :NodeA,NodeB;`
pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // split in parts and remove final ";"
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // 1) "BA_"
    match parts.next() {
        Some("BO_TX_BU_") => {}
        _ => return,
    }

    // 2) ID
    let id: u32 = match parts.next() {
        Some(a) => a.parse::<u32>().unwrap_or(0),
        None => return,
    };

    if id == 0 {
        return;
    }

    // 3) Node Parts
    let nodes_part: &str = match parts.next() {
        Some(a) => a.trim_start_matches(':'),
        None => return,
    };

    // Resolve/create NodeIds first (no &mut msg held)
    let mut node_keys: Vec<NodeKey> = Vec::new();
    for token in nodes_part.split(',') {
        let name: &str = token.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(k) = db.get_node_key_by_name(name) {
            node_keys.push(k);
        }
    }
    if node_keys.is_empty() {
        return;
    }

    // take MessageKey once before mutable borrow
    let Some(msg_key) = db.get_msg_key_by_id(&id) else {
        return;
    };

    // Update the MessageDB
    {
        if let Some(msg) = db.get_message_by_key_mut(msg_key) {
            for &nk in &node_keys {
                if !msg.sender_nodes.contains(&nk) {
                    msg.sender_nodes.push(nk);
                }
            }
        } else {
            return;
        }
    } // end of &mut MessageDB

    // Update the nodes
    for &nk in &node_keys {
        let _ = db.add_sender_relation(msg_key, nk);
    }
}
