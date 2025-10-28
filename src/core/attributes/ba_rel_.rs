use crate::types::{
    attributes::{AttrValueType, AttributeValue},
    database::DatabaseDBC,
};

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats (relational attribute assignments):
    // BU_SG_REL_:
    //   BA_REL_ "GenSigTimeoutTime" BU_SG_REL_ <NodeName> SG_ <MsgId> <SigName> <value>;
    // BU_BO_REL_:
    //   BA_REL_ "GenMsgTimeoutTime" BU_BO_REL_ <NodeName> BO_ <MsgId> <value>;
    // BU_EV_REL_:
    //   BA_REL_ "SomeEnvRelAttr"   BU_EV_REL_ <NodeName> EV_ <EnvVarName> <value>;

    // ...plus other attributes listed below.

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // 1) "BA_REL_"
    match parts.next() {
        Some("BA_REL_") => {}
        _ => return,
    }

    // 2) Attribute name (e.g., "\"GenSigTimeoutTime\"")
    let attr_name: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    // 3) Relationship (e.g., BU_SG_REL_)
    let relation: &str = match parts.next() {
        Some(a) => a,
        None => return,
    };

    // 4) Node (e.g. Motor_01)
    let node_name: &str = match parts.next() {
        Some(a) => a,
        None => return,
    };
    // Branch by relation value_type
    match relation {
        "BU_SG_REL_" => {
            // SG_ <MsgId> <SigName> <value>
            match parts.next() {
                Some("SG_") => {}
                _ => return,
            }
            let Some(msg_id_tok) = parts.next() else {
                return;
            };
            let Ok(msg_id) = msg_id_tok.parse::<u32>() else {
                return;
            };
            let sig_name: &str = match parts.next() {
                Some(a) => a,
                None => return,
            };

            let rest_joined: String = parts.collect::<Vec<_>>().join(" ");
            let rest: &str = rest_joined.trim();
            let value: &str = if let Some(inner) = rest.strip_prefix('"') {
                match inner.find('"') {
                    Some(end) => &inner[..end],
                    None => return,
                }
            } else {
                rest
            };

            // Resolve spec and parse value
            let spec = match db.rel_attr_spec_bu_sg.get(attr_name) {
                Some(d) => d,
                None => return,
            };

            let attr_value: AttributeValue = match spec.value_type {
                AttrValueType::String => AttributeValue::Str(value.to_string()),
                AttrValueType::Int => match value.parse::<i64>() {
                    Ok(v) => AttributeValue::Int(v),
                    Err(_) => return,
                },
                AttrValueType::Hex => match value.parse::<u64>() {
                    Ok(v) => AttributeValue::Hex(v),
                    Err(_) => return,
                },
                AttrValueType::Float => match value.parse::<f64>() {
                    Ok(v) => AttributeValue::Float(v),
                    Err(_) => return,
                },
                AttrValueType::Enum => {
                    let Ok(idx) = value.parse::<usize>() else {
                        return;
                    };
                    let Some(v) = spec.enum_values.get(idx) else {
                        return;
                    };
                    AttributeValue::Enum(v.clone())
                }
            };

            // Resolve keys and assign
            let nk = match db.get_node_key_by_name(node_name) {
                Some(nk) => nk,
                None => return,
            };
            let msg = match db.get_message_by_id(msg_id) {
                Some(m) => m,
                None => return,
            };
            let sk_opt = msg.signals.iter().copied().find(|&sk| {
                db.get_sig_by_key(sk)
                    .is_some_and(|s| s.name.eq_ignore_ascii_case(sig_name))
            });
            let Some(sk) = sk_opt else { return };

            let entry = db.bu_sg_rel_attributes.entry((nk, sk)).or_default();
            entry.insert(attr_name.to_string(), attr_value);
        }
        "BU_BO_REL_" => {
            // BO_ <MsgId> <value>
            match parts.next() {
                Some("BO_") => {}
                _ => return,
            }
            let Some(msg_id_tok) = parts.next() else {
                return;
            };
            let Ok(msg_id) = msg_id_tok.parse::<u32>() else {
                return;
            };

            let rest_joined: String = parts.collect::<Vec<_>>().join(" ");
            let rest: &str = rest_joined.trim();
            let value: &str = if let Some(inner) = rest.strip_prefix('"') {
                match inner.find('"') {
                    Some(end) => &inner[..end],
                    None => return,
                }
            } else {
                rest
            };

            let spec = match db.rel_attr_spec_bu_bo.get(attr_name) {
                Some(d) => d,
                None => return,
            };

            let attr_value: AttributeValue = match spec.value_type {
                AttrValueType::String => AttributeValue::Str(value.to_string()),
                AttrValueType::Int => match value.parse::<i64>() {
                    Ok(v) => AttributeValue::Int(v),
                    Err(_) => return,
                },
                AttrValueType::Hex => match value.parse::<u64>() {
                    Ok(v) => AttributeValue::Hex(v),
                    Err(_) => return,
                },
                AttrValueType::Float => match value.parse::<f64>() {
                    Ok(v) => AttributeValue::Float(v),
                    Err(_) => return,
                },
                AttrValueType::Enum => {
                    let Ok(idx) = value.parse::<usize>() else {
                        return;
                    };
                    let Some(v) = spec.enum_values.get(idx) else {
                        return;
                    };
                    AttributeValue::Enum(v.clone())
                }
            };

            let nk = match db.get_node_key_by_name(node_name) {
                Some(nk) => nk,
                None => return,
            };
            let msg_key = match db.get_msg_key_by_id(&msg_id) {
                Some(mk) => mk,
                None => return,
            };
            let entry = db.bu_bo_rel_attributes.entry((nk, msg_key)).or_default();
            entry.insert(attr_name.to_string(), attr_value);
        }
        _ => {}
    }
}
