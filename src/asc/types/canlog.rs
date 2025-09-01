//! CAN log model.
//!
//! Types representing a parsed CAN trace and its derived structures, kept intentionally **decoupled**:
//! - [`CanFrame`]: timing/channel/direction plus a handle to the associated [`MessageLog`].
//! - [`MessageLog`]: identifier, name, payload, and per-frame message metadata.
//! - [`SignalLog`]: time series for a single decoded signal (values over timestamps).
//!
//!
//! _Docs refreshed: 2025-08-22_
//!
use crate::asc::types::{
    absolute_time::AbsoluteTime, canframe::CanFrame, message_log::MessageLog, signal_log::SignalLog,
};

/// In-memory representation of a parsed CAN trace.
///
/// It stores frames in file order (`can_frames`) and keeps a separate store of message and signal data.
#[derive(Clone, Default)]
pub struct CanLog {
    /// Absolute start time extracted from the `date` header, if present.
    pub absolute_time: AbsoluteTime,

    /// All parsed frames in file order.
    pub can_frames: Vec<CanFrame>,

    /// One index per `(id, channel)` — points to the most recent frame in `can_frames` by `timestamp`.
    /// Invariant: every index is valid for `can_frames[idx]` and there is at most one index per unique pair.
    /// Useful when you need one snapshot per message/channel pair.
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
