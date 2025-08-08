use chrono::NaiveDateTime;

/// Represents an absolute, timezone-unaware timestamp.
///
/// `AbsoluteTime` keeps both the raw textual representation (`text`) and the
/// parsed value as a `NaiveDateTime` (`value`). The timestamp is **naive**
/// (i.e., it has no timezone or offset information) and should not be used for
/// DST/offset-sensitive computations without attaching a timezone.
///
/// If the input line does not start with `"date"` or the timestamp does not
/// match the expected format, parsing returns `None`.
///
/// # Fields
/// - `text`: The raw timestamp string **after** the leading `"date "`
///   prefix (e.g., `"Tue Aug 05 07:23:45.123 pm 2025"`).
/// - `value`: The parsed timestamp as `Some(NaiveDateTime)` on success, or
///   `None` if not available.
/// 
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AbsoluteTime {
    pub text: String,
    pub value: Option<NaiveDateTime>,
}
impl AbsoluteTime {
    /// Clears all metadata from this `AbsoluteTime`.
    ///
    /// # Effects
    /// - `text` → `""`
    /// - `value` → `None`
    pub fn clear(&mut self) {
        self.text.clear();
        self.value = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

    fn build_test_abs_time() -> AbsoluteTime {
        let date: NaiveDate = NaiveDate::from_ymd_opt(2025, 8, 5).unwrap();
        let time: NaiveTime = NaiveTime::from_hms_milli_opt(19, 23, 45, 123).unwrap(); // 19:23:45.123
        let dt: NaiveDateTime = NaiveDateTime::new(date, time);

        AbsoluteTime {
            // testo “decorativo” per il test; non viene validato
            text: "Tue Aug 05 07:23:45.123 pm 2025".into(),
            value: Some(dt),
        }
    }

    #[test]
    fn test_clear() {
        let mut abs_time: AbsoluteTime = build_test_abs_time();

        // Check that everything is back to default value
        abs_time.clear();
        assert_eq!(abs_time, AbsoluteTime::default());
    }
}
