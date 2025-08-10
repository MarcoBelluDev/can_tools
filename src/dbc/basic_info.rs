use crate::types::database::Database;

pub(crate) fn fct(db: &mut Database, line: &str) {
    // Expected formats:
    // BA_ "DBName" "TestCAN";
    // BA_ "BusType" "CAN FD";
    // BA_ "Baudrate" 500000;
    // BA_  "BaudrateCANFD" 2000000;

    if line.contains(r#""Baudrate""#) {
        // BA_ "Baudrate" "500000";
        let mut parts = line.trim_end_matches(';').split_whitespace();
        parts.next(); // BA_
        parts.next(); // Baudrate
        if let Some(text) = parts.next() {
            if let Ok(baudrate) = text.parse::<u32>() {
                db.baudrate = baudrate;
            }
        }
    } else if line.contains(r#""BusType""#) {
        let mut parts = line.trim_end_matches(';').splitn(3, ' ');
        parts.next(); // BA_
        parts.next(); // BusType
        if let Some(text) = parts.next() {
            db.bustype = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""DBName""#) {
        let mut parts = line.trim_end_matches(';').split_whitespace();
        parts.next(); // BA_
        parts.next(); // DBName
        if let Some(text) = parts.next() {
            db.name = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""BaudrateCANFD""#) {
        let mut parts = line.trim_end_matches(';').split_whitespace();
        parts.next(); // BA_
        parts.next(); // Baudrate
        if let Some(text) = parts.next() {
            if let Ok(baudrate_canfd) = text.parse::<u32>() {
                db.baudrate_canfd = baudrate_canfd;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fct() {
        let mut db: Database = Database::default();

        // check that invalid lines are not accepted
        fct(&mut db, r#"BA_ "UnknownAttr" "SomeValue";"#);
        fct(&mut db, r#"This is not a valid line"#);
        fct(&mut db, r#"BA_ "Baudrate";"#); // Missing value

        // Nothing should be set
        assert_eq!(db.baudrate, 0);
        assert_eq!(db.bustype, "");
        assert_eq!(db.name, "");
        assert_eq!(db.baudrate_canfd, 0);

        // check valid lines are accepted
        fct(&mut db, r#"BA_ "Baudrate" 500000;"#);
        fct(&mut db, r#"BA_ "BusType" "CAN FD";"#);
        fct(&mut db, r#"BA_ "DBName" "TestCAN";"#);
        fct(&mut db, r#"BA_  "BaudrateCANFD" 2000000;"#);

        assert_eq!(db.baudrate, 500000);
        assert_eq!(db.bustype, "CAN FD");
        assert_eq!(db.name, "TestCAN");
        assert_eq!(db.baudrate_canfd, 2000000);
    }
}