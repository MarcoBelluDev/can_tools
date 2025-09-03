use crate::dbc::types::{
    attributes::{AttrType, AttributeValue},
    database::DatabaseDBC,
};

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats (global BA_ attributes):
    // BA_ "DBName" "TestCAN";
    // BA_ "BusType" "CAN FD";
    // BA_ "Baudrate" 500000;
    // BA_ "BaudrateCANFD" 2000000;

    // ...plus other attributes listed below.

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // 1) "BA_"
    match parts.next() {
        Some("BA_") => {}
        _ => return,
    }

    // 2) Attribute name (e.g., "\"DBName\"")
    let attr_name: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    // 3) Rebuild the remaining tail to preserve spaces inside quoted values
    let rest_joined: String = parts.collect::<Vec<_>>().join(" ");
    let rest: &str = rest_joined.trim();

    // 4) Extract the value:
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

    if attr_name == "DBName" {
        db.name = value.to_string();
        return;
    }

    if let Some(attr_spec) = db.db_attr_spec.get_mut(attr_name)
        && let Some(attr_def) = &attr_spec.def
    {
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
        db.attributes.insert(attr_name.to_string(), attr_value);
    }
}
