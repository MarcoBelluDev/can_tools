use crate::types::database::Database;

pub(crate) fn decode(db: &mut Database, line: &str) {
    // Expected formats:
    // BA_ "DBName" "TestCAN";
    // BA_ "BusType" "CAN FD";
    // BA_ "Baudrate" 500000;
    // BA_ "BaudrateCANFD" 2000000;

    if line.contains(r#""Baudrate""#) {
        // BA_ "Baudrate" "500000";
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // Baudrate
        if let Some(text) = parts.next() {
            if let Ok(baudrate) = text.parse::<u32>() {
                db.baudrate = baudrate;
            }
        }
    } else if line.contains(r#""BusType""#) {
        // Expected: BA_ "BusType" "CAN FD";
        let s: &str = line.trim_end_matches(';').trim();

        // After split by '"': [unquoted, "BusType", unquoted, "CAN FD", ...]
        let mut quoted = s.split('"').skip(1).step_by(2);
        if let (Some(key), Some(val)) = (quoted.next(), quoted.next()) {
            if key.eq_ignore_ascii_case("BusType") {
                db.bustype = val.to_string(); // "CAN" or "CAN FD"
            }
        }
    } else if line.contains(r#""DBName""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // DBName
        if let Some(text) = parts.next() {
            db.name = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""BaudrateCANFD""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // BaudrateCANFD
        if let Some(text) = parts.next() {
            if let Ok(baudrate_canfd) = text.parse::<u32>() {
                db.baudrate_canfd = baudrate_canfd;
            }
        }
    }
}

pub(crate) fn comment(db: &mut Database, line: &str) {
    // Expected formats:
    // CM_ "Comment regarding the network";
    let s: &str = line.trim_end_matches(';');
    if let Some((_, rest)) = s.split_once('"') {
        if let Some((inner, _)) = rest.rsplit_once('"') {
            db.comment = inner.to_string(); // quotes removed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let mut db: Database = Database::default();

        // check that invalid lines are not accepted
        decode(&mut db, r#"BA_ "UnknownAttr" "SomeValue";"#);
        decode(&mut db, r#"This is not a valid line"#);
        decode(&mut db, r#"BA_ "Baudrate";"#); // Missing value

        // Nothing should be set
        assert_eq!(db.baudrate, 0);
        assert_eq!(db.bustype, "");
        assert_eq!(db.name, "");
        assert_eq!(db.baudrate_canfd, 0);

        // check valid lines are accepted
        decode(&mut db, r#"BA_ "Baudrate" 500000;"#);
        decode(&mut db, r#"BA_ "BusType" "CAN FD";"#);
        decode(&mut db, r#"BA_ "DBName" "TestCAN";"#);
        decode(&mut db, r#"BA_ "BaudrateCANFD" 2000000;"#);

        assert_eq!(db.baudrate, 500000);
        assert_eq!(db.bustype, "CAN FD");
        assert_eq!(db.name, "TestCAN");
        assert_eq!(db.baudrate_canfd, 2000000);
    }
}
