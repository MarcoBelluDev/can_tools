/// Represents a single CAN (or CAN FD) frame parsed from a `.asc` log line.
///
/// A typical row in the source log has the shape:
///
/// ```text
/// <timestamp> <channel> <id> <direction> d <length> <data...>
/// e.g.:
/// 0.016728 1  17334410x       Rx   d 8 3E 42 03 00 39 00 03 01
/// ```
///
/// This struct keeps both the raw textual tokens (e.g. `timestamp`, `byte_length`, `data`)
/// and the normalized numeric values (e.g. `timestamp_value`, `byte_length_value`) for
/// convenience.
///
/// # Field semantics
///
/// - `absolute_time`:
///   Human-readable absolute timestamp. When an absolute start time is known (parsed
///   from the log header), this is `start + timestamp_value` formatted as
///   `"%Y-%m-%d %H:%M:%S%.3f"`. If no start time is known, it may be derived from a
///   fixed reference date by the parser.
///
/// - `timestamp` / `timestamp_value`:
///   Relative capture time since trace start, as string (usually with 6 decimals) and
///   as `f64` seconds respectively. These two fields represent the same concept in
///   different forms and should stay consistent.
///
/// - `channel`:
///   Bus/channel index as reported by the logger (typically 1-based).
///
/// - `protocol`:
///   Protocol label inferred from payload length; `"CAN"` for payloads up to 8 bytes,
///   `"CAN FD"` for larger payloads.
///
/// - `id`:
///   Message identifier as it appears in the log. Depending on the logger, it may
///   include suffixes (e.g. an `'x'` to denote an extended identifier).
///
/// - `name` / `sender_node`:
///   Optional metadata if available from the database or the log; empty when unknown.
///
/// - `direction`:
///   Direction of the frame as seen by the logger, typically `"Rx"` or `"Tx"`.
///
/// - `byte_length` / `byte_length_value`:
///   Payload length, kept both as raw token and as parsed `usize`.
///
/// - `data`:
///   The payload bytes formatted as hex pairs separated by spaces (e.g. `"3E 42 03 00 …"`).
///
/// # Invariants
///
/// * `protocol == "CAN"` iff `byte_length_value <= 8`, otherwise `"CAN FD"`.
/// * `timestamp` and `timestamp_value` represent the same instant (string vs numeric).
/// * When an absolute start time is present, `absolute_time` equals
///   `start + Duration::from_millis(round(timestamp_value * 1000))`.
///
/// # Examples
///
/// Create and reset a frame:
///
/// ```rust
/// # use can_tools::types::canframe::CanFrame;
/// let mut f = CanFrame::default();
/// f.channel = 1;
/// f.protocol = "CAN".into();
/// f.byte_length_value = 8;
/// f.data = "00 11 22 33 44 55 66 77".into();
///
/// // Clear back to defaults
/// f.clear();
/// assert_eq!(f, CanFrame::default());
/// ```
///
/// Parsing from a log line (simplified):
///
/// ```text
/// let start_abs_time = AbsoluteTime { ... };
/// let line = "0.016728 1  17334410x       Rx   d 8 3E 42 03 00 39 00 03 01";
/// let frame = asc::frame::from_line(line, &start_abs_time).unwrap();
/// assert_eq!(frame.protocol, "CAN");
/// assert_eq!(frame.byte_length_value, 8);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CanFrame {
    /// Absolute timestamp in `%Y-%m-%d %H:%M:%S%.3f` when available,
    /// otherwise derived by the parser.
    pub absolute_time: String,

    /// Relative timestamp as it appeared in the log (e.g. `"0.016728"`).
    pub timestamp: String,

    /// Relative timestamp in seconds since trace start.
    pub timestamp_value: f32,

    /// Logger channel index (typically 1-based).
    pub channel: u8,

    /// `"CAN"` or `"CAN FD"` depending on payload size.
    pub protocol: String,

    /// Raw identifier token as seen in the log (may include format suffixes).
    pub id: String,

    /// Optional message name (empty if unknown).
    pub name: String,

    /// Optional sender node (empty if unknown).
    pub sender_node: String,

    /// Direction as recorded by the logger, e.g. `"Rx"` or `"Tx"`.
    pub direction: String,

    /// Raw payload length token.
    pub byte_length: String,

    /// Parsed payload length in bytes.
    pub byte_length_value: u16,

    /// Payload bytes as hex pairs separated by spaces.
    pub data: String,

    /// Optional comment (empty if unknown).
    pub comment: String,
}

impl CanFrame {
    /// Prints all fields in a deterministic, human-readable order.
    ///
    /// Fields are aligned in two columns as `label: value`.  
    /// Empty optional fields (like `name` and `sender_node`) are shown as `-`.
    ///
    /// # Example
    /// ```
    /// # use can_tools::CanFrame;
    /// let frame = CanFrame {
    ///     absolute_time: "2023-05-12 16:16:06.532".into(),
    ///     timestamp: "0.016728".into(),
    ///     timestamp_value: 0.016728,
    ///     channel: 1,
    ///     protocol: "CAN".into(),
    ///     id: "17334410x".into(),
    ///     name: "".into(),
    ///     sender_node: "".into(),
    ///     direction: "Rx".into(),
    ///     byte_length: "8".into(),
    ///     byte_length_value: 8,
    ///     data: "3E 42 03 00 39 00 03 01".into(),
    ///     comment: "test comment".into(),
    /// };
    ///
    /// frame.print_ordered();
    /// ```
    pub fn print_ordered(&self) {
        // Fixed label width for alignment
        let w = 20;

        println!("CanFrame");
        println!("{:>w$}: {}", "absolute_time", self.absolute_time, w = w);
        println!("{:>w$}: {}", "timestamp", self.timestamp, w = w);
        println!("{:>w$}: {}", "timestamp_value", self.timestamp_value, w = w);
        println!("{:>w$}: {}", "channel", self.channel, w = w);
        println!("{:>w$}: {}", "protocol", self.protocol, w = w);
        println!("{:>w$}: {}", "id", self.id, w = w);
        println!(
            "{:>w$}: {}",
            "name",
            if self.name.is_empty() { "-" } else { &self.name },
            w = w
        );
        println!(
            "{:>w$}: {}",
            "sender_node",
            if self.sender_node.is_empty() { "-" } else { &self.sender_node },
            w = w
        );
        println!("{:>w$}: {}", "direction", self.direction, w = w);
        println!("{:>w$}: {}", "byte_length", self.byte_length, w = w);
        println!("{:>w$}: {}", "byte_length_value", self.byte_length_value, w = w);
        println!("{:>w$}: {}", "data", self.data, w = w);
        println!("{:>w$}: {}", "comment", self.comment, w = w);
    }

    /// Resets all fields to their default values (empty strings / zeros).
    ///
    /// This is equivalent to:
    /// ```rust
    /// # use can_tools::types::canframe::CanFrame;
    /// # let mut f = CanFrame::default();
    /// // f = CanFrame::default();
    /// ```
    pub fn clear(&mut self) {
        self.absolute_time.clear();
        self.timestamp.clear();
        self.timestamp_value = 0.0;
        self.channel = 0;
        self.protocol.clear();
        self.id.clear();
        self.name.clear();
        self.sender_node.clear();
        self.direction.clear();
        self.byte_length.clear();
        self.byte_length_value = 0;
        self.data.clear();
        self.comment.clear();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_can_frame() -> CanFrame {
        CanFrame {
            absolute_time: "date Mon Mar 10 12:00:00.000 pm 2025".to_string(),
            timestamp: "15.0".to_string(),
            timestamp_value: 15.0,
            channel: 2,
            protocol: "CAN FD".to_string(),
            id: "0x5E3".to_string(),
            name: "TestMessage".to_string(),
            sender_node: "Gateway".to_string(),
            direction: "Tx".to_string(),
            byte_length: "12".to_string(),
            byte_length_value: 12,
            data: "11 22 33 44 55 66 77 88 99 AA BB CC ".to_string(),
            comment: "test comment".into()
        }
    }

    #[test]
    fn test_clear() {
        let mut frame: CanFrame = build_test_can_frame();

        // Check that everything is back to default value
        frame.clear();
        assert_eq!(frame, CanFrame::default());
    }
}
