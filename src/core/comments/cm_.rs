use crate::types::database::CanDatabase;

/// Decodes a free-standing database comment (`CM_ "..."`).
pub(crate) fn decode(db: &mut CanDatabase, line: &str) {
    let s: &str = line.trim_end_matches(';');
    if let Some((_, rest)) = s.split_once('"')
        && let Some((inner, _)) = rest.rsplit_once('"')
    {
        db.comment = inner.to_string(); // quotes removed
    }
}
