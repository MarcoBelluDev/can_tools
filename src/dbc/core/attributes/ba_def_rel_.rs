use crate::dbc::core::strings::collect_all_quoted;
use crate::dbc::types::{
    attributes::{AttrValueType, AttributeSpec},
    database::DatabaseDBC,
};

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats (relational attribute definitions):
    // BA_DEF_REL_ BU_SG_REL_  "GenSigTimeoutTime" INT 0 65535;
    // BA_DEF_REL_ BU_BO_REL_  "GenMsgTimeoutTime" INT 0 65535;
    // BA_DEF_REL_ BU_EV_REL_  "SomeEnvRelAttr"   ENUM "Off","On";

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // "BA_DEF_REL_"
    match parts.next() {
        Some("BA_DEF_REL_") => {}
        _ => return,
    }

    // Reletionship (e.g., BU_SG_REL_)
    let relation: &str = match parts.next() {
        Some(a) => a,
        None => return,
    };

    // Attribute token (e.g., "\"GenSigTimeoutTime\"")
    let name: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    // Attribute type (e.g., INT/HEX/FLOAT/STRING/ENUM)
    let attr_type: &str = match parts.next() {
        Some(a) => a,
        None => return,
    };

    let mut attr_spec: AttributeSpec = AttributeSpec::default();

    match attr_type {
        "STRING" => {
            attr_spec.value_type = AttrValueType::String;
        }
        "INT" => {
            attr_spec.value_type = AttrValueType::Int;
            attr_spec.int_min = match parts.next() {
                Some(a) => Some(a.parse::<i64>().unwrap_or_default()),
                None => return,
            };
            attr_spec.int_max = match parts.next() {
                Some(a) => Some(a.parse::<i64>().unwrap_or_default()),
                None => return,
            };
        }
        "HEX" => {
            attr_spec.value_type = AttrValueType::Hex;
            attr_spec.hex_min = match parts.next() {
                Some(a) => Some(a.parse::<u64>().unwrap_or_default()),
                None => return,
            };
            attr_spec.hex_max = match parts.next() {
                Some(a) => Some(a.parse::<u64>().unwrap_or_default()),
                None => return,
            };
        }
        "FLOAT" => {
            attr_spec.value_type = AttrValueType::Float;
            attr_spec.float_min = match parts.next() {
                Some(a) => Some(a.parse::<f64>().unwrap_or_default()),
                None => return,
            };
            attr_spec.float_max = match parts.next() {
                Some(a) => Some(a.parse::<f64>().unwrap_or_default()),
                None => return,
            };
        }
        "ENUM" => {
            attr_spec.value_type = AttrValueType::Enum;
            let mut quoted: Vec<String> = collect_all_quoted(line);
            if !quoted.is_empty() {
                // First quoted token is the attribute name; remove it.
                quoted.remove(0);
            }
            attr_spec.enum_values = quoted;
        }
        _ => {}
    }

    match relation {
        "BU_SG_REL_" => {
            attr_spec.name = name.to_string();
            db.rel_attr_spec_bu_sg.insert(name.to_string(), attr_spec);
        }
        "BU_BO_REL_" => {
            attr_spec.name = name.to_string();
            db.rel_attr_spec_bu_bo.insert(name.to_string(), attr_spec);
        }
        _ => {}
    }
}
