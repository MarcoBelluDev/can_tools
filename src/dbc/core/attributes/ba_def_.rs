use crate::dbc::core::strings::collect_all_quoted;
use crate::dbc::types::{
    attributes::{AttrType, AttributeDef, AttributeSpec},
    database::DatabaseDBC,
};

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats (global BA_ attributes):
    // BA_DEF_  "DBName" STRING;
    // BA_DEF_  "Baudrate" INT 1 1000000;
    // BA_DEF_  "BaudrateCANFD" INT 1 16000000;
    // BA_DEF_  "NmhBaseAddress" HEX 0 536870911;
    // BA_DEF_ "IsCan" ENUM "No", "Yes";

    // keep a copy to extract quoted string for Enum
    let line_copy: &str = line.trim().trim_end_matches(';');

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // "BA_DEF_"
    match parts.next() {
        Some("BA_DEF_") => {}
        _ => return,
    }

    // Attribute token (e.g., "\"DBName\"")
    let name: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    // Attribute token (e.g., "\"STRING\"")
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
            let mut quoted: Vec<String> = collect_all_quoted(line_copy);
            if !quoted.is_empty() {
                quoted.remove(0); // remove attribute name
            }
            attr_def.enum_values = quoted;
        }
        _ => {}
    }

    attr_spec.def = Some(attr_def);
    db.db_attr_spec.insert(name.to_string(), attr_spec);
}
