use crate::dbc::core::strings::collect_all_quoted;
use crate::dbc::types::{
    attributes::{AttrObject, AttrValueType, AttributeSpec},
    database::DatabaseDBC,
};

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats (global BA_ attributes):
    // BA_DEF_ BU_  "NodeLayerModules" STRING;
    // BA_DEF_ BU_  "GenNodSleepTime" INT 0 1000000;
    // BA_DEF_ BU_  "NodeAddress" HEX 0 4536436;
    // BA_DEF_ BU_  "SamplePointMin" FLOAT 50.0 99.9;
    // BA_DEF_ BU_  "GenNodAutoGenSnd" ENUM "No", "Yes";

    // keep a copy to extract quoted string for Enum
    let line_copy: &str = line.trim().trim_end_matches(';');

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // "BA_DEF_"
    match parts.next() {
        Some("BA_DEF_") => {}
        _ => return,
    }

    // "BU_"
    match parts.next() {
        Some("BU_") => {}
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
            let mut quoted: Vec<String> = collect_all_quoted(line_copy);
            if !quoted.is_empty() {
                quoted.remove(0); // remove attribute name
            }
            attr_spec.enum_values = quoted;
        }
        _ => {}
    }

    attr_spec.name = name.to_string();
    attr_spec.type_of_object = AttrObject::Node;
    db.attr_spec.insert(name.to_string(), attr_spec);
}
