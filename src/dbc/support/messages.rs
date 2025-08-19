use crate::types::database::{Database, MessageDB, NodeDB, NodeId};

const CAN_SFF_MASK: u64 = 0x7FF; // 11-bit
const CAN_EFF_MASK: u64 = 0x1FFF_FFFF; // 29-bit

#[inline]
fn id_to_hex(id: u64) -> String {
    if id <= CAN_SFF_MASK {
        format!("0x{:03X}", id)
    } else {
        format!("0x{:08X}", id & CAN_EFF_MASK)
    }
}

/// Decode a `BO_` line robustly using `:` as separator between name and length.
/// Accepts both: `BO_ 123 NAME: 8 Node` and `BO_ 123 NAME : 8 Node`.
pub(crate) fn decode(db: &mut Database, line: &str) {
    let line: &str = line.trim();
    if !line.starts_with("BO_") {
        return;
    }

    // Strip leading "BO_"
    let after: &str = line.trim_start_matches("BO_").trim();

    // 1) ID (first token)
    let mut split_once = after.splitn(2, char::is_whitespace);
    let id_str: &str = split_once.next().unwrap_or("0");
    let rest: &str = split_once.next().unwrap_or("").trim();
    let id: u64 = id_str.parse::<u64>().unwrap_or(0);

    // 2) NAME (everything up to the first ':')
    let colon_pos: usize = match rest.find(':') {
        Some(p) => p,
        None => return,
    };
    let name: String = rest[..colon_pos].trim().trim_end_matches(':').to_string();

    // 3) After ':' → <len> <sender?>
    let mut it = rest[colon_pos + 1..].trim().split_ascii_whitespace();
    let byte_length: u16 = it.next().and_then(|t| t.parse::<u16>().ok()).unwrap_or(0);
    let sender_name: &str = it.next().unwrap_or("").trim_end_matches(';');

    let id_hex = id_to_hex(id);

    // Ensure the sender node exists (if present)
    let sender_node_id: Option<NodeId> = if !sender_name.is_empty() {
        if let Some(nid) = db.get_node_id_by_name(sender_name) {
            Some(nid)
        } else {
            Some(db.add_node(NodeDB {
                name: sender_name.to_string(),
                comment: String::new(),
                messages_sent: Vec::new(),
            }))
        }
    } else {
        None
    };

    let mut msg: MessageDB = MessageDB {
        id,
        id_hex,
        name,
        byte_length,
        msgtype: if byte_length <= 8 {
            "CAN".into()
        } else {
            "CAN FD".into()
        },
        cycle_time: 0,
        sender_nodes: Vec::new(),
        signals: Vec::new(),
        comment: String::new(),
    };

    if let Some(nid) = sender_node_id {
        msg.sender_nodes.push(nid);
    }

    db.add_message(msg);
}

/// Parse `BO_TX_BU_` lines assigning transmit-capable nodes to a message.
/// Example: `BO_TX_BU_ 123 : NodeA,NodeB;`
pub(crate) fn tx_nodes(db: &mut Database, line: &str) {
    let l: &str = line.trim();
    if !l.starts_with("BO_TX_BU_") {
        return;
    }

    // Find first numeric ID
    let id: u64 = l
        .split_ascii_whitespace()
        .filter_map(|w| w.parse::<u64>().ok())
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
    let mut node_ids: Vec<NodeId> = Vec::new();
    for token in nodes_part.split(|c| c == ',' || c == ' ') {
        let name = token.trim();
        if name.is_empty() {
            continue;
        }
        if let Some(nid) = db.get_node_id_by_name(name) {
            node_ids.push(nid);
        } else {
            node_ids.push(db.add_node(NodeDB {
                name: name.to_string(),
                comment: String::new(),
                messages_sent: Vec::new(),
            }));
        }
    }

    // Now update the message
    if let Some(msg) = db.get_message_by_id_mut(id) {
        for nid in node_ids {
            if !msg.sender_nodes.iter().any(|&x| x == nid) {
                msg.sender_nodes.push(nid);
            }
        }
    }
}

/// `CM_ BO_ <ID> "Comment...";`
pub(crate) fn comments(db: &mut Database, line: &str) {
    let mut parts = line.split_ascii_whitespace();
    if parts.next() != Some("CM_") {
        return;
    }
    if parts.next() != Some("BO_") {
        return;
    }

    let id: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    if id == 0 {
        return;
    }

    let line: &str = line.trim_end_matches(';').trim();
    let first: usize = match line.find('\"') {
        Some(p) => p,
        None => return,
    };
    let last: usize = match line.rfind('\"') {
        Some(p) if p > first => p,
        _ => return,
    };
    let comment: &str = &line[first + 1..last];

    if let Some(msg) = db.get_message_by_id_mut(id) {
        msg.comment = comment.to_string();
    }
}

/// `BA_ "GenMsgCycleTime" BO_ <ID> <ms>;`
pub(crate) fn cycle_time(db: &mut Database, line: &str) {
    if !line.contains("GenMsgCycleTime") {
        return;
    }
    let mut parts = line.split_ascii_whitespace();
    parts.next(); // BA_
    parts.next(); // "GenMsgCycleTime"
    if parts.next() != Some("BO_") {
        return;
    }

    let id: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    if id == 0 {
        return;
    }

    if let Some(ct_str) = parts.next() {
        if let Some(msg) = db.get_message_by_id_mut(id) {
            msg.cycle_time = ct_str.trim_end_matches(';').parse::<u16>().unwrap_or(0);
        }
    }
}
