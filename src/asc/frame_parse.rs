use chrono::{Duration, NaiveDateTime};
use std::collections::HashMap;

use crate::{AbsoluteTime, CanFrame, Database};

// Example: 
// 0.016728 1  17334410x       Rx   d 8 3E 42 03 00 39 00 03 01
pub(crate) fn from_line(line: &str, start_abs_time: &AbsoluteTime, db_list: &HashMap<usize, Database>) -> Option<CanFrame> {
    // check first part is a number f64 (timestamp)
    let first_token: &str = line.split_whitespace().next()?;
    if first_token.parse::<f64>().is_err() {
        return None; // Non è una riga CAN valida
    }

    // split line by whitespaces
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return None;
    }

    let timestamp_value: f64 = parts[0].parse().ok()?;

    // absolute time
    let absolute_time: String;
    if let Some(start_time) = start_abs_time.value {
        let seconds: Duration =
            Duration::milliseconds((timestamp_value * 1000.0).round() as i64);
        let abs_time_value: NaiveDateTime = start_time + seconds;
        absolute_time = abs_time_value.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    } else {
        absolute_time = seconds_to_hms_string(timestamp_value);
    }

    let channel: usize = parts[1].parse::<usize>().ok()?;
    let id: String = parts[2].to_string();
    let mut name: String = String::new();
    let mut sender_node: String = String::new();

    // check if there is a dbc in list for that channel
    if let Some(dbc) = db_list.get(&channel) {
        // in .asc traces, ID are represented without 0x so we need to add it
        // they might also have a final x in case of Extended ID
        let id_no_x = id.trim_end_matches(|c| c == 'x' || c == 'X');
        if let Some(msg) = dbc.get_message_by_id_hex(&format!("0x{}", &id_no_x)) {
            name = msg.name.clone();
            if !msg.sender_nodes.is_empty() {
                sender_node = msg.sender_nodes[0].name.clone(); // first sender
            }
        }
    }
    
    let direction: String = parts[3].to_string();
    let byte_length_value: usize = parts[5].parse::<usize>().ok()?;

    // I dati CAN iniziano dopo il campo lunghezza
    let data_start_index = 6;
    let mut data_bytes = Vec::new();
    for p in &parts[data_start_index..] {
        // Si ferma prima di "Length =" se presente
        if *p == "Length" {
            break;
        }
        data_bytes.push(*p);
    }
    let data: String = data_bytes.join(" ");

    let protocol: String = if byte_length_value <= 8 {
        "CAN".to_string()
    } else {
        "CAN FD".to_string()
    };

    Some(CanFrame {
        absolute_time,
        timestamp: format!("{:.6}", timestamp_value),
        timestamp_value,
        channel,
        protocol,
        id,
        name,
        sender_node,
        direction,
        byte_length: byte_length_value.to_string(),
        byte_length_value,
        data,
    })
}

fn seconds_to_hms_string(seconds: f64) -> String {
    let total_millis: u64 = (seconds * 1000.0).round() as u64;

    let hours: u64 = total_millis / 3_600_000;
    let minutes: u64 = (total_millis % 3_600_000) / 60_000;
    let secs: u64 = (total_millis % 60_000) / 1000;
    let millis: u64 = total_millis % 1000;

    format!(
        "2025-01-01 {:02}:{:02}:{:02}.{:03}",
        hours, minutes, secs, millis
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use chrono::{NaiveDate, NaiveDateTime};
    use crate::{Node, Message};

    // ------------------------
    // Helpers
    // ------------------------

    fn abs_none() -> AbsoluteTime {
        // Absolute time not provided -> fallback format will be used
        AbsoluteTime { text: String::new(), value: None }
    }

    fn abs_some(dt: NaiveDateTime) -> AbsoluteTime {
        // Absolute time provided -> relative seconds will be added to this
        AbsoluteTime { text: String::new(), value: Some(dt) }
    }

    fn dt(y: i32, m: u32, d: u32, hh: u32, mm: u32, ss: u32, ms: u32) -> NaiveDateTime {
        // Small utility to build a NaiveDateTime for tests
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
            .and_hms_milli_opt(hh, mm, ss, ms).unwrap()
    }

    fn line_can_8() -> &'static str {
        // byte_length = 8 -> protocol CAN
        "0.016728 1  17334410x       Rx   d 8 3E 42 03 00 39 00 03 01"
    }

    fn line_canfd_12() -> &'static str {
        // byte_length = 12 -> protocol CAN FD
        "0.100000 2  1F334410        Tx   d 12 11 22 33 44 55 66 77 88 99 AA BB CC"
    }

    fn line_with_length_suffix() -> &'static str {
        // Data should stop before the token "Length"
        "0.050000 1  123             Rx   d 8 01 02 03 04 05 06 07 08 Length = 8"
    }

    // Build a HashMap<channel, Database> with a single message on that channel.
    // The message is minimal but enough for lookup by id_hex and sender_nodes[0].
    fn db_map_with_msg(channel: usize, id_hex: &str, msg_name: &str, sender: &str) -> HashMap<usize, Database> {
        let mut db: Database = Database::default();

        let mut msg: Message = Message::default();
        msg.id_hex = id_hex.to_string();
        msg.name = msg_name.to_string();

        let mut node = Node::default();
        node.name = sender.to_string();
        msg.sender_nodes = vec![node];

        db.messages.push(msg);

        let mut map = HashMap::new();
        map.insert(channel, db);
        map
    }

    // ------------------------
    // Tests
    // ------------------------

    #[test]
    fn parses_without_db_keeps_name_and_sender_empty() {
        // No DB -> name and sender_node should be empty strings
        let db_list: HashMap<usize, Database> = HashMap::new();
        let frame = from_line(line_can_8(), &abs_none(), &db_list).expect("should parse");
        assert_eq!(frame.channel, 1);
        assert_eq!(frame.id, "17334410x");
        assert_eq!(frame.direction, "Rx");
        assert_eq!(frame.protocol, "CAN");
        assert_eq!(frame.byte_length_value, 8);
        assert_eq!(frame.data, "3E 42 03 00 39 00 03 01");
        assert_eq!(frame.name, "");
        assert_eq!(frame.sender_node, "");
        // Timestamp string fixed to 6 decimals
        assert_eq!(frame.timestamp, "0.016728");
        // Fallback absolute time: 0.016728s ≈ 17ms after 00:00:00.000
        assert_eq!(frame.absolute_time, "2025-01-01 00:00:00.017");
    }

    #[test]
    fn decodes_with_db_for_matching_channel() {
        // DB on channel 1, line is channel 1 -> decode name and sender
        let db_list = db_map_with_msg(1, "0x17334410", "OBDC_Funktionaler_Req_All", "ECU");
        let frame: CanFrame = from_line(line_can_8(), &abs_none(), &db_list).expect("should parse");
        assert_eq!(frame.name, "OBDC_Funktionaler_Req_All");
        assert_eq!(frame.sender_node, "ECU");
    }

    #[test]
    fn does_not_decode_if_db_is_on_other_channel() {
        // DB only on channel 2, line is channel 1 -> no decoding
        let db_list = db_map_with_msg(2, "17334410x", "OBDC_Funktionaler_Req_All", "ECU");
        let frame = from_line(line_can_8(), &abs_none(), &db_list).expect("should parse");
        assert_eq!(frame.name, "");
        assert_eq!(frame.sender_node, "");
    }

    #[test]
    fn absolute_time_is_from_start_when_present() {
        // When absolute start time is provided, add the relative seconds to it
        let start = dt(2025, 1, 1, 12, 0, 0, 0); // 12:00:00.000
        let frame = from_line(line_can_8(), &abs_some(start), &HashMap::new()).expect("should parse");
        // 0.016728 s ≈ 17 ms after start
        assert_eq!(frame.absolute_time, "2025-01-01 12:00:00.017");
    }

    #[test]
    fn protocol_is_can_fd_when_length_gt_8() {
        // byte_length = 12 -> "CAN FD"
        let frame = from_line(line_canfd_12(), &abs_none(), &HashMap::new()).expect("should parse");
        assert_eq!(frame.protocol, "CAN FD");
        assert_eq!(frame.byte_length_value, 12);
    }

    #[test]
    fn stops_data_before_length_suffix() {
        // Ensure parser stops data collection at the token "Length"
        let frame = from_line(line_with_length_suffix(), &abs_none(), &HashMap::new()).expect("should parse");
        assert_eq!(frame.data, "01 02 03 04 05 06 07 08");
    }

    #[test]
    fn returns_none_if_first_token_is_not_number() {
        // First token must be a f64, otherwise it's not a valid CAN line
        assert!(from_line("XYZ 1  123  Rx  d 8 00 00 00 00 00 00 00 00", &abs_none(), &HashMap::new()).is_none());
    }
}
