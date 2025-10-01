use crate::dbc::types::{
    attributes::{AttrType, AttributeDef, AttributeSpec, AttributeValue},
    database::DatabaseDBC,
};

/// `BA_ "Attribute" BU_ <Name> <value>;`
pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    let mut parts = line.trim_end_matches(';').split_ascii_whitespace();

    // 1) "BA_"
    match parts.next() {
        Some("BA_") => {}
        _ => return,
    }

    // 2) Attribute name (e.g., "\"DBName\"")
    let attr_tok: &str = match parts.next() {
        Some(a) => a,
        None => return,
    };
    let attr_name: &str = attr_tok.trim_matches('"');

    // 3) "BU_"
    match parts.next() {
        Some("BU_") => {}
        _ => return,
    }

    // 4) Retrieve node name
    let Some(node_name) = parts.next() else {
        return;
    };

    // 5) Rebuild the remaining tail to preserve spaces inside quoted values
    let rest_joined: String = parts.collect::<Vec<_>>().join(" ");
    let rest: &str = rest_joined.trim();

    // 6) Extract the value:
    //    - if it starts with a quote => take content up to the next quote
    //    - otherwise treat the remainder as the numeric value (already ';'-stripped)
    let value: &str = if let Some(inner) = rest.strip_prefix('"') {
        match inner.find('"') {
            Some(end) => &inner[..end],
            None => return, // unmatched quotes
        }
    } else {
        rest
    };

    // immutable borrow to Attribute Specification
    let attr_spec: &AttributeSpec = match db.node_attr_spec.get(attr_name) {
        Some(spec) => spec,
        None => return, // exit immediately
    };

    // immutable borrow to Attribute Definition
    let attr_def: &AttributeDef = match attr_spec.def.as_ref() {
        Some(d) => d,
        None => return,
    };

    // check the type from the Attribute Definition
    // if value is not found, use default value from Attribute Specification
    let attr_value: AttributeValue = match attr_def.kind {
        AttrType::String => AttributeValue::Str(value.to_string()),
        AttrType::Int => {
            let Ok(num) = value.parse::<i64>() else {
                return;
            };
            AttributeValue::Int(num)
        }
        AttrType::Hex => {
            let Ok(num) = value.parse::<u64>() else {
                return;
            };
            AttributeValue::Hex(num)
        }
        AttrType::Float => {
            let Ok(num) = value.parse::<f64>() else {
                return;
            };
            AttributeValue::Float(num)
        }
        AttrType::Enum => {
            // Accept only numeric index into enum_values
            let Ok(idx) = value.parse::<usize>() else {
                return;
            };
            let Some(v) = attr_def.enum_values.get(idx) else {
                return;
            };
            AttributeValue::Enum(v.clone())
        }
    };

    if let Some(node) = db.get_node_by_name_mut(node_name)
        && let Some(slot) = node.attributes.get_mut(attr_name)
    {
        *slot = attr_value;
    }
}
