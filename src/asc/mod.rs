//! # asc
//!
//! `asc` is the module to work with .asc files

pub(crate) mod frame_parse;
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
/// - From that point on, `frame::from_line` formats `absolute_time` as
///   `start + timestamp_value` using `"%Y-%m-%d %H:%M:%S%.3f"`.
///
/// # Parameters
/// - `path`: Path to the `.asc` file. The function requires it to end with `.asc`.
///
/// # Returns
/// - `Ok(CanLog)` on success, where:
///   - `all_frame` contains **all** parsed frames, in file order;
///   - `last_id_chn_frame` contains **one** frame per `(id, channel)`—the one
///     with the greatest `timestamp_value`;
///   - `absolute_time` is set if a `date` header was found, otherwise left at default.
/// - `Err(String)` if the extension is not `.asc` or if the file cannot be opened.
///
/// # Errors
/// - Returns `Err("Not a valid .asc file format")` if `path` does not end with `.asc`.
/// - Returns `Err("Error while opening .asc file: ...")` on I/O errors.
///
/// # Behavior & Invariants
/// - Only the **first** valid `date` header is used; subsequent lines are treated as data.
/// - Frame parsing is delegated to `asc::frame::from_line`, which infers protocol
///   (`"CAN"` vs `"CAN FD"`) from payload length.
/// - The `(id, channel)` key uses the raw `id` string as found in the log (e.g. it may
///   include suffixes like `'x'` for extended identifiers) and `channel` as `usize`.
///
/// # Complexity
/// - Time: O(N) over the number of lines (single pass).
/// - Space: O(U) for the number of unique `(id, channel)` pairs.
///
/// # Example
/// ```text
/// let log = can_tools::asc::parse_from_file("trace.asc")?;
/// println!("Total frames: {}", log.all_frame.len());
/// println!("Unique id/channel last frames: {}", log.last_id_chn_frame.len());
/// ```
///
/// # Notes
/// - Lines are streamed with `BufRead::lines()`. Non-frame lines are ignored unless
///   they match the `date` header format handled by `abs_time::from_line`.
pub fn parse_from_file(path: &str, db_list: &HashMap<usize, Database>) -> Result<CanLog, String> {
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
    let mut latest_by_id_channel: HashMap<(String, usize), CanFrame> = HashMap::new();

    // read .asc file line by line
    for line in reader.lines().map_while(Result::ok) {
        if !found_abs_time {
            if let Some(time) = abs_time::from_line(&line) {
                log.absolute_time = time;
                found_abs_time = true;
                continue; // skip abs_time check for rest of the line
            }
        }
        if let Some(frame) = frame_parse::from_line(&line, &log.absolute_time, db_list) {
            // all CanFrame needs to be pushed in this vector
            log.all_frame.push(frame.clone());

            // key = (id, channel)
            let key: (String, usize) = (frame.id.clone(), frame.channel.clone());

            // check if key of current CanFrame is already present in HashMap
            // if it is already present, consider only the CanFrame with biggest timestamp
            latest_by_id_channel
                .entry(key)
                .and_modify(|existing| {
                    if frame.timestamp_value > existing.timestamp_value {
                        *existing = frame.clone();
                    }
                })
                .or_insert(frame.clone());
        }
    }

    // convert HashMap into the Vec<CanFrame> with only last Frame per id/channel combination
    log.last_id_chn_frame = latest_by_id_channel.into_values().collect();

    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Pull in crate-level types the parser fills.
    use crate::Database;
    use crate::types::canframe::CanFrame;
    use crate::types::canlog::CanLog;

    // If your Message/Node types are re-exported, these will work.
    // Otherwise, adjust the paths (e.g., crate::types::message::Message).
    use crate::Message;
    use crate::Node;

    // ----------------------------------
    // Helpers
    // ----------------------------------

    fn tmp_path_with_name(name: &str) -> PathBuf {
        // Create a (mostly) unique filename under the OS temp dir
        let mut p = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        p.push(format!("{}_{}.asc", name, nanos));
        p
    }

    fn write_asc(contents: &str) -> PathBuf {
        let path = tmp_path_with_name("trace");
        let mut f = File::create(&path).expect("create temp asc");
        f.write_all(contents.as_bytes()).expect("write temp asc");
        path
    }

    fn empty_dbmap() -> HashMap<usize, Database> {
        HashMap::new()
    }

    // Build a HashMap<channel, Database> containing a minimal message matching (id_hex, msg_name, sender)
    fn db_map_with_msg(channel: usize, id_hex: &str, msg_name: &str, sender: &str) -> HashMap<usize, Database> {
        let mut db = Database::default();

        let mut msg = Message::default();
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

    fn get_last_by_id_channel<'a>(log: &'a CanLog, id: &str, chn: usize) -> Option<&'a CanFrame> {
        log.last_id_chn_frame.iter().find(|f| f.id == id && f.channel == chn)
    }

    // ----------------------------------
    // Tests
    // ----------------------------------

    #[test]
    fn error_if_not_asc_extension() {
        let res = parse_from_file("whatever.txt", &empty_dbmap());
        assert!(res.is_err());
    }

    #[test]
    fn error_if_cannot_open_file() {
        let res = parse_from_file("definitely-does-not-exist/foobar.asc", &empty_dbmap());
        assert!(res.is_err());
    }

    #[test]
    fn parses_without_date_header_uses_fallback_absolute_time() {
        // No "date ..." line -> absolute_time should use fallback formatter
        let asc = r#"
base hex  timestamps absolute
internal events logged
0.016728 1  17334410x       Rx   d 8 3E 42 03 00 39 00 03 01
"#;
        let path = write_asc(asc);
        let log = parse_from_file(path.to_str().unwrap(), &empty_dbmap()).expect("parse ok");

        // One frame parsed
        assert_eq!(log.all_frame.len(), 1);
        assert_eq!(log.last_id_chn_frame.len(), 1);

        let f = &log.all_frame[0];
        assert_eq!(f.timestamp, "0.016728");
        assert_eq!(f.protocol, "CAN");
        assert_eq!(f.byte_length_value, 8);
        assert_eq!(f.data, "3E 42 03 00 39 00 03 01");
        // Fallback absolute time (rounded to 17 ms)
        assert_eq!(f.absolute_time, "2025-01-01 00:00:00.017");

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_with_date_header_uses_first_date_and_keeps_latest_per_id_channel() {
        // Multiple frames for same (id, channel); the latest timestamp wins in last_id_chn_frame.
        // Also ensure parser stops data at "Length =".
        let asc = r#"
date Fri May 12 04:16:06.532 pm 2023
base hex  timestamps absolute
internal events logged
0.000000 1  100             Rx   d 8 01 02 03 04 05 06 07 08
0.010000 1  100             Rx   d 8 11 12 13 14 15 16 17 18
0.020000 1  100             Rx   d 8 21 22 23 24 25 26 27 28 Length = 8
0.005000 2  200x            Tx   d 4 AA BB CC DD
some junk line that should be ignored
"#;
        let path = write_asc(asc);
        let log = parse_from_file(path.to_str().unwrap(), &empty_dbmap()).expect("parse ok");

        // All parsed frames (4 valid frame lines)
        assert_eq!(log.all_frame.len(), 4);

        // Latest-per-(id, channel): we expect 2 entries -> (100,1) and (200x,2)
        assert_eq!(log.last_id_chn_frame.len(), 2);

        // (100,1) must be the one at 0.020000, and data must stop before "Length"
        let f_100_ch1 = get_last_by_id_channel(&log, "100", 1).expect("missing (100,1)");
        assert_eq!(f_100_ch1.timestamp, "0.020000");
        assert_eq!(f_100_ch1.data, "21 22 23 24 25 26 27 28");
        // Absolute time = 2023-05-12 16:16:06.532 + 20ms = .552
        assert_eq!(f_100_ch1.absolute_time, "2023-05-12 16:16:06.552");

        // (200x,2) only appears once at 0.005000
        let f_200x_ch2 = get_last_by_id_channel(&log, "200x", 2).expect("missing (200x,2)");
        assert_eq!(f_200x_ch2.timestamp, "0.005000");
        assert_eq!(f_200x_ch2.data, "AA BB CC DD");

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn uses_per_channel_database_for_name_and_sender() {
        // Only channel 1 has a DB with id "100"
        let db_list = db_map_with_msg(1, "0x100", "MSG100", "ECU1");

        let asc = r#"
date Fri May 12 04:16:06.532 pm 2023
0.000000 1  100             Rx   d 8 01 02 03 04 05 06 07 08
0.005000 2  200x            Tx   d 4 AA BB CC DD
"#;

        let path = write_asc(asc);
        let log = parse_from_file(path.to_str().unwrap(), &db_list).expect("parse ok");

        // Frame on channel 1, id "100" -> should be decoded
        let f_100_ch1 = get_last_by_id_channel(&log, "100", 1).expect("missing (100,1)");
        assert_eq!(f_100_ch1.name, "MSG100");
        assert_eq!(f_100_ch1.sender_node, "ECU1");

        // Frame on channel 2, no DB for ch2 -> name/sender should be empty
        let f_200x_ch2 = get_last_by_id_channel(&log, "200x", 2).expect("missing (200x,2)");
        assert_eq!(f_200x_ch2.name, "");
        assert_eq!(f_200x_ch2.sender_node, "");

        // Cleanup
        let _ = fs::remove_file(path);
    }

    #[test]
    fn ignores_non_frame_lines_and_secondary_date_headers() {
        // Only the first valid "date ..." line matters. Later "date" lines should be ignored.
        let asc = r#"
date Fri May 12 04:16:06.000 pm 2023
this line should be ignored
0.001000 1  123             Rx   d 1 01
date Mon Jan 01 01:00:00.000 am 2024
0.002000 1  123             Rx   d 1 02
"#;
        let path = write_asc(asc);
        let log = parse_from_file(path.to_str().unwrap(), &empty_dbmap()).expect("parse ok");

        assert_eq!(log.all_frame.len(), 2);
        // Latest is 0.002000
        let f = get_last_by_id_channel(&log, "123", 1).expect("missing (123,1)");
        assert_eq!(f.timestamp, "0.002000");
        // Absolute time should be based on the FIRST date header (May 12, 2023 16:16:06.000)
        // 2 ms after -> .002
        assert_eq!(f.absolute_time, "2023-05-12 16:16:06.002");

        // Cleanup
        let _ = fs::remove_file(path);
    }
}
