use chrono::{Duration, NaiveDateTime};
use std::collections::HashMap;

use crate::{CanFrame, Database, CanLog};

// Example: 
// 0.016728 1  17334410x       Rx   d 8 3E 42 03 00 39 00 03 01
// 0.016728 1  17334410x       Rx   Name ECU d 8 3E 42 03 00 39 00 03 01
pub(crate) fn parse(line: &str, log: &mut CanLog, db_list: &HashMap<usize, Database>, latesy_by_id_channel: &mut HashMap<(String, usize), CanFrame>) {
    // split line by whitespaces
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 7 {
        return;
    }

    // check timestamp is a valid number
    let timestamp_value: f64 = match parts[0].parse() {
        Ok(value) => value,
        Err(_) => return, // not found or not a valid number
    };

    // absolute time of the single CanFrame
    let absolute_time: String;
    if let Some(start_time) = log.absolute_time.value {
        let seconds: Duration =
            Duration::milliseconds((timestamp_value * 1000.0).round() as i64);
        let abs_time_value: NaiveDateTime = start_time + seconds;
        absolute_time = abs_time_value.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    } else {
        absolute_time = seconds_to_hms_string(timestamp_value);
    }

    let channel: usize = match parts[1].parse::<usize>() {
        Ok(value) => value,
        Err(_) => return, // not a valid number
    };

    let id: String = parts[2].to_string();
    let direction: String = parts[3].to_string();
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
    
    // look for 'd' or 'D' starting from parts[4]
    let d_idx = match parts[4..].iter().position(|p| *p == "d" || *p == "D") {
        Some(off) => 4 + off,
        None => return,
    };

    // part after 'd' is byte lenght
    let byte_length_value: usize = match parts.get(d_idx + 1).and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return,
    };

    // data are from d_ix + 2 until byte_length
    let data_start: usize = d_idx + 2;
    let data_end: usize = data_start + byte_length_value;
    if data_end > parts.len() {
        return; // malformed line: not enough data bytes
    }
    let data: String = parts[data_start..data_end].join(" ");

    let protocol: String = if byte_length_value <= 8 {
        "CAN".to_string()
    } else {
        "CAN FD".to_string()
    };

    let frame: CanFrame = CanFrame {
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
    };

    // fill log.all_frame = Vec<CanFrame>
    // all frames need to be pushd here
    log.all_frame.push(frame.clone());

    // key = (id, channel)
    let key: (String, usize) = (frame.id.clone(), frame.channel.clone());

    // check if key of current CanFrame is already present in HashMap
    // if it is already present, consider only the CanFrame with biggest timestamp
    latesy_by_id_channel
        .entry(key)
        .and_modify(|existing| {
            if frame.timestamp_value > existing.timestamp_value {
                *existing = frame.clone();
            }
        })
        .or_insert(frame.clone());
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
    use chrono::NaiveDateTime;
    use std::collections::HashMap;

    fn empty_db_list() -> HashMap<usize, Database> {
        HashMap::new()
    }

    #[test]
    fn parse_basic_no_ecu_name() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, usize), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.016728 1  17334410x  Rx   d 8 3E 42 03 00 39 00 03 01";
        parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.timestamp, "0.016728");
        assert!((f.timestamp_value - 0.016728).abs() < 1e-12);
        assert_eq!(f.channel, 1);
        assert_eq!(f.id, "17334410x");
        assert_eq!(f.direction, "Rx");
        assert_eq!(f.byte_length_value, 8);
        assert_eq!(f.data, "3E 42 03 00 39 00 03 01");
        assert_eq!(f.protocol, "CAN");
        // 16.728 ms → arrotondato a 17 ms
        assert_eq!(f.absolute_time, "2025-01-01 00:00:00.017");

        let key = ("17334410x".to_string(), 1usize);
        let lf = latest.get(&key).expect("missing latest frame");
        assert_eq!(lf.timestamp_value, f.timestamp_value);
    }

    #[test]
    fn parse_with_ecu_single_word_between_direction_and_d() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.016728 1  17334410x  Rx  Gateway   d 8 3E 42 03 00 39 00 03 01";
        parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.direction, "Rx");
        assert_eq!(f.byte_length_value, 8);
        assert_eq!(f.data, "3E 42 03 00 39 00 03 01");
        assert_eq!(f.protocol, "CAN");
    }

    #[test]
    fn parse_with_ecu_multiword_between_direction_and_d() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.016728 1  17334410x  Rx  Nome ECU   d 8 3E 42 03 00 39 00 03 01";
        parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.direction, "Rx");
        assert_eq!(f.byte_length_value, 8);
        assert_eq!(f.data, "3E 42 03 00 39 00 03 01");
    }

    #[test]
    fn parse_accepts_uppercase_d_marker() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.010000 1  7C1  Rx   D 4 6C 0D 01 00";
        parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.id, "7C1");
        assert_eq!(f.byte_length_value, 4);
        assert_eq!(f.data, "6C 0D 01 00");
        assert_eq!(f.protocol, "CAN");
    }

    #[test]
    fn parse_can_fd_when_length_gt_8() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.030000 1  17334410x  Rx   d 12 11 22 33 44 55 66 77 88 99 AA BB CC";
        parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.byte_length_value, 12);
        assert_eq!(f.protocol, "CAN FD");
        assert_eq!(f.data, "11 22 33 44 55 66 77 88 99 AA BB CC");
    }

    #[test]
    fn parse_ignores_trailing_after_exact_length() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.020000 1  7C1  Rx   d 4 AA BB CC DD Length = 32 anything else";
        parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.byte_length_value, 4);
        assert_eq!(f.data, "AA BB CC DD");
    }

    #[test]
    fn parse_returns_early_if_not_enough_data_bytes() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        // dichiara 6 byte, ma fornisce solo 5 → deve ritornare senza push
        let line = "0.050000 1  7C1  Rx   d 6 01 02 03 04 05";
        parse(line, &mut log, &db_list, &mut latest);

        assert!(log.all_frame.is_empty());
        assert!(latest.is_empty());
    }

    #[test]
    fn keeps_latest_by_id_and_channel() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let l1 = "0.100000 1  7C1  Rx   d 4 01 02 03 04";
        let l2 = "0.200000 1  7C1  Rx   d 4 05 06 07 08";
        parse(l1, &mut log, &db_list, &mut latest);
        parse(l2, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 2);
        let key = ("7C1".to_string(), 1usize);
        let lf = latest.get(&key).expect("missing latest frame");
        assert!((lf.timestamp_value - 0.200000).abs() < 1e-12);
        assert_eq!(lf.data, "05 06 07 08");
        assert_eq!(latest.len(), 1);
    }

    #[test]
    fn absolute_time_when_start_time_is_set() {
        let mut log = CanLog::default();
        log.absolute_time.value = Some(
            NaiveDateTime::parse_from_str("2025-03-10 12:00:00.000", "%Y-%m-%d %H:%M:%S%.3f").unwrap(),
        );

        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "1.500000 1  7C1  Rx   d 4 00 00 00 00";
        parse(line, &mut log, &db_list, &mut latest);

        let f = &log.all_frame[0];
        assert_eq!(f.absolute_time, "2025-03-10 12:00:01.500");
    }

    #[test]
    fn extended_id_uppercase_x_is_supported() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.010000 1  ABCDEF01X  Rx   d 2 00 FF";
        parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.id, "ABCDEF01X");
        assert_eq!(f.byte_length_value, 2);
        assert_eq!(f.data, "00 FF");
    }

    #[test]
    fn direction_tx_is_parsed_and_kept() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.011000 2  1A2B  Tx   d 3 DE AD BE";
        parse(line, &mut log, &db_list, &mut latest);

        let f = &log.all_frame[0];
        assert_eq!(f.channel, 2);
        assert_eq!(f.direction, "Tx");
        assert_eq!(f.byte_length_value, 3);
        assert_eq!(f.data, "DE AD BE");
        assert_eq!(f.protocol, "CAN");
    }

    #[test]
    fn returns_early_on_invalid_timestamp() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "abc 1  7C1  Rx   d 3 01 02 03";
        parse(line, &mut log, &db_list, &mut latest);

        assert!(log.all_frame.is_empty());
        assert!(latest.is_empty());
    }

    #[test]
    fn returns_early_on_invalid_channel() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.010000 x  7C1  Rx   d 3 01 02 03";
        parse(line, &mut log, &db_list, &mut latest);

        assert!(log.all_frame.is_empty());
        assert!(latest.is_empty());
    }

    #[test]
    fn returns_early_when_no_d_marker() {
        let mut log = CanLog::default();
        let mut latest = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.010000 1  7C1  Rx   3 01 02 03"; // manca 'd'/'D'
        parse(line, &mut log, &db_list, &mut latest);

        assert!(log.all_frame.is_empty());
        assert!(latest.is_empty());
    }
}
