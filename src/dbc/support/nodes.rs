use crate::types::database::{Database, NodeDB};

/// Decode the BU_ line listing node names and register them in the database.
/// Example: `BU_: ECU1 ECU2 ECU3`
pub(crate) fn decode(db: &mut Database, line: &str) {
    // Split tokens, skip the "BU_:"
    let mut parts = line.split_ascii_whitespace();
    let first: Option<&str> = parts.next();
    if first != Some("BU_:") && first != Some("BU_") {
        return;
    }

    for name in parts {
        if name.is_empty() {
            continue;
        }
        // Insert only if not already present
        if db.get_nodes_by_name(name).is_none() {
            db.add_node(NodeDB {
                name: name.to_string(),
                comment: String::new(),
                messages_sent: Vec::new(),
            });
        }
    }
}

/// Parse a node-level comment:
/// `CM_ BU_ NodeName "Comment..."`
pub(crate) fn comments(db: &mut Database, text: &str) {
    let mut parts = text.split_ascii_whitespace();
    if parts.next() != Some("CM_") {
        return;
    }
    if parts.next() != Some("BU_") {
        return;
    }
    let node_name = match parts.next() {
        Some(n) => n,
        None => return,
    };

    // Extract the quoted comment as-is (preserving inner spaces/newlines)
    let first_quote = match text.find('\"') {
        Some(p) => p,
        None => return,
    };
    let last_quote = match text.rfind('\"') {
        Some(p) if p > first_quote => p,
        _ => return,
    };
    let comment = text[first_quote + 1..last_quote].to_string();

    // Update single source of truth
    if let Some(node) = db.get_nodes_by_name_mut(node_name) {
        node.comment = comment;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_nodes() {
        let mut db = Database::default();
        decode(&mut db, "BU_: ECU1 ECU2 ECU3");
        assert_eq!(db.nodes.len(), 3);
        assert_eq!(db.nodes[0].name, "ECU1");
    }

    #[test]
    fn test_comments() {
        let mut db = Database::default();
        decode(&mut db, "BU_: Gateway");
        comments(&mut db, r#"CM_ BU_ Gateway "Main gateway";"#);
        assert_eq!(
            db.get_nodes_by_name("Gateway").unwrap().comment,
            "Main gateway"
        );
    }
}
