use crate::dbc::support;
use crate::dbc::support::strings::{accumulate_until_two_unescaped_quotes, has_complete_quoted_segment};
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
    // check if provided file has .dbc format
    if !path.ends_with(".dbc") {
        return Err("Not a valid .dbc file format".to_string());
    }

    let file: File = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    let mut reader: BufReader<File> = BufReader::new(file);

    // read raw bytes
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
        // Work on a trimmed-start slice to preserve inner spaces
        let line: &str = lines[i].trim_start();

        // skip comments and empty lines
        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        // Extract first, second and third part from the line
        let mut parts = line.split_ascii_whitespace();
        let first: &str = parts.next().unwrap_or("");
        let second: &str = parts.next().unwrap_or("");
        let third: &str = parts.next().unwrap_or("");

        match first {
            "VERSION" => {
                support::version::decode(&mut db, line);
            },
            "BA_" => {
                if third == "BU_" {
                    support::ba_bu_::decode(&mut db, line);
                } else if third == "BO_" {
                    support::ba_bo_::decode(&mut db, line);
                } else if third == "SG_" {
                } else {
                    support::ba_::decode(&mut db, line);
                }
            },
            "BU_" => {
                support::bu_::decode(&mut db, line);
            },
            "BO_" => {
                support::bo_::decode(&mut db, line);
            },
            "SG_" => {
                support::sg_::decode(&mut db, line);
            },
            "BO_TX_BU_" => {
                support::bo_tx_bu_::decode(&mut db, line);
            },
            "CM_" => {
                if second.starts_with('"') {
                    // Network/global comment: CM_ "…";
                    support::cm_::decode(&mut db, line);
                } else if second == "BO_" {
                    support::cm_bo_::decode(&mut db, line);
                } else if second == "SG_" {
                    // Accumulate multiline until the comment has two unescaped quotes
                    let mut full_comment_line: String = line.to_string();
                    if !has_complete_quoted_segment(&full_comment_line) {
                        accumulate_until_two_unescaped_quotes(&mut full_comment_line, &lines, &mut i);
                    }
                    support::cm_sg_::decode(&mut db, &full_comment_line);
                } else if second == "BU_" {
                    let mut full_comment_line: String = line.to_string();
                    if !has_complete_quoted_segment(&full_comment_line) {
                        accumulate_until_two_unescaped_quotes(&mut full_comment_line, &lines, &mut i);
                    }
                    support::cm_bu_::decode(&mut db, &full_comment_line);
                }
            },
            "VAL_" => {
                support::val_::decode(&mut db, line);
            },
            _ => {},
        }

        i += 1;
    }

    
    // re-order
    db.sort_db_nodes_by_name();
    db.sort_db_messages_by_name();
    db.sort_db_signals_by_name();
    db.sort_all_node_fields();
    db.sort_all_message_fields();
    db.sort_all_signal_fields();

    Ok(db)
}
