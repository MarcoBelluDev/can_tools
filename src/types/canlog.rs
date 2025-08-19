//! Types representing a parsed CAN trace and its derived structures.
//!
//! The model is intentionally **decoupled**:
//! - [`CanFrame`] is a light record about *when* and *where* something was seen.
//! - [`MessageLog`] contains the actual CAN message metadata and payload for a frame.
//! - [`SignalLog`] aggregates the evolution of a single decoded signal across the whole log.
//!
//! This split makes the UI code simpler (fast sorting by frame timestamp) while still
//! letting you access message metadata and iterate signals efficiently.
use std::collections::HashMap;

use crate::types::absolute_time::AbsoluteTime;

/// In-memory representation of a parsed CAN trace.
///
/// A `CanLog` is created by the ASC/DBC parsers and then consumed by downstream UIs/tools.
/// It stores frames in file order (`all_frame`) and keeps a separate store of message and signal data.
#[derive(Clone, Default)]
pub struct CanLog {
    /// Absolute start time extracted from the `date` header, if present.
    pub absolute_time: AbsoluteTime,

    /// All parsed frames in file order.
    pub can_frames: Vec<CanFrame>,

    /// One index per `(id, channel)` — points to the most recent frame in `all_frame` by `timestamp`.
    /// Invariant: every index is valid for `all_frame[idx]` and there is at most one index per unique pair.
    pub last_id_chn_frame: Vec<usize>,

    /// One MessageLog per parsed frame (decoupled from `CanFrame`).
    pub messages: Vec<MessageLog>,

    /// SignalLog contains everything needed to chart a signal over time (aggregated).
    pub signals: Vec<SignalLog>,
}

impl CanLog {
    /// Resets the log to its default (empty) state.
    pub fn clear(&mut self) {
        self.absolute_time.clear();
        self.can_frames = Vec::default();
        self.last_id_chn_frame = Vec::default();
        self.messages = Vec::default();
        self.signals = Vec::default();
    }
}

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
    pub fn clear(&mut self) {
        self.absolute_time.clear();
        self.timestamp = 0.0;
        self.channel = 0;
        self.direction.clear();
        self.message = 0;
    }
}

/// Metadata and payload carried by a [`CanFrame`].
///
/// There is typically one `MessageLog` per `CanFrame`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MessageLog {
    /// Logger channel index (typically 1-based).
    pub channel: u8,

    /// Raw payload length token.
    pub byte_length: u16,

    /// `"CAN"` or `"CAN FD"` depending on payload size.
    pub protocol: String,

    /// Raw identifier token as seen in the log (may include format suffixes).
    pub id: String,

    /// Empty if unknown (missing DB).
    pub name: String,

    /// Empty if unknown (missing DB).
    pub sender_node: String,

    /// Payload bytes as hex pairs separated by spaces.
    pub data: String,

    /// Optional comment (empty if unknown).
    pub comment: String,

    /// Indices into `CanLog.signals` for all `SignalLog` present in this message.
    pub signals: Vec<usize>,
}

impl MessageLog {
    pub fn clear(&mut self) {
        self.channel = 0;
        self.byte_length = 0;
        self.protocol.clear();
        self.id.clear();
        self.name.clear();
        self.sender_node.clear();
        self.data.clear();
        self.comment.clear();
        self.signals.clear();
    }
}

/// Aggregated time-series for a decoded signal.
///
/// The latest sample is mirrored in [field@SignalLog::raw],
/// [field@SignalLog::value] and [field@SignalLog::text], while the full
/// series is available in [field@SignalLog::values].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignalLog {
    /// Parent `MessageLog` index in `CanLog.messages` (most recent message contributing to this signal).
    pub message: usize,
    /// Signal name.
    pub name: String,
    /// Scale factor and offset used for value conversion.
    pub factor: f64,
    pub offset: f64,
    /// Channel where the signal appears.
    pub channel: u8,
    /// Last raw integer value (signed if needed).
    pub raw: i64,
    /// Last physical value = raw * factor + offset.
    pub value: f64,
    /// Unit of measurement.
    pub unit: String,
    /// Text mapped from value_table if matched, otherwise empty.
    pub text: String,
    /// Optional comment at signal level.
    pub comment: String,
    /// Value mapping table.
    pub value_table: HashMap<i32, String>,
    /// Time series of `[timestamp, value]` pairs (timestamp in seconds).
    pub values: Vec<[f64; 2]>,
}

impl SignalLog {
    /// Clears this `SignalLog` to defaults.
    pub fn clear(&mut self) {
        self.message = 0;
        self.name.clear();
        self.factor = 0.0;
        self.offset = 0.0;
        self.channel = 0;
        self.raw = 0;
        self.value = 0.0;
        self.unit.clear();
        self.text.clear();
        self.comment.clear();
        self.value_table = HashMap::default();
        self.values = Vec::default();
    }

    /// Returns the value at or just before `ts` and the related `text` derived via `value_table`,
    /// using `factor`/`offset` to back-compute the raw integer if an exact mapping is required.
    /// If there is no sample at or before `ts`, returns the first sample if present.
    pub fn value_text_at(&self, ts: f64) -> Option<(f64, String)> {
        if self.values.is_empty() {
            return None;
        }
        let factor = if self.factor == 0.0 { 1.0 } else { self.factor };
        let offset = self.offset;

        // Fast-path: last sample <= ts
        if self.values[self.values.len() - 1][0] <= ts {
            let v = self.values[self.values.len() - 1][1];
            let raw = ((v - offset) / factor).round() as i32;
            let txt = self.value_table.get(&raw).cloned().unwrap_or_default();
            return Some((v, txt));
        }
        // Scan backwards
        for i in (0..self.values.len()).rev() {
            let t = self.values[i][0];
            if (t - ts).abs() <= 1e-6 || t < ts {
                let v = self.values[i][1];
                let raw = ((v - offset) / factor).round() as i32;
                let txt = self.value_table.get(&raw).cloned().unwrap_or_default();
                return Some((v, txt));
            }
        }
        let v = self.values[0][1];
        let raw = ((v - offset) / factor).round() as i32;
        let txt = self.value_table.get(&raw).cloned().unwrap_or_default();
        Some((v, txt))
    }
}

impl std::fmt::Display for CanFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Minimal display: timestamp, channel, direction, message index
        write!(
            f,
            "{:.6} ch:{} {} msg:{}",
            self.timestamp, self.channel, self.direction, self.message
        )
    }
}

/// Yields the [`SignalLog`]s referenced by the [`MessageLog`] at `msg_idx`.
pub fn resolve_message_signals<'a>(
    log: &'a CanLog,
    msg_idx: usize,
) -> impl Iterator<Item = &'a SignalLog> + 'a {
    log.messages
        .get(msg_idx)
        .into_iter()
        .flat_map(|msg| msg.signals.iter().copied())
        .filter_map(|sidx| log.signals.get(sidx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_text_at_picks_correct_samples() {
        let mut sig = SignalLog::default();
        sig.factor = 1.0;
        sig.offset = 0.0;
        sig.value_table.insert(10, "ten".to_string());
        sig.value_table.insert(20, "twenty".to_string());
        sig.values.push([1.0, 10.0]);
        sig.values.push([2.0, 20.0]);

        // exact
        let (v, t) = sig.value_text_at(2.0).unwrap();
        assert_eq!(v, 20.0);
        assert_eq!(t, "twenty");

        // before
        let (v, t) = sig.value_text_at(1.5).unwrap();
        assert_eq!(v, 10.0);
        assert_eq!(t, "ten");

        // before first
        let (v, t) = sig.value_text_at(0.5).unwrap();
        assert_eq!(v, 10.0);
        assert_eq!(t, "ten");
    }

    #[test]
    fn resolve_message_signals_yields_expected() {
        let mut log = CanLog::default();
        // build two signals
        let s0 = SignalLog {
            name: "A".into(),
            ..Default::default()
        };
        let s1 = SignalLog {
            name: "B".into(),
            ..Default::default()
        };
        log.signals.push(s0);
        log.signals.push(s1);

        // message that references them
        let msg = MessageLog {
            signals: vec![0, 1],
            ..Default::default()
        };
        log.messages.push(msg);

        // iterate
        let it = resolve_message_signals(&log, 0);
        let names: Vec<String> = it.map(|s| s.name.clone()).collect();
        assert_eq!(names, vec!["A".to_string(), "B".to_string()]);
    }
}
