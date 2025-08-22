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
        let s: &str = lines[i].trim_start();

        // skip comments and empty lines
        if s.is_empty() || s.starts_with("//") {
            i += 1;
            continue;
        }

        // Extract first token (keyword) without allocating
        let mut it = s.split_ascii_whitespace();
        let tok: &str = it.next().unwrap_or("");

        if tok.eq_ignore_ascii_case("VERSION") {
            support::version::decode(&mut db, s);
        } else if tok.eq_ignore_ascii_case("BA_") {
            // Third part must be used to check where the line must be analyzed
            it.next();
            let third: &str = it.next().unwrap_or(""); 
            if third == "BU_" {
                // additional node info
                support::nodes::add_info(&mut db, s);
            } else if third == "BO_" {
                // additional message info
                support::messages::cycle_time(&mut db, s);
            } else if third == "SG_" {
                // additinoal signal info
            } else {
                // additional database info
                support::basic_info::decode(&mut db, s);
            }
        } else if tok.eq_ignore_ascii_case("BU_") {
            support::nodes::decode(&mut db, s);
        } else if tok.eq_ignore_ascii_case("BO_") {
            support::messages::decode(&mut db, s);
        } else if tok.eq_ignore_ascii_case("SG_") {
            support::signals::decode(&mut db, s);
        } else if tok.eq_ignore_ascii_case("BO_TX_BU_") {
            support::messages::tx_nodes(&mut db, s);
        } else if tok.eq_ignore_ascii_case("CM_") {
            // Second token determines target: "…", BO_, SG_, BU_
            let second: &str = it.next().unwrap_or("");
            if second.starts_with('"') {
                // Network/global comment: CM_ "…";
                support::basic_info::comment(&mut db, s);
            } else if second.eq_ignore_ascii_case("BO_") {
                support::messages::comments(&mut db, s);
            } else if second.eq_ignore_ascii_case("SG_") {
                // Accumulate multiline until at least two quotes (very common in DBC)
                let mut full_comment_line: String = s.to_string();
                while full_comment_line.matches('"').count() < 2 && i + 1 < lines.len() {
                    i += 1;
                    full_comment_line.push('\n');
                    full_comment_line.push_str(lines[i].trim());
                }
                support::signals::comments(&mut db, &full_comment_line);
            } else if second.eq_ignore_ascii_case("BU_") {
                let mut full_comment_line: String = s.to_string();
                while full_comment_line.matches('"').count() < 2 && i + 1 < lines.len() {
                    i += 1;
                    full_comment_line.push('\n');
                    full_comment_line.push_str(lines[i].trim());
                }
                support::nodes::comments(&mut db, &full_comment_line);
            }
        } else if tok.eq_ignore_ascii_case("VAL_") {
            support::signals::value_table(&mut db, s);
        }

        i += 1;
    }

    
    // Sanity checks with SlotMap: order keys exist and lookups point to valid entries
    #[cfg(debug_assertions)]
    {
        debug_assert!(db.nodes_order.iter().all(|&k| db.nodes.contains_key(k)));
        debug_assert!(db.messages_order.iter().all(|&k| db.messages.contains_key(k)));
        debug_assert!(db.signals_order.iter().all(|&k| db.signals.contains_key(k)));

        debug_assert!(db.node_key_by_name.values().all(|&k| db.nodes.contains_key(k)));
        debug_assert!(db.msg_key_by_id.values().all(|&k| db.messages.contains_key(k)));
        debug_assert!(db.msg_key_by_hex.values().all(|&k| db.messages.contains_key(k)));
        debug_assert!(db.msg_key_by_name.values().all(|&k| db.messages.contains_key(k)));
        debug_assert!(db.sig_key_by_name.values().all(|&k| db.signals.contains_key(k)));

        debug_assert!(db.messages.values().all(|m|
            m.sender_nodes.iter().all(|&nk| db.nodes.contains_key(nk))
        ));
        debug_assert!(db.messages.values().all(|m|
            m.signals.iter().all(|&sk| db.signals.contains_key(sk))
        ));
        debug_assert!(db.signals.values().all(|s|
            db.messages.contains_key(s.message) &&
            s.receiver_nodes.iter().all(|&nk| db.nodes.contains_key(nk))
        ));
    }

    db.sort_nodes_by_name();
    db.sort_messages_by_name();
    db.sort_signals_by_name();

    Ok(db)
}
