use crate::asc::types::signal_log::SignalLog;
use crate::dbc::types::signal::SignalDBC;

/// Convert a decoded raw integer into a `SignalLog` snapshot using the DBC signal metadata.
///
/// Note: the unit is normalized by removing an optional "Unit_" prefix.
#[inline]
pub(crate) fn to_sigframe(sig: &SignalDBC, raw_i: i64) -> SignalLog {
    let value: f64 = (raw_i as f64) * sig.factor + sig.offset;
    let text: String = sig
        .value_table
        .get(&(raw_i as i32))
        .cloned()
        .unwrap_or_default();
    SignalLog {
        message: 0,
        name: sig.name.clone(),
        factor: sig.factor,
        offset: sig.offset,
        channel: 0,
        raw: raw_i,
        value,
        unit: sig
            .unit_of_measurement
            .strip_prefix("Unit_")
            .unwrap_or(&sig.unit_of_measurement)
            .to_string(),
        text,
        comment: sig.comment.clone(),
        value_table: sig.value_table.clone(),
        values: Vec::new(),
    }
}
