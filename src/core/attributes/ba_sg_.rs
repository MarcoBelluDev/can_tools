use crate::types::{
    attributes::{AttrValueType, AttributeSpec, AttributeValue},
    database::CanDatabase,
};

/// `BA_ "Attribute" SG_ <ID msg> <sig_name> <value>;`
pub(crate) fn decode(db: &mut CanDatabase, line: &str) {
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

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

    // 3) "SG_"
    match parts.next() {
        Some("SG_") => {}
        _ => return,
    }

    // 4) message id (numeric)
    let Some(msg_id_tok) = parts.next() else {
        return;
    };
    let Ok(msg_id) = msg_id_tok.parse::<u32>() else {
        return;
    };

    // 5) Retrieve sig name
    let Some(sig_name) = parts.next() else {
        return;
    };

    // 6) Rebuild the remaining tail to preserve spaces inside quoted values
    let rest_joined: String = parts.collect::<Vec<_>>().join(" ");
    let rest: &str = rest_joined.trim();

    // 7) Extract the value:
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
    let attr_spec: &AttributeSpec = match db.attr_spec.get(attr_name) {
        Some(spec) => spec,
        None => return, // exit immediately
    };

    // immutable borrow to Attribute Definition

    let attr_value: AttributeValue = match attr_spec.value_type {
        AttrValueType::String => AttributeValue::Str(value.to_string()),
        AttrValueType::Int => {
            let Ok(num) = value.parse::<i64>() else {
                return;
            };
            AttributeValue::Int(num)
        }
        AttrValueType::Hex => {
            let Ok(num) = value.parse::<u64>() else {
                return;
            };
            AttributeValue::Hex(num)
        }
        AttrValueType::Float => {
            let Ok(num) = value.parse::<f64>() else {
                return;
            };
            AttributeValue::Float(num)
        }
        AttrValueType::Enum => {
            // Accept only numeric index into enum_values
            let Ok(idx) = value.parse::<usize>() else {
                return;
            };
            let Some(v) = attr_spec.enum_values.get(idx) else {
                return;
            };
            AttributeValue::Enum(v.clone())
        }
    };

    // 9) assign the value: resolve the signal within the specific message ID
    let sig_key_opt = {
        let msg = match db.get_message_by_id(msg_id) {
            Some(m) => m,
            None => return,
        };
        msg.signals.iter().copied().find(|&sk| {
            db.get_sig_by_key(sk)
                .is_some_and(|s| s.name.eq_ignore_ascii_case(sig_name))
        })
    };

    if let Some(sk) = sig_key_opt
        && let Some(sig) = db.get_sig_by_key_mut(sk)
        && let Some(slot) = sig.attributes.get_mut(attr_name)
    {
        *slot = attr_value;
    }
}
