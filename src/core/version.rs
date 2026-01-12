use crate::types::database::CanDatabase;

/// Parses the `VERSION` line and stores the version string on the database.
pub(crate) fn decode(db: &mut CanDatabase, line: &str) {
    db.version = line
        .to_lowercase()
        .replace("version", "") // delete version text
        .trim() // delete whitespaces
        .trim_matches('"') // delete "
        .to_string() // convert in string
}
