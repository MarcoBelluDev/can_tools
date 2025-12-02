use crate::types::database::CanDatabase;

/// Decode the BU_ line listing node names and register them in the database.
/// Example: `BU_: ECU1 ECU2 ECU3`
pub(crate) fn decode(db: &mut CanDatabase, line: &str) {
    // Split tokens, skip the "BU_:"
    let mut parts = line.split_ascii_whitespace();
    let first: Option<&str> = parts.next();
    if first != Some("BU_:") && first != Some("BU_") {
        return;
    }

    for name in parts {
        let name = name.trim();
        if !name.is_empty() {
            // creates the node and ignore the NodeKey returned
            let _ = db.add_node(name);
        }
    }
}
