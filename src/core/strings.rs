//! Utilities for parsing quoted strings in DBC files.
//!
//! These helpers support escaped quotes (`\"`) and multi-line quoted strings,
//! which are common in `CM_` comments or attribute values.

/// Counts unescaped double quotes in a string.
///
/// A quote is considered escaped if immediately preceded by an **odd** number
/// of backslashes. This matches how DBC escapes quoted content.
pub(crate) fn count_unescaped_quotes(s: &str) -> usize {
    let mut count = 0usize;
    let mut backslashes = 0usize;
    for ch in s.chars() {
        if ch == '\\' {
            backslashes += 1;
            continue;
        }
        if ch == '"' && backslashes.is_multiple_of(2) {
            count += 1;
        }
        backslashes = 0;
    }
    count
}

/// Returns `true` if the string contains at least two unescaped quotes.
pub(crate) fn has_complete_quoted_segment(s: &str) -> bool {
    count_unescaped_quotes(s) >= 2
}

/// Collects every quoted segment (`"..."`) within the provided string.
///
/// This is tolerant to unclosed quotes: parsing stops at the first unmatched
/// `"` encountered.
pub(crate) fn collect_all_quoted(s: &str) -> Vec<String> {
    let bytes: &[u8] = s.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i: usize = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'"' {
            i += 1; // skip opening quote
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            if i <= bytes.len() {
                out.push(s[start..i].to_string());
                i += 1; // skip closing quote
                continue;
            } else {
                break; // unclosed quotes
            }
        }
        i += 1;
    }

    out
}
