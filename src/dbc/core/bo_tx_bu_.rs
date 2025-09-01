use crate::dbc::types::database::{DatabaseDBC, NodeKey};

/// Parse `BO_TX_BU_` lines assigning transmit-capable nodes to a message.
/// Example: `BO_TX_BU_ 123 : NodeA,NodeB;`
pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    let l: &str = line.trim();
    if !l.starts_with("BO_TX_BU_") {
        return;
    }

    // Find first numeric ID
    let id: u32 = l
        .split_ascii_whitespace()
        .filter_map(|w| w.parse::<u32>().ok())
        .next()
        .unwrap_or(0);
    if id == 0 {
        return;
    }

    // Take substring after the id and then after the colon
    let after_id: &str = match l.find(&id.to_string()) {
        Some(pos) => &l[pos + id.to_string().len()..],
        None => return,
    };
    let nodes_part: &str = match after_id.find(':') {
        Some(p) => &after_id[p + 1..],
        None => return,
    };
    let nodes_part = nodes_part.trim().trim_end_matches(';');

    // Resolve/create NodeIds first (no &mut msg held)
    let mut node_keys: Vec<NodeKey> = Vec::new();
    for token in nodes_part.split([',', ' ']) {
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
        db.add_tx_msg_for_node(nk, msg_key);
    }
}
