use crate::models::database::Database;

use std::fs::File;
use std::io::{BufRead, BufReader};

/// read file dbc and populate Database, Messages, Signals and Node structs
pub fn parse(path: &str) -> Result<Database, String> {
    let file: File = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    let reader: BufReader<File> = BufReader::new(file);

    let mut db: Database = Database::default();

    for line in reader.lines().map_while(Result::ok) {
        let line: &str = line.trim();

        // skip comments and empty lines
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if line.to_lowercase().starts_with("version") {
            db.parse_version(line);
        } else if line.to_lowercase().starts_with("bs_") {
            db.parse_bit_timing(line);
        } else if line.to_lowercase().starts_with("bu_") {
            db.parse_nodes(line);
        } else if line.to_lowercase().starts_with("bo_") {
            db.parse_messages(line);
        } else if line.trim_start().to_lowercase().starts_with("sg_") {
            db.parse_signal(line);
        } else if line.starts_with("val_") {
            db.parse_value_table(line);
        }
        // else if line.starts_with("CM_") { ... }
    }

    Ok(db)
}
