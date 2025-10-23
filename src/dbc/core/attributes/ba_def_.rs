use crate::dbc::core::strings::collect_all_quoted;
use crate::dbc::types::{
    attributes::{AttrObject, AttrType, AttributeSpec},
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

    let mut attr_spec: AttributeSpec = AttributeSpec::default();

    match attr_type {
        "STRING" => {
            attr_spec.kind = AttrType::String;
        }
        "INT" => {
            attr_spec.kind = AttrType::Int;
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
            attr_spec.kind = AttrType::Hex;
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
            attr_spec.kind = AttrType::Float;
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
            attr_spec.kind = AttrType::Enum;
            let mut quoted: Vec<String> = collect_all_quoted(line_copy);
            if !quoted.is_empty() {
                quoted.remove(0); // remove attribute name
            }
            attr_spec.enum_values = quoted;
        }
        _ => {}
    }

    attr_spec.name = name.to_string();
    attr_spec.type_of_object = AttrObject::Database;
    db.attr_spec.insert(name.to_string(), attr_spec);
}
