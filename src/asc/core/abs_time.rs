use chrono::NaiveDateTime;

use crate::asc::types::absolute_time::AbsoluteTime;

pub(crate) fn from_line(line: &str) -> Option<AbsoluteTime> {
    // splits in words by whitespaces
    let mut parts = line.split_ascii_whitespace();

    // check first word
    if parts.next()? != "date" {
        return None;
    }

    // rebuild data
    let date_str: String = parts.collect::<Vec<_>>().join(" ");

    // Chrono parsing pattern
    let fmt = "%a %b %d %I:%M:%S%.3f %P %Y";

    // parsing
    let naive_dt: NaiveDateTime = NaiveDateTime::parse_from_str(&date_str, fmt).ok()?;

    Some(AbsoluteTime {
        text: date_str,
        value: Some(naive_dt),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};

    #[test]
    fn parses_valid_lowercase_pm() {
        // Valid header exactly matching the fixed format:
        // "%a %b %d %I:%M:%S%.3f %P %Y"
        let line = "date Mon Mar 10 12:34:56.789 pm 2025";

        let abs = from_line(line).expect("should parse valid 'date' line");
        // `text` must equal the portion after the leading "date "
        assert_eq!(abs.text, "Mon Mar 10 12:34:56.789 pm 2025");

        let got: NaiveDateTime = abs.value.expect("value should be Some");
        let expected = NaiveDate::from_ymd_opt(2025, 3, 10)
            .unwrap()
            .and_hms_milli_opt(12, 34, 56, 789)
            .unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn normalizes_multiple_spaces() {
        // The code uses `split_whitespace` + `join(" ")`, so extra spaces get normalized.
        let line = "date Mon Mar 10 12:00:00.000 pm 2025";

        let abs = from_line(line).expect("should parse despite extra spaces");
        assert_eq!(abs.text, "Mon Mar 10 12:00:00.000 pm 2025");

        let got = abs.value.unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 3, 10)
            .unwrap()
            .and_hms_milli_opt(12, 0, 0, 0)
            .unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn parses_12_am_as_midnight() {
        // 12:xx am in 12-hour clock = 00:xx in 24-hour clock.
        let line = "date Mon Mar 10 12:00:00.000 am 2025";

        let abs = from_line(line).expect("should parse 12 am correctly");
        let got = abs.value.unwrap();
        let expected = NaiveDate::from_ymd_opt(2025, 3, 10)
            .unwrap()
            .and_hms_milli_opt(0, 0, 0, 0)
            .unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn rejects_non_date_prefix() {
        // The function is case-sensitive on the leading token "date".
        let line = "DATE Mon Mar 10 12:00:00.000 pm 2025";
        assert!(from_line(line).is_none(), "prefix must be exactly 'date'");
    }
}
