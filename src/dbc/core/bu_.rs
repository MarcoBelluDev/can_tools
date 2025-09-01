use crate::dbc::types::database::DatabaseDBC;

/// Decode the BU_ line listing node names and register them in the database.
/// Example: `BU_: ECU1 ECU2 ECU3`
pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Split tokens, skip the "BU_:"
    let mut parts = line.split_ascii_whitespace();
    let first: Option<&str> = parts.next();
    if first != Some("BU_:") && first != Some("BU_") {
        return;
    }

    for name in parts {
        let name = name.trim();
        if !name.is_empty() {
            // creates if missing, returns existing rif otherwise
            db.add_node_if_absent(name);
        }
    }
}
