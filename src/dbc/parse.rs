use crate::dbc::support;
use crate::types::database::Database;

use std::fs::File;
use std::io::{BufReader, Read};

use encoding_rs::WINDOWS_1252;

/// Parses a DBC file and returns a populated [`Database`] instance.
///
/// This function reads a DBC file from disk, parses its content line by line,
/// and fills the [`Database`] structure with all parsed information:
/// - **Name** (from `BA_ "DBName"` line)
/// - **Version** (from `VERSION` line)
/// - **Baudrate** (from `BA_ "Baudrate"` line)
/// - **Baudrate CAN FD** (from `BA_ "BaudrateCANFD"` line)
/// - **BusType** (from `BA_ "BusType"` line)
/// - **Nodes** (from `BU_` line)
/// - **Messages** (from `BO_` lines)
/// - **Signals** (from `SG_` lines)
/// - **Sender nodes** (from `BO_TX_BU_` lines)
/// - **Comments** for messages, signals, and nodes (from `CM_` lines)
/// - **Value tables** (from `VAL_` lines)
///
/// The parsing logic is tolerant to extra spaces, comments, and multi-line strings.
/// Multi-line comments for signals and nodes are correctly joined before parsing.
///
/// # Parameters
/// - `path`: Path to the `.dbc` file to parse.
///
/// # Returns
/// - `Ok(Database)` if the file was successfully read and parsed.
/// - `Err(String)` if the file could not be opened or read.
///
/// # Errors
/// Returns an `Err` with a human-readable error message if:
/// - The file cannot be opened.
/// - There are I/O errors while reading.
/// - The DBC content is malformed beyond recovery (most parsing errors are ignored and result in missing elements).
///
/// # Notes
/// - This function is the main entry point for converting a DBC file into a structured [`Database`].
/// - Internal parsing details are handled by [`Database`] methods and are **not** part of the public API.
/// - Parsing stops only at the end of the file; malformed lines are skipped.
///
pub fn from_file(path: &str) -> Result<Database, String> {
    // check if provided file has .asc format
    if !path.ends_with(".dbc") {
        return Err("Not a valid .dbc file format".to_string());
    }

    let file: File = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    let mut reader: BufReader<File> = BufReader::new(file);

    // read raw byted
    let mut bytes: Vec<u8> = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read error: {}", e))?;

    // Decode in Windows-1252
    let (text, _, _) = WINDOWS_1252.decode(&bytes);

    // Swap german chars with utf-8 chars
    let mut text: String = text.into_owned();
    text = text
        .replace('ü', "u")
        .replace('ö', "o")
        .replace('ä', "a")
        .replace('ß', "ss")
        .replace('Ü', "U")
        .replace('Ö', "O")
        .replace('Ä', "A")
        .replace('¿', "?");

    // split text in lines
    let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();

    // Initialize database and row counter
    let mut db: Database = Database::default();
    let mut i: usize = 0;

    while i < lines.len() {
        let line: &str = lines[i].trim();

        // skip comments and empty lines
        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        if line.to_lowercase().starts_with("version") {
            support::version::decode(&mut db, line);
        } else if line.to_lowercase().starts_with("ba_ ") {
            support::basic_info::decode(&mut db, line);
        } else if line.to_lowercase().starts_with("bu_") {
            support::nodes::decode(&mut db, line);
        } else if line.to_lowercase().starts_with("bo_ ") {
            if line.split_ascii_whitespace().count() >= 4 {
                support::messages::decode(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("sg_") {
            if line.split_ascii_whitespace().count() >= 5 {
                support::signals::decode(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("bo_tx_bu_") {
            if line.split_ascii_whitespace().count() >= 2 {
                support::messages::tx_nodes(&mut db, line); // ok
            }
        } else if line.to_lowercase().starts_with(r#""cm_ """#) {
            if line.split_ascii_whitespace().count() >= 1 {
                support::basic_info::comment(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("cm_ bo_") {
            if line.split_ascii_whitespace().count() >= 2 {
                support::messages::comments(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("cm_ sg_") {
            let mut full_comment_line: String = line.to_string();
            while full_comment_line.matches('"').count() < 2 && i + 1 < lines.len() {
                i += 1;
                full_comment_line.push('\n');
                full_comment_line.push_str(lines[i].trim());
            }
            support::signals::comments(&mut db, line);
        } else if line.to_lowercase().starts_with("cm_ bu_") {
            let mut full_comment_line: String = line.to_string();
            while full_comment_line.matches('"').count() < 2 && i + 1 < lines.len() {
                i += 1;
                full_comment_line.push('\n');
                full_comment_line.push_str(lines[i].trim());
            }
            support::nodes::comments(&mut db, line);
        } else if line.to_lowercase().starts_with(r#""ba_ "genmsgcycletime""#)
            && line.split_ascii_whitespace().count() >= 3
        {
            support::messages::cycle_time(&mut db, line);
        } else if line.to_lowercase().starts_with("val_")
            && line.split_ascii_whitespace().count() >= 3
        {
            support::signals::value_table(&mut db, line);
        }

        i += 1;
    }

    Ok(db)
}
