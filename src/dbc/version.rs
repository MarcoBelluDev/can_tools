use crate::types::database::Database;

pub(crate) fn fct(db: &mut Database, line: &str) {
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
    fn test_fct() {
        let mut db: Database = Database::default();
        // Example Line
        fct(&mut db, r#"VERSION "1.0.2""#);
        assert_eq!(db.version, "1.0.2");
    }
}