use crate::dbc::core;
use crate::dbc::types::database::DatabaseDBC;
use crate::dbc::types::errors::DbcParseError;

use std::fs::File;
use std::io::{BufRead, BufReader};

use encoding_rs::WINDOWS_1252;

/// Parses a DBC file and returns a populated [`DatabaseDBC`] instance.
///
/// This function reads a DBC file from disk, parses its content line by line,
/// and fills the [`DatabaseDBC`] structure with all parsed information:
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
/// The reader decodes the file as Windows-1252 and transliterates a handful of characters
/// (e.g., `ü`, `ö`, `ß`) to ASCII fallbacks to keep downstream processing UTF-8 safe.
///
/// # Parameters
/// - `path`: Path to the `.dbc` file to parse.
///
/// # Returns
/// - `Ok(DatabaseDBC)` if the file was successfully read and parsed.
/// - `Err(DbcParseError)` detailing why the file could not be opened or read.
///
/// # Errors
/// Returns an `Err(DbcParseError)` if:
/// - The file cannot be opened.
/// - There are I/O errors while reading.
/// - The path does not end in `.dbc`.
///
/// # Notes
/// - This function is the main entry point for converting a DBC file into a structured [`DatabaseDBC`].
/// - Internal parsing details are handled by [`DatabaseDBC`] methods and are **not** part of the public API.
/// - Parsing stops only at the end of the file; malformed lines are skipped.
///
pub fn from_file(path: &str) -> Result<DatabaseDBC, DbcParseError> {
    // check if provided file has .dbc format
    if !path.ends_with(".dbc") {
        return Err(DbcParseError::InvalidExtension {
            path: path.to_string(),
        });
    }

    let path_owned: String = path.to_string();
    let file: File = File::open(path).map_err(|source| DbcParseError::OpenFile {
        path: path_owned.clone(),
        source,
    })?;
    let mut reader: BufReader<File> = BufReader::new(file);

    // Initialize DatabaseDBC
    let mut db: DatabaseDBC = DatabaseDBC::default();

    // Buffer for raw bytes of a line
    let mut raw_line: Vec<u8> = Vec::with_capacity(256);

    // For each line, transform german characters in UTF-8 compatible characters
    let read_decoded_line = |reader: &mut BufReader<File>,
                             buf: &mut Vec<u8>|
     -> Result<Option<String>, DbcParseError> {
        buf.clear();
        let read = reader
            .read_until(b'\n', buf)
            .map_err(|source| DbcParseError::Read {
                path: path_owned.clone(),
                source,
            })?;
        if read == 0 {
            return Ok(None);
        }
        let (s, _, _) = WINDOWS_1252.decode(buf);
        let src: String = s.into_owned();
        let mut out: String = String::with_capacity(src.len());
        for ch in src.chars() {
            match ch {
                'ü' => out.push('u'),
                'ö' => out.push('o'),
                'ä' => out.push('a'),
                'ß' => {
                    out.push('s');
                    out.push('s');
                }
                'Ü' => out.push('U'),
                'Ö' => out.push('O'),
                'Ä' => out.push('A'),
                '¿' => out.push('?'),
                _ => out.push(ch),
            }
        }
        // trim trailing CR/LF to behave like .lines()
        while out.ends_with(['\n', '\r']) {
            out.pop();
        }
        Ok(Some(out))
    };

    // Read and process each .dbc line
    loop {
        let Some(line) = read_decoded_line(&mut reader, &mut raw_line)? else {
            break;
        };

        // Work on a trimmed-start slice to preserve inner spaces elsewhere
        let line_trimmed: &str = line.trim_start();

        // skip comments and empty lines
        if line_trimmed.is_empty() || line_trimmed.starts_with("//") {
            continue;
        }

        // Extract first, second and third part from the line
        let mut parts = line_trimmed.split_ascii_whitespace();
        let first: &str = parts.next().unwrap_or("");
        let second: &str = parts.next().unwrap_or("");
        let third: &str = parts.next().unwrap_or("");

        match first {
            "VERSION" => {
                core::version::decode(&mut db, line_trimmed);
            }
            // Some DBCs use "BU_:" while others use "BU_". Accept both.
            "BU_:" => {
                core::bu_::decode(&mut db, line_trimmed);
            }
            "BO_" => {
                core::bo_::decode(&mut db, line_trimmed);
            }
            "SG_" => {
                core::sg_::decode(&mut db, line_trimmed);
            }
            "BO_TX_BU_" => {
                core::bo_tx_bu_::decode(&mut db, line_trimmed);
            }
            "CM_" => {
                if second.starts_with('"') {
                    // Network/global comment: CM_ "…";
                    core::comments::cm_::decode(&mut db, line_trimmed);
                } else if second == "BO_" {
                    core::comments::cm_bo_::decode(&mut db, line_trimmed);
                } else if second == "SG_" {
                    // Accumulate multiline until the comment has two unescaped quotes
                    let mut full_comment_line: String = line_trimmed.to_string();
                    if !core::strings::has_complete_quoted_segment(&full_comment_line) {
                        // Read subsequent lines until we close the quoted segment
                        while let Some(next) = read_decoded_line(&mut reader, &mut raw_line)? {
                            let next_trim = next.trim_start();
                            full_comment_line.push('\n');
                            full_comment_line.push_str(next_trim);
                            if core::strings::has_complete_quoted_segment(&full_comment_line) {
                                break;
                            }
                        }
                    }
                    core::comments::cm_sg_::decode(&mut db, &full_comment_line);
                } else if second == "BU_" {
                    let mut full_comment_line: String = line_trimmed.to_string();
                    if !core::strings::has_complete_quoted_segment(&full_comment_line) {
                        while let Some(next) = read_decoded_line(&mut reader, &mut raw_line)? {
                            let next_trim = next.trim_start();
                            full_comment_line.push('\n');
                            full_comment_line.push_str(next_trim);
                            if core::strings::has_complete_quoted_segment(&full_comment_line) {
                                break;
                            }
                        }
                    }
                    core::comments::cm_bu_::decode(&mut db, &full_comment_line);
                }
            }
            "BA_DEF_" => {
                if second == "BU_" {
                    core::attributes::ba_def_bu_::decode(&mut db, line_trimmed);
                } else if second == "BO_" {
                    core::attributes::ba_def_bo_::decode(&mut db, line_trimmed);
                } else if second == "SG_" {
                    core::attributes::ba_def_sg_::decode(&mut db, line_trimmed);
                } else {
                    core::attributes::ba_def_::decode(&mut db, line_trimmed);
                }
            }
            "BA_DEF_DEF_" => {
                core::attributes::ba_def_def_::decode(&mut db, line_trimmed);
            }
            "BA_" => {
                if third == "BU_" {
                    core::attributes::ba_bu_::decode(&mut db, line_trimmed);
                } else if third == "BO_" {
                    core::attributes::ba_bo_::decode(&mut db, line_trimmed);
                } else if third == "SG_" {
                    core::attributes::ba_sg_::decode(&mut db, line_trimmed);
                } else {
                    core::attributes::ba_::decode(&mut db, line_trimmed);
                }
            }
            "BA_DEF_REL_" => {
                core::attributes::ba_def_rel_::decode(&mut db, line_trimmed);
            }
            "BA_DEF_DEF_REL_" => {
                core::attributes::ba_def_def_rel_::decode(&mut db, line_trimmed);
            }
            "BA_REL_" => {
                core::attributes::ba_rel_::decode(&mut db, line_trimmed);
            }
            "VAL_" => {
                core::val_::decode(&mut db, line_trimmed);
            }
            "SIG_VALTYPE_" => {
                core::attributes::sig_valtype_::decode(&mut db, line_trimmed);
            }
            _ => {}
        }
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
