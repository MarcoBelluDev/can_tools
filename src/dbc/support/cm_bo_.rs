use crate::types::database::Database;

/// `CM_ BO_ <ID> "Comment...";`
pub(crate) fn decode(db: &mut Database, line: &str) {
    let mut parts = line.split_ascii_whitespace();
    if parts.next() != Some("CM_") {
        return;
    }
    if parts.next() != Some("BO_") {
        return;
    }

    let id: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    if id == 0 {
        return;
    }

    let line: &str = line.trim_end_matches(';').trim();
    let first: usize = match line.find('\"') {
        Some(p) => p,
        None => return,
    };
    let last: usize = match line.rfind('\"') {
        Some(p) if p > first => p,
        _ => return,
    };
    let comment: &str = &line[first + 1..last];

    if let Some(msg) = db.get_message_by_id_mut(id) {
        msg.comment = comment.to_string();
    }
}