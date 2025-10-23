use crate::dbc::types::{
    attributes::{AttrType, AttributeValue},
    database::DatabaseDBC,
};

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats (defaults for relational attributes):
    // BA_DEF_DEF_REL_  "GenSigTimeoutTime" 0;

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // "BA_DEF_DEF_REL_"
    match parts.next() {
        Some("BA_DEF_DEF_REL_") => {}
        _ => return,
    }

    let attr_name: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    let value: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };
    // Locate the attribute spec among relation groups. Attribute names are expected
    // to be unique among relation specs within a DBC.
    if let Some(spec) = db.rel_attr_spec_bu_sg.get_mut(attr_name) {
        match spec.kind {
            AttrType::String => spec.default = Some(AttributeValue::Str(value.to_string())),
            AttrType::Int => match value.parse::<i64>() {
                Ok(n) => spec.default = Some(AttributeValue::Int(n)),
                Err(_) => return,
            },
            AttrType::Hex => match value.parse::<u64>() {
                Ok(n) => spec.default = Some(AttributeValue::Hex(n)),
                Err(_) => return,
            },
            AttrType::Float => match value.parse::<f64>() {
                Ok(n) => spec.default = Some(AttributeValue::Float(n)),
                Err(_) => return,
            },
            AttrType::Enum => {
                // Accept only string default for ENUM
                if spec.enum_values.iter().any(|s| s == value) {
                    spec.default = Some(AttributeValue::Str(value.to_string()));
                }
            }
        }
        return;
    }

    if let Some(spec) = db.rel_attr_spec_bu_bo.get_mut(attr_name) {
        match spec.kind {
            AttrType::String => spec.default = Some(AttributeValue::Str(value.to_string())),
            AttrType::Int => {
                if let Ok(n) = value.parse::<i64>() {
                    spec.default = Some(AttributeValue::Int(n))
                }
            }
            AttrType::Hex => {
                if let Ok(n) = value.parse::<u64>() {
                    spec.default = Some(AttributeValue::Hex(n))
                }
            }
            AttrType::Float => {
                if let Ok(n) = value.parse::<f64>() {
                    spec.default = Some(AttributeValue::Float(n))
                }
            }
            AttrType::Enum => {
                if spec.enum_values.iter().any(|s| s == value) {
                    spec.default = Some(AttributeValue::Str(value.to_string()));
                }
            }
        }
    }
}
