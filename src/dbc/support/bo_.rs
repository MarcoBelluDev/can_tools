use crate::types::database::Database;

const CAN_EFF_MASK: u32 = 0x1FFF_FFFF; // 29 bit
const CAN_SFF_MASK: u32 = 0x0000_07FF; // 11 bit

#[inline]
fn id_to_hex(id: u32) -> String {
    if id <= CAN_SFF_MASK {
        format!("0x{:03X}", id)
    } else {
        format!("0x{:08X}", id & CAN_EFF_MASK)
    }
}

/// Decode a `BO_` line robustly using `:` as separator between name and length.
/// Accepts both: `BO_ 123 NAME: 8 Node` and `BO_ 123 NAME : 8 Node`.
pub(crate) fn decode(db: &mut Database, line: &str) {
    let line: &str = line.trim();
    if !line.starts_with("BO_") {
        return;
    }

    // Strip leading "BO_"
    let after: &str = line.trim_start_matches("BO_").trim();

    // 1) ID (first token)
    let mut split_once = after.splitn(2, char::is_whitespace);
    let id_str: &str = split_once.next().unwrap_or("0");
    let rest: &str = split_once.next().unwrap_or("").trim();
    let id: u32 = id_str.parse::<u32>().unwrap_or(0);

    // 2) NAME (everything up to the first ':')
    let colon_pos: usize = match rest.find(':') {
        Some(p) => p,
        None => return,
    };
    let name: String = rest[..colon_pos].trim().trim_end_matches(':').to_string();

    // 3) After ':' → <len> <sender?>
    let mut it = rest[colon_pos + 1..].trim().split_ascii_whitespace();
    let byte_length: u16 = it.next().and_then(|t| t.parse::<u16>().ok()).unwrap_or(0);
    let sender_name: &str = it.next().unwrap_or("").trim_end_matches(';');

    let id_hex: String = id_to_hex(id);

    db.add_message_if_absent(&name, id, &id_hex, byte_length, sender_name);
}
