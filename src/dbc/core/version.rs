use crate::dbc::types::database::DatabaseDBC;

pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    // Example: VERSION "1.0"
    db.version = line
        .to_lowercase()
        .replace("version", "") // delete version text
        .trim() // delete whitespaces
        .trim_matches('"') // delete "
        .to_string() // convert in string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let mut db: DatabaseDBC = DatabaseDBC::default();
        // Example Line
        decode(&mut db, r#"VERSION "1.0.2""#);
        assert_eq!(db.version, "1.0.2");
    }
}
