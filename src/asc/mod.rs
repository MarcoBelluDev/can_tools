//! # asc
//!
//! `asc` is the module to work with .asc files

pub(crate) mod line;
pub(crate) mod abs_time;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::{Database, CanLog, CanFrame};

/// Parses a Vector ASCII trace (`.asc`) file and builds a `CanLog`.
///
/// The function reads the file **line by line**, discovers an optional absolute-time
/// header (a line starting with `date`), and then parses CAN/CAN-FD frames.
/// Every parsed frame is pushed into `log.all_frame`. In addition, the function
/// keeps, for each unique `(id, channel)` pair, **only the most recent frame**
/// (by `timestamp_value`) and stores those in `log.last_id_chn_frame`.
///
/// Absolute time handling:
/// - The first line matching `abs_time::from_line` is taken as the **start time**.
/// - From that point on, `asc::line::parse` formats each frame’s `absolute_time` as
///   `start + timestamp_value` using `"%Y-%m-%d %H:%M:%S%.3f"`.
///   If no `date` header is found, frames fall back to a synthetic time string
///   derived from the timestamp (see `seconds_to_hms_string`).
///
/// # Parameters
/// - `path`: Path to the `.asc` file. Must end with `.asc`.
/// - `db_list`: Mapping **channel → Database** used to enrich frames (e.g., message
///   name and sender). If a channel has no entry, enrichment is skipped.
///   Extended IDs in traces may end with `x`/`X`; the parser trims that and adds `0x`
///   before calling `Database::get_message_by_id_hex`.
///
/// # Returns
/// - `Ok(CanLog)` on success, where:
///   - `all_frame` contains **all** parsed frames, in file order;
///   - `last_id_chn_frame` contains **one** frame per `(id, channel)`—the one
///     with the greatest `timestamp_value`;
///   - `absolute_time` (in `CanLog`) is set if a `date` header was found, otherwise left at default.
/// - `Err(String)` if the extension is not `.asc` or if the file cannot be opened.
///
/// # Errors
/// - Returns `Err("Not a valid .asc file format")` if `path` does not end with `.asc`.
/// - Returns `Err("Error while opening .asc file: ...")` on I/O errors.
///
/// # Behavior & Invariants
/// - Only the **first** valid `date` header is used; subsequent lines are treated as data.
/// - Frame parsing is delegated to `asc::line::parse`, which infers protocol
///   (`"CAN"` vs `"CAN FD"`) from payload length.
/// - The `(id, channel)` key uses the raw `id` string from the log (which may
///   include an `'x'`/`'X'` suffix for extended identifiers) and `channel` as `usize`.
/// - Lines may contain optional ECU tokens between `direction` and the `d` marker; the
///   line parser locates `d` dynamically and reads exactly `length` data bytes.
///
/// # Complexity
/// - Time: O(N) over the number of lines (single pass).
/// - Space: O(U) for the number of unique `(id, channel)` pairs.
///
/// # Example
/// ```rust no_run
/// use std::collections::HashMap;
/// use can_tools::{asc, Database};
///
/// let db_by_channel: HashMap<u8, Database> = HashMap::new();
/// let log = asc::parse_from_file("trace.asc", &db_by_channel).expect("parse failed");
/// println!("Total frames: {}", log.all_frame.len());
/// println!("Unique id/channel last frames: {}", log.last_id_chn_frame.len());
/// ```
///
/// # Notes
/// - Lines are streamed with `BufRead::lines()`. Non-frame lines are ignored unless
///   they match the `date` header format handled by `abs_time::from_line`.

pub fn parse_from_file(path: &str, db_list: &HashMap<u8, Database>) -> Result<CanLog, String> {
    // check if provided file has .asc format
    if !path.ends_with(".asc") {
        return Err(format!("Not a valid .asc file format"));
    }

    // initialize canlog
    let mut log: CanLog = CanLog::default();
    let mut found_abs_time: bool = false;

    let reader: BufReader<File>;
    // open trace and create reader
    match File::open(path) {
        Ok(file) => {
            reader = BufReader::new(file);
        }, 
        Err(e) => {
            return Err(format!{"Error while opening .asc file: {}", e});
        },
    }

    // temporary map: (id, channel) → last CanFrame per channel and id
    let mut latest_by_id_channel: HashMap<(String, u8), CanFrame> = HashMap::new();

    // read .asc file line by line
    for line in reader.lines().map_while(Result::ok) {
        if !found_abs_time {
            if let Some(time) = abs_time::from_line(&line) {
                log.absolute_time = time;
                found_abs_time = true;
                continue; // skip abs_time check for rest of the line
            }
        }
        line::parse(&line, &mut log, db_list, &mut latest_by_id_channel);
    }

    // convert HashMap into the Vec<CanFrame> with only last Frame per id/channel combination
    log.last_id_chn_frame = latest_by_id_channel.into_values().collect();

    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use std::collections::HashMap;

    fn empty_db_list() -> HashMap<u8, Database> {
        HashMap::new()
    }

    #[test]
    fn parse_basic_no_ecu_name() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.016728 1  17334410x  Rx   d 8 3E 42 03 00 39 00 03 01";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.timestamp, "0.016728");
        assert!((f.timestamp_value - 0.016728).abs() < 1e-9);
        assert_eq!(f.channel, 1);
        assert_eq!(f.id, "17334410x");
        assert_eq!(f.direction, "Rx");
        assert_eq!(f.byte_length_value, 8);
        assert_eq!(f.data, "3E 42 03 00 39 00 03 01");
        assert_eq!(f.protocol, "CAN");
        // seconds_to_hms_string arrotonda a millisecondo
        assert_eq!(f.absolute_time, "2025-01-01 00:00:00.017");

        let key: (String, u8) = ("17334410x".to_string(), 1u8);
        let lf = latest.get(&key).expect("missing latest frame");
        assert_eq!(lf.timestamp_value, f.timestamp_value);
    }

    #[test]
    fn parse_with_ecu_single_word_between_direction_and_d() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.016728 1  17334410x  Rx  Gateway   d 8 3E 42 03 00 39 00 03 01";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

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
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.016728 1  17334410x  Rx  Nome ECU   d 8 3E 42 03 00 39 00 03 01";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.direction, "Rx");
        assert_eq!(f.byte_length_value, 8);
        assert_eq!(f.data, "3E 42 03 00 39 00 03 01");
    }

    #[test]
    fn parse_accepts_uppercase_d_marker() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.010000 1  7C1  Rx   D 4 6C 0D 01 00";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

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
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.030000 1  17334410x  Rx   d 12 11 22 33 44 55 66 77 88 99 AA BB CC";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.byte_length_value, 12);
        assert_eq!(f.protocol, "CAN FD");
        assert_eq!(f.data, "11 22 33 44 55 66 77 88 99 AA BB CC");
    }

    #[test]
    fn parse_ignores_trailing_after_exact_length() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.020000 1  7C1  Rx   d 4 AA BB CC DD Length = 32 anything else";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.byte_length_value, 4);
        assert_eq!(f.data, "AA BB CC DD");
    }

    #[test]
    fn parse_returns_early_if_not_enough_data_bytes() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        // dichiara 6 byte, ma fornisce solo 5
        let line = "0.050000 1  7C1  Rx   d 6 01 02 03 04 05";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        // nessun frame aggiunto
        assert!(log.all_frame.is_empty());
        assert!(latest.is_empty());
    }

    #[test]
    fn keeps_latest_by_id_and_channel() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let l1 = "0.100000 1  7C1  Rx   d 4 01 02 03 04";
        let l2 = "0.200000 1  7C1  Rx   d 4 05 06 07 08";
        crate::asc::line::parse(l1, &mut log, &db_list, &mut latest);
        crate::asc::line::parse(l2, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 2);
        let key: (String, u8) = ("7C1".to_string(), 1u8);
        let lf = latest.get(&key).expect("missing latest frame");
        assert!((lf.timestamp_value - 0.200000).abs() < 1e-9);
        assert_eq!(lf.data, "05 06 07 08");
        assert_eq!(latest.len(), 1);
    }

    #[test]
    fn absolute_time_when_start_time_is_set() {
        let mut log = CanLog::default();
        log.absolute_time.value =
            Some(NaiveDateTime::parse_from_str("2025-03-10 12:00:00.000", "%Y-%m-%d %H:%M:%S%.3f").unwrap());

        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "1.500000 1  7C1  Rx   d 4 00 00 00 00";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        let f = &log.all_frame[0];
        assert_eq!(f.absolute_time, "2025-03-10 12:00:01.500");
    }

    #[test]
    fn extended_id_uppercase_x_is_supported() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.010000 1  ABCDEF01X  Rx   d 2 00 FF";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        assert_eq!(log.all_frame.len(), 1);
        let f = &log.all_frame[0];
        assert_eq!(f.id, "ABCDEF01X");
        assert_eq!(f.byte_length_value, 2);
        assert_eq!(f.data, "00 FF");
    }

    #[test]
    fn direction_tx_is_parsed_and_kept() {
        let mut log = CanLog::default();
        let mut latest: HashMap<(String, u8), CanFrame> = HashMap::new();
        let db_list = empty_db_list();

        let line = "0.011000 2  1A2B  Tx   d 3 DE AD BE";
        crate::asc::line::parse(line, &mut log, &db_list, &mut latest);

        let f = &log.all_frame[0];
        assert_eq!(f.channel, 2);
        assert_eq!(f.direction, "Tx");
        assert_eq!(f.byte_length_value, 3);
        assert_eq!(f.data, "DE AD BE");
    }
}
