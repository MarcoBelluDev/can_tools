use crate::dbc::types::database::DatabaseDBC;

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Expected formats:
    // CM_ "Comment regarding the network";
    let s: &str = line.trim_end_matches(';');
    if let Some((_, rest)) = s.split_once('"')
        && let Some((inner, _)) = rest.rsplit_once('"')
    {
        db.comment = inner.to_string(); // quotes removed
    }
}
