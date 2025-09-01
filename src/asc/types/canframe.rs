/// A single row from the log with timing/channel and a pointer to its [`MessageLog`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CanFrame {
    /// Absolute timestamp in `%Y-%m-%d %H:%M:%S%.3f` when available,
    /// otherwise derived by the parser.
    pub absolute_time: String,

    /// Relative timestamp as it appeared in the log (e.g. `"0.016728"`).
    pub timestamp: f32,

    /// Logger channel index (typically 1-based).
    pub channel: u8,

    /// Direction as recorded by the logger, e.g. `"Rx"` or `"Tx"`.
    pub direction: String,

    /// Index into `CanLog.messages` for the message carried by this frame.
    pub message: usize,
}

impl CanFrame {
    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        self.absolute_time.clear();
        self.timestamp = 0.0;
        self.channel = 0;
        self.direction.clear();
        self.message = 0;
    }
}
