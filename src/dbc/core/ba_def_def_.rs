use crate::dbc::types::{
    attributes::{AttrType, AttributeValue},
    database::DatabaseDBC,
};

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats (global BA_ attributes):
    // BA_DEF_DEF_  "DBName" "";
    // BA_DEF_DEF_  "GenMsgDelayTime" 0;
    // BA_DEF_DEF_  "SyncJumpWidthCANFDMin" 50;
    // BA_DEF_DEF_  "IsCan" "Yes";

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // "BA_DEF_"
    match parts.next() {
        Some("BA_DEF_DEF_") => {}
        _ => return,
    }

    // Attribute token (e.g., "\"DBName\"")
    let name: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    let value: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    if let Some(attr_spec) = db.db_attr_spec.get_mut(name)
        && let Some(attr_def) = &attr_spec.def
    {
        match attr_def.kind {
            AttrType::String => {
                attr_spec.default = Some(AttributeValue::Str(value.to_string()));
            }
            AttrType::Int => {
                let Ok(num) = value.parse::<i64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Int(num));
            }
            AttrType::Hex => {
                let Ok(num) = value.parse::<u64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Hex(num));
            }
            AttrType::Float => {
                let Ok(num) = value.parse::<f64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Float(num));
            }
            AttrType::Enum => {
                // Accept only string default for ENUM
                if attr_def.enum_values.iter().any(|s| s == value) {
                    attr_spec.default = Some(AttributeValue::Str(value.to_string()));
                }
            }
        }
    }

    if let Some(attr_spec) = db.node_attr_spec.get_mut(name)
        && let Some(attr_def) = &attr_spec.def
    {
        match attr_def.kind {
            AttrType::String => {
                attr_spec.default = Some(AttributeValue::Str(value.to_string()));
            }
            AttrType::Int => {
                let Ok(num) = value.parse::<i64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Int(num));
            }
            AttrType::Hex => {
                let Ok(num) = value.parse::<u64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Hex(num));
            }
            AttrType::Float => {
                let Ok(num) = value.parse::<f64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Float(num));
            }
            AttrType::Enum => {
                // Accept only string default for ENUM
                if attr_def.enum_values.iter().any(|s| s == value) {
                    attr_spec.default = Some(AttributeValue::Str(value.to_string()));
                }
            }
        }
    }

    if let Some(attr_spec) = db.msg_attr_spec.get_mut(name)
        && let Some(attr_def) = &attr_spec.def
    {
        match attr_def.kind {
            AttrType::String => {
                attr_spec.default = Some(AttributeValue::Str(value.to_string()));
            }
            AttrType::Int => {
                let Ok(num) = value.parse::<i64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Int(num));
            }
            AttrType::Hex => {
                let Ok(num) = value.parse::<u64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Hex(num));
            }
            AttrType::Float => {
                let Ok(num) = value.parse::<f64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Float(num));
            }
            AttrType::Enum => {
                // Accept only string default for ENUM
                if attr_def.enum_values.iter().any(|s| s == value) {
                    attr_spec.default = Some(AttributeValue::Str(value.to_string()));
                }
            }
        }
    }

    if let Some(attr_spec) = db.sig_attr_spec.get_mut(name)
        && let Some(attr_def) = &attr_spec.def
    {
        match attr_def.kind {
            AttrType::String => {
                attr_spec.default = Some(AttributeValue::Str(value.to_string()));
            }
            AttrType::Int => {
                let Ok(num) = value.parse::<i64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Int(num));
            }
            AttrType::Hex => {
                let Ok(num) = value.parse::<u64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Hex(num));
            }
            AttrType::Float => {
                let Ok(num) = value.parse::<f64>() else {
                    return;
                };
                attr_spec.default = Some(AttributeValue::Float(num));
            }
            AttrType::Enum => {
                // Accept only string default for ENUM
                if attr_def.enum_values.iter().any(|s| s == value) {
                    attr_spec.default = Some(AttributeValue::Str(value.to_string()));
                }
            }
        }
    }
}
