use crate::types::{database::Database, message_db::GenMsgSendType};

/// `BA_ "Attribute" BO_ <ID> <value>;`
pub(crate) fn decode(db: &mut Database, line: &str) {
    let mut parts = line.split_ascii_whitespace();
    parts.next(); // BA_
    let attribute: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    match attribute {
        "GenMsgCycleTime" => {
            parts.next(); // BO_
            let id: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            if id == 0 {
                return;
            }

            if let Some(value) = parts.next() {
                if let Some(msg) = db.get_message_by_id_mut(id) {
                    msg.cycle_time = value.trim_end_matches(';').parse::<u16>().unwrap_or(0);
                }
            }
        },
        "GenMsgSendType" => {
            parts.next(); // BO_
            let id: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            if id == 0 {
                return;
            }

            if let Some(value) = parts.next() {
                if let Some(msg) = db.get_message_by_id_mut(id) {
                    match value {
                        "0" => msg.tx_method = GenMsgSendType::Cyclic,
                        "7" => msg.tx_method = GenMsgSendType::IfActive,
                        "8" => msg.tx_method = GenMsgSendType::NoMsgSendType,
                        _ => msg.tx_method = GenMsgSendType::NotUsed,
                    }
                }
            }
        },
        _ => {},
    }
}
