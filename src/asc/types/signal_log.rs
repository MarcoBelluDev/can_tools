use std::collections::HashMap;

use crate::asc::types::canframe::CanFrame;

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
