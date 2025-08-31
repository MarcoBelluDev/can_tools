use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::asc::support;
use crate::{CanLog, Database};

/// Parses a Vector ASCII trace (`.asc`) file and builds a `CanLog`.
///
/// The function reads the file **line by line**, discovers an optional absolute-time
/// header (a line starting with `date`), and then parses CAN/CAN-FD frames.
/// Every parsed frame is pushed into `log.all_frame`. In addition, the function
/// keeps, for each unique `(numeric id, channel)` pair, **only the index of the most recent frame**
/// (by `timestamp` seconds) and stores those indices in `log.last_id_chn_frame`.
///
/// Absolute time handling:
/// - The first line matching `abs_time::from_line` is taken as the **start time**.
/// - From that point on, `asc::line::parse` formats each frame’s `absolute_time` as
///   `start + timestamp` using `"%Y-%m-%d %H:%M:%S%.3f"`.
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
/// - `all_frame` contains **all** parsed frames, in file order;
/// - `last_id_chn_frame` contains **one** frame per `(numeric id, channel)`—the one
///   with the greatest `timestamp`;
/// - `absolute_time` (in `CanLog`) is set if a `date` header was found, otherwise left at default.
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
/// - The `(numeric id, channel)` key uses the raw `id` string from the log (which may
///   include an `'x'`/`'X'` suffix for extended identifiers) and `channel` as `usize`.
/// - Lines may contain optional ECU tokens between `direction` and the `d` marker; the
///   line parser locates `d` dynamically and reads exactly `length` data bytes.
///
/// # Complexity
/// - Time: O(N) over the number of lines (single pass).
/// - Space: O(U) for the number of unique `(numeric id, channel)` pairs.
///
///
/// # Notes
/// - Lines are streamed with `BufRead::lines()`. Non-frame lines are ignored unless
///   they match the `date` header format handled by `abs_time::from_line`.
pub fn from_file(path: &str, db_list: &HashMap<u8, Database>) -> Result<CanLog, String> {
    // check if provided file has .asc format
    if !path.ends_with(".asc") {
        return Err("Not a valid .asc file format".to_string());
    }

    // initialize canlog and all the helper needed for its internal fields
    let mut log: CanLog = CanLog::default();
    // temporary map: (numeric id, channel) → last frame index per channel and id
    let mut latest_by_id_channel: HashMap<(u32, u8), usize> = HashMap::new();
    // temporary registry: (name, channel) -> Signal index
    let mut chart_by_key: HashMap<String, usize> = HashMap::new();
    let mut found_abs_time: bool = false;

    let reader: BufReader<File> = match File::open(path) {
        Ok(file) => BufReader::new(file),
        Err(e) => {
            return Err(format! {"Error while opening .asc file: {}", e});
        }
    };

    // read .asc file line by line
    for line in reader.lines().map_while(Result::ok) {
        if !found_abs_time && let Some(time) = support::abs_time::from_line(&line) {
            log.absolute_time = time;
            found_abs_time = true;
            continue; // skip abs_time check for rest of the line
        }
        support::line::parse(
            &line,
            &mut log,
            db_list,
            &mut latest_by_id_channel,
            &mut chart_by_key,
        );
    }

    // convert HashMap into the Vec<CanFrame> with only last Frame per id/channel combination
    log.last_id_chn_frame = latest_by_id_channel.into_values().collect();

    Ok(log)
}
