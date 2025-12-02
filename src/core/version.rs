use crate::types::database::CanDatabase;

pub(crate) fn decode(db: &mut CanDatabase, line: &str) {
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
        let mut db: CanDatabase = CanDatabase::default();
        // Example Line
        decode(&mut db, r#"VERSION "1.0.2""#);
        assert_eq!(db.version, "1.0.2");
    }
}
