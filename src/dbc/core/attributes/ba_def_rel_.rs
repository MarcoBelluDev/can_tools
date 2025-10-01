use crate::dbc::core::strings::collect_all_quoted;
use crate::dbc::types::{
    attributes::{AttrType, AttributeDef, AttributeSpec},
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

    let mut attr_def: AttributeDef = AttributeDef::default();
    let mut attr_spec: AttributeSpec = AttributeSpec::default();

    attr_def.name = name.to_string();

    match attr_type {
        "STRING" => {
            attr_def.kind = AttrType::String;
        }
        "INT" => {
            attr_def.kind = AttrType::Int;
            attr_def.int_min = match parts.next() {
                Some(a) => Some(a.parse::<i64>().unwrap_or_default()),
                None => return,
            };
            attr_def.int_max = match parts.next() {
                Some(a) => Some(a.parse::<i64>().unwrap_or_default()),
                None => return,
            };
        }
        "HEX" => {
            attr_def.kind = AttrType::Hex;
            attr_def.hex_min = match parts.next() {
                Some(a) => Some(a.parse::<u64>().unwrap_or_default()),
                None => return,
            };
            attr_def.hex_max = match parts.next() {
                Some(a) => Some(a.parse::<u64>().unwrap_or_default()),
                None => return,
            };
        }
        "FLOAT" => {
            attr_def.kind = AttrType::Float;
            attr_def.float_min = match parts.next() {
                Some(a) => Some(a.parse::<f64>().unwrap_or_default()),
                None => return,
            };
            attr_def.float_max = match parts.next() {
                Some(a) => Some(a.parse::<f64>().unwrap_or_default()),
                None => return,
            };
        }
        "ENUM" => {
            attr_def.kind = AttrType::Enum;
            let mut quoted: Vec<String> = collect_all_quoted(line);
            if !quoted.is_empty() {
                // First quoted token is the attribute name; remove it.
                quoted.remove(0);
            }
            attr_def.enum_values = quoted;
        }
        _ => {}
    }

    attr_spec.def = Some(attr_def);

    match relation {
        "BU_SG_REL_" => {
            db.rel_attr_spec_bu_sg.insert(name.to_string(), attr_spec);
        }
        "BU_BO_REL_" => {
            db.rel_attr_spec_bu_bo.insert(name.to_string(), attr_spec);
        }
        _ => {}
    }
}
