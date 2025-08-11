use crate::{AbsoluteTime, CanFrame, SigLog};

/// Aggregates CAN/CAN-FD frames parsed from a Vector ASCII trace (`.asc`) file.
///
/// A typical `.asc` file starts with an optional absolute time header, followed by
/// configuration lines and then the frames, for example:
///
/// ```text
/// date Fri May 12 04:16:06.532 pm 2023
/// base hex  timestamps absolute
/// internal events logged
/// Begin TriggerBlock
///    0.016728 1  17334410x       Rx   d 8 3E 42 03 00 39 00 03 01
///    0.020212 1  7C1             Tx   d 4 6C 0D 01 00
///    0.020421 2  7C1             Rx   d 4 6C 0D 01 00
///    0.026958 1  17334410x       Tx   d 8 3D DA 00 00 00 00 00 00
///    0.029046 2  3D0             Rx   d 8 00 00 00 00 00 00 00 00
/// End TriggerBlock
/// ```
///
/// Use [`asc::parse_from_file`](crate::asc::parse_from_file) to build a `CanLog`
/// from disk. That function fills:
///
/// * [`all_frame`](Self::all_frame) with **every** parsed frame (in file order), and
/// * [`last_id_chn_frame`](Self::last_id_chn_frame) with **one** frame per
///   `(id, channel)` pair — specifically, the one with the greatest
///   `timestamp_value`.
///
/// # Fields
///
/// - [`absolute_time`](Self::absolute_time):
///   The absolute start time parsed from the line starting with `date` (if present).
///   When set, per-frame `absolute_time` strings are computed as
///   `start + timestamp_value` formatted as `"%Y-%m-%d %H:%M:%S%.3f"`.
///
/// - [`all_frame`](Self::all_frame):
///   Flat list of all frames exactly as parsed from the trace. Useful for
///   replay/inspection and time-series processing.
///
/// - [`last_id_chn_frame`](Self::last_id_chn_frame):
///   Deduplicated view keeping only the **latest** frame for each `(id, channel)`
///   combination. This is convenient to show the most recent state of each message
///   on each channel without scanning the entire log. Order is not guaranteed.
///
/// # Example
/// ```no_run
/// use can_tools::asc;
/// use std::collections::HashMap;
///
/// // Parse an .asc file into a CanLog
/// let log = asc::parse_from_file("path/to/trace.asc", &HashMap::new())?;
/// println!("Frames total: {}", log.all_frame.len());
/// println!("Unique (id,channel): {}", log.last_id_chn_frame.len());
/// # Ok::<_, String>(())
/// ```
///
/// # Notes
/// - Message IDs are kept as raw strings as they appear in the trace (e.g. an
///   extended identifier may be logged with a trailing `x`).
/// - The library infers `"CAN"` vs `"CAN FD"` from payload length when constructing
///   frames.
/// - Non-frame lines are ignored except for the first valid `date` header, which
///   initializes [`absolute_time`](Self::absolute_time).
#[derive(Clone, Default)]
pub struct CanLog {
    /// Absolute start time extracted from the `date` header, if present.
    pub absolute_time: AbsoluteTime,

    /// All parsed frames in file order.
    pub all_frame: Vec<CanFrame>,

    /// One frame per `(id, channel)` — the most recent by `timestamp_value`.
    pub last_id_chn_frame: Vec<CanFrame>,

    // detailed signal list
    pub sig_list: Vec<SigLog>,
}

impl CanLog {
    /// Resets the log to its default (empty) state.
    ///
    /// This clears the absolute start time and empties both frame vectors.
    ///
    /// # Example
    /// ```no_run
    /// use can_tools::asc;
    /// use std::collections::HashMap;
    ///
    /// let mut log = asc::parse_from_file("trace.asc", &HashMap::new())?;
    /// log.clear();
    /// assert!(log.all_frame.is_empty());
    /// assert!(log.last_id_chn_frame.is_empty());
    /// # Ok::<_, String>(())
    /// ```
    pub fn clear(&mut self) {
        self.absolute_time.clear();
        self.all_frame = Vec::default();
        self.last_id_chn_frame = Vec::default();
        self.sig_list = Vec::default();
    }
}