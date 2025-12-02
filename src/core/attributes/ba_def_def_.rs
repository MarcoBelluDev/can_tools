use crate::types::{
    attributes::{AttrValueType, AttributeValue},
    database::CanDatabase,
};

pub(crate) fn decode(db: &mut CanDatabase, line: &str) {
    // Expected formats:
    // BA_DEF_DEF_ "AttrName" <value>;

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // "BA_DEF_DEF_"
    match parts.next() {
        Some("BA_DEF_DEF_") => {}
        _ => return,
    }

    // Attribute name
    let name: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    // Value token (may be quoted for STRING/ENUM default)
    let value_raw: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    // Find spec & its definition
    let Some(spec) = db.attr_spec.get_mut(name) else {
        return;
    };

    // Parse default according to value_type
    let parsed_default: Option<AttributeValue> = match spec.value_type {
        AttrValueType::String => Some(AttributeValue::Str(value_raw.to_string())),
        AttrValueType::Int => match value_raw.parse::<i64>() {
            Ok(n) => Some(AttributeValue::Int(n)),
            Err(_) => None,
        },
        AttrValueType::Hex => match value_raw.parse::<u64>() {
            Ok(n) => Some(AttributeValue::Hex(n)),
            Err(_) => None,
        },
        AttrValueType::Float => match value_raw.parse::<f64>() {
            Ok(n) => Some(AttributeValue::Float(n)),
            Err(_) => None,
        },
        AttrValueType::Enum => {
            // Only accept if it's one of the enum variants
            if spec.enum_values.iter().any(|s| s == value_raw) {
                Some(AttributeValue::Enum(value_raw.to_string()))
            } else {
                None
            }
        }
    };

    if let Some(default_value) = parsed_default {
        // Save on spec
        spec.default = default_value.clone();

        // And propagate to existing entities for non-DB scopes
        match spec.type_of_object {
            // Do not inject into db.attributes here; BA_ assignments or creator code handles those.
            crate::types::attributes::AttrObject::Database => {}
            crate::types::attributes::AttrObject::Node => {
                db.for_each_node_mut(|node| {
                    node.attributes
                        .insert(name.to_string(), default_value.clone());
                });
            }
            crate::types::attributes::AttrObject::Message => {
                db.for_each_message_mut(|message| {
                    message
                        .attributes
                        .insert(name.to_string(), default_value.clone());
                });
            }
            crate::types::attributes::AttrObject::Signal => {
                db.for_each_signal_mut(|signal| {
                    signal
                        .attributes
                        .insert(name.to_string(), default_value.clone());
                });
            }
        }
    }
}
