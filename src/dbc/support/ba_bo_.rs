use crate::types::message_db::GenMsgSendType;
use crate::{Database, MessageDB};

/// `BA_ "Attribute" BO_ <ID> <value>;`
pub(crate) fn decode(db: &mut Database, line: &str) {
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();
    parts.next(); // BA_
    let attribute: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    parts.next(); // BO_

    // ID
    let id: u32 = match parts.next().and_then(|s| s.parse::<u32>().ok()) {
        Some(0) | None => return,
        Some(id) => id,
    };

    // Early exit if the message doesn't exist
    let msg: &mut MessageDB = match db.get_message_by_id_mut(id) {
        Some(m) => m,
        None => return,
    };

    // Value as raw string
    let value: &str = match parts.next() {
        Some(v) => v.trim(),
        None => return,
    };

    match attribute {
        "GenMsgCycleTime" => {
            msg.cycle_time = value.parse::<u16>().unwrap_or(0);
        }
        "GenMsgSendType" => {
            msg.tx_method = match value {
                "0" => GenMsgSendType::Cyclic,
                "7" => GenMsgSendType::IfActive,
                "8" => GenMsgSendType::NoMsgSendType,
                _ => GenMsgSendType::NotUsed,
            };
        }
        _ => {}
    }
}
