/// Utilities for parsing quoted strings in DBC files.
///
/// These helpers support escaped quotes (\") and multi-line quoted strings,
/// which are common in CM_ comments or attribute values.

/// Count unescaped double quotes in a string.
/// A quote is considered escaped if immediately preceded by an odd number of backslashes.
pub(crate) fn count_unescaped_quotes(s: &str) -> usize {
    let mut count = 0usize;
    let mut backslashes = 0usize;
    for ch in s.chars() {
        if ch == '\\' {
            backslashes += 1;
            continue;
        }
        if ch == '"' && backslashes % 2 == 0 {
            count += 1;
        }
        backslashes = 0;
    }
    count
}

/// Return true if the string contains at least two unescaped quotes.
pub(crate) fn has_complete_quoted_segment(s: &str) -> bool {
    count_unescaped_quotes(s) >= 2
}

/// Accumulate subsequent lines until the buffer contains at least two unescaped quotes.
///
/// - `acc` should start with the current line content.
/// - `lines` is the full file as owned Strings.
/// - `i` is the current line index; it will be advanced as lines are consumed.
///
/// Newlines are inserted between concatenated physical lines.
pub(crate) fn accumulate_until_two_unescaped_quotes(
    acc: &mut String,
    lines: &[String],
    i: &mut usize,
)
{
    while !has_complete_quoted_segment(acc) && *i + 1 < lines.len() {
        *i += 1;
        acc.push('\n');
        // Preserve leading spaces as in original code used trim(); prefer trim_start to avoid trailing-quote issues
        acc.push_str(lines[*i].trim_start());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_unescaped_quotes() {
        assert_eq!(count_unescaped_quotes("\"a\""), 2);
        assert_eq!(count_unescaped_quotes("\\\"a\\\""), 0);
        assert!(has_complete_quoted_segment("before \"x\" after"));
        assert!(!has_complete_quoted_segment("before \"x without end"));
    }

    #[test]
    fn test_accumulate_until_two_unescaped_quotes() {
        let lines = vec![
            "CM_ SG_ 123 Sig \"part1".to_string(),
            "continues\";".to_string(),
        ];
        let mut acc = lines[0].clone();
        let mut i = 0usize;
        accumulate_until_two_unescaped_quotes(&mut acc, &lines, &mut i);
        assert!(has_complete_quoted_segment(&acc));
        assert_eq!(i, 1);
    }
}

