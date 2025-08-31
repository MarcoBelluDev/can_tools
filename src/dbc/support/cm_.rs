use crate::types::database::Database;

pub(crate) fn decode(db: &mut Database, line: &str) {
    // Expected formats:
    // CM_ "Comment regarding the network";
    let s: &str = line.trim_end_matches(';');
    if let Some((_, rest)) = s.split_once('"') {
        if let Some((inner, _)) = rest.rsplit_once('"') {
            db.comment = inner.to_string(); // quotes removed
        }
    }
}