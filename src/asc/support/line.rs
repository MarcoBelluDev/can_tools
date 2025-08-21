use chrono::{Duration, NaiveDateTime};
use std::collections::HashMap;

use crate::{CanFrame, CanLog, Database, MessageLog, SignalLog};

// Example:
// 0.016728 1 17334410x Rx d 8 3E 42 03 00 39 00 03 01
// 0.016728 1 17334410x Rx Name ECU d 8 3E 42 03 00 39 00 03 01
pub(crate) fn parse(
    line: &str,
    log: &mut CanLog,
    db_list: &HashMap<u8, Database>,
    latesy_by_id_channel: &mut HashMap<(u32, u8), usize>,
    chart_by_key: &mut HashMap<String, usize>,
) {
    // split line by whitespaces (ASCII only, faster than Unicode-aware split)
    let mut it = line.split_ascii_whitespace();

    // 0) timestamp
    let ts_tok = match it.next() {
        Some(v) => v,
        None => return,
    };
    let timestamp: f32 = match ts_tok.parse() {
        Ok(v) => v,
        Err(_) => return,
    };

    // 1) channel
    let ch_tok = match it.next() {
        Some(v) => v,
        None => return,
    };
    let channel: u8 = match ch_tok.parse::<u8>() {
        Ok(v) => v,
        Err(_) => return,
    };

    // 2) id token (keep original string for the frame)
    let id_tok = match it.next() {
        Some(v) => v,
        None => return,
    };
    let id: String = id_tok.to_string();

    // 3) direction
    let direction = match it.next() {
        Some(v) => v.to_string(),
        None => return,
    };

    // 4) scan forward to 'd' or 'D', then read byte length and payload tokens
    let mut after_d = None;
    while let Some(tok) = it.next() {
        if tok == "d" || tok == "D" {
            after_d = it.next(); // next is byte length
            break;
        }
    }
    let byte_length: u16 = match after_d.and_then(|s| s.parse().ok()) {
        Some(v) => v,
        None => return,
    };

    // Collect N payload tokens into a single space-separated String (no intermediate Vec)
    let mut data = String::with_capacity(byte_length as usize * 3);
    for i in 0..byte_length as usize {
        match it.next() {
            Some(tok) => {
                if i != 0 {
                    data.push(' ');
                }
                data.push_str(tok);
            }
            None => return, // malformed: not enough data bytes
        }
    }

    // absolute time of the single CanFrame
    let absolute_time: String;
    if let Some(start_time) = log.absolute_time.value {
        let seconds: Duration = Duration::milliseconds((timestamp * 1000.0).round() as i64);
        let abs_time_value: NaiveDateTime = start_time + seconds;
        absolute_time = abs_time_value.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    } else {
        absolute_time = seconds_to_hms_string(timestamp);
    }

    let mut name: String = String::new();
    let mut sender_node: String = String::new();
    let mut comment: String = String::new();

    // Protocol
    let protocol: String = if byte_length <= 8 {
        "CAN".to_string()
    } else {
        "CAN FD".to_string()
    };

    // Vector of decoded signals (if DB is available)
    let mut msg_signal_indices: Vec<usize> = Vec::new();

    // If a DBC is available for this channel, try to decode
    if let Some(dbc) = db_list.get(&channel) {
        if let Some(msg) = dbc.get_message_by_id_hex(&canonicalize_id(&id)) {
            name = msg.name.clone();
            if let Some(&node_rif) = msg.sender_nodes.first() {
                if let Some(node) = dbc.get_node_by_key(node_rif) {
                    sender_node = node.name.clone();
                }
            }
            comment = msg.comment.clone();

            // Parse payload bytes
            let payload_bytes: Vec<u8> = parse_hex_bytes(&data);

            msg_signal_indices.reserve(msg.signals.len());
            for &sig_key in &msg.signals {
                if let Some(s) = dbc.get_sig_by_key(sig_key) {
                    let raw: i64 = s.extract_raw_i64(&payload_bytes);
                    let sigf: SignalLog = s.to_sigframe(raw);

                    // Append a point to the corresponding SignalLog time series
                    let t: f32 = timestamp;
                    let key: String = build_sig_key(channel, &id, &s.name);
                    let idx = *chart_by_key.entry(key.clone()).or_insert_with(|| {
                        let i = log.signals.len();
                        let unit_norm = s
                            .unit_of_measurement
                            .strip_prefix("Unit_")
                            .unwrap_or(&s.unit_of_measurement)
                            .to_string();
                        log.signals.push(SignalLog {
                            message: 0, // set below
                            name: s.name.clone(),
                            factor: s.factor,
                            offset: s.offset,
                            channel,
                            raw: sigf.raw,
                            value: sigf.value,
                            unit: unit_norm,
                            text: sigf.text.clone(),
                            comment: s.comment.clone(),
                            value_table: s.value_table.clone(),
                            values: Vec::new(),
                        });
                        i
                    });
                    log.signals[idx].raw = sigf.raw;
                    log.signals[idx].value = sigf.value;
                    log.signals[idx].text = sigf.text.clone();
                    log.signals[idx].values.push([t.into(), sigf.value]);

                    msg_signal_indices.push(idx);
                }
            }
        }
    }

    // Build the MessageLog for this frame
    let message = MessageLog {
        channel,
        byte_length,
        protocol,
        id,
        name,
        sender_node,
        data,
        comment,
        signals: msg_signal_indices,
    };

    // Push message and get its index
    log.messages.push(message);
    let message_idx: usize = log.messages.len() - 1;

    // point each SignalLog to this message as last updater
    for &sidx in &log.messages[message_idx].signals {
        if let Some(s) = log.signals.get_mut(sidx) {
            s.message = message_idx;
        }
    }

    // Build the frame referencing the message
    let frame: CanFrame = CanFrame {
        absolute_time,
        timestamp, // f32
        channel,   // u8
        direction, // String
        message: message_idx,
    };

    // push frame
    log.can_frames.push(frame);

    // index of the frame we just pushed
    let idx: usize = log.can_frames.len() - 1;

    // key = (numeric id, channel) from the message
    let id_num: u32 = match parse_id_u32(&log.messages[message_idx].id) {
        Some(v) => v,
        None => return,
    };
    let key: (u32, u8) = (id_num, log.can_frames[idx].channel);

    // Update: keep largest timestamp per (id, channel)
    match latesy_by_id_channel.get_mut(&key) {
        Some(last_idx) => {
            if log.can_frames[idx].timestamp > log.can_frames[*last_idx].timestamp {
                *last_idx = idx;
            }
        }
        None => {
            latesy_by_id_channel.insert(key, idx);
        }
    }
    latesy_by_id_channel
        .entry(key)
        .and_modify(|existing_idx| {
            let existing_ts = log.can_frames[*existing_idx].timestamp;
            let new_ts = log.can_frames[idx].timestamp;
            if new_ts > existing_ts {
                *existing_idx = idx;
            }
        })
        .or_insert(idx);
}

fn seconds_to_hms_string(seconds: f32) -> String {
    let total_millis: u32 = (seconds * 1000.0).round() as u32;

    let hours: u32 = total_millis / 3_600_000;
    let minutes: u32 = (total_millis % 3_600_000) / 60_000;
    let secs: u32 = (total_millis % 60_000) / 1000;
    let millis: u32 = total_millis % 1000;

    format!(
        "2025-01-01 {:02}:{:02}:{:02}.{:03}",
        hours, minutes, secs, millis
    )
}

/// Turn "3E 42 03 00 39 00 03 01" into Vec<u8>.
pub(crate) fn parse_hex_bytes(data: &str) -> Vec<u8> {
    data.split_ascii_whitespace()
        .filter_map(|b| u8::from_str_radix(b, 16).ok())
        .collect()
}

fn build_sig_key(channel: u8, id_token: &str, sig_name: &str) -> String {
    let id: String = canonicalize_id(id_token);
    // Keep it simple and stable; not shown to users, only for the map:
    format!("{}|{}|{}", channel, id, sig_name)
}

/// Normalize an ASC id token like "17334410x" or "12AB" to "0x17334410" / "0x12AB".
fn parse_id_u32(id_token: &str) -> Option<u32> {
    // strip trailing x/X (extended id marker)
    let s = id_token.trim_end_matches(['x', 'X']);
    let s = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        hex
    } else {
        s
    };
    u32::from_str_radix(s, 16).ok()
}

fn canonicalize_id(id_token: &str) -> String {
    // strip trailing x/X (extended id marker) and uppercase for stability
    let id_no_x: String = id_token.trim_end_matches(['x', 'X']).to_uppercase();

    // ensure "0x" prefix (ASC ids usually don't have it)
    if id_no_x.starts_with("0X") || id_no_x.starts_with("0x") {
        id_no_x
    } else {
        format!("0x{}", id_no_x)
    }
}
