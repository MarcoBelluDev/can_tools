//! # arxml
//!
//! Parser utilities for reading **AUTOSAR ARXML** files and extracting CAN clusters
//! into the SlotMap-backed [`DatabaseDBC`](crate::dbc::types::database::DatabaseDBC). Ethernet clusters are ignored by design.
//!
//! _Module docs refreshed._
//!

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fs::File;
use std::io::BufReader;

use crate::arxml::types::database::{BusType, DatabaseARXML};

pub mod types;

// <SHORT-NAME>word</SHORT-NAME>
// triggers three events:
// Event::Start("SHORT-NAME")
// Event::Text("SHORT-NAME")
// Event::End("SHORT-NAME")

/// Parses an AUTOSAR ARXML file and extracts CAN clusters.
///
/// Returns one [`DatabaseARXML`] per `<CAN-CLUSTER>` found.
/// Ethernet clusters and unrelated content are ignored. The parser reads the file as XML, captures the
/// cluster `SHORT-NAME`, baud rates (`BAUDRATE`, `CAN-FD-BAUDRATE`) and the
/// version string from `ADMIN-DATA/SD[GID="Version"]` when present.
///
/// - `path`: Path to the `.arxml` file. Must end with `.arxml`.
/// - `Ok(Vec<DatabaseARXML>)` on success; `Err(String)` on invalid extension, I/O,
///   or XML errors.
pub fn parse_from_file(path: &str) -> Result<Vec<DatabaseARXML>, String> {
    if !path.ends_with(".arxml") {
        return Err("Not a valid .arxml file format".to_string());
    }

    let file: File = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut reader: Reader<BufReader<File>> = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);

    let mut buf: Vec<u8> = Vec::new();
    let mut databases: Vec<DatabaseARXML> = Vec::new();
    let mut current_tag_stack: Vec<String> = Vec::new();

    let mut current_db: Option<DatabaseARXML> = None;
    let mut in_can_cluster: bool = false;
    let mut in_admin_data: bool = false;
    let mut capture_version: bool = false;

    loop {
        match reader.read_event_into(&mut buf) {
            // enter at every event start like <word>
            Ok(Event::Start(word)) => {
                let tag = std::str::from_utf8(word.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                current_tag_stack.push(tag.clone());

                match tag.as_str() {
                    "CAN-CLUSTER" => {
                        in_can_cluster = true;
                        current_db = Some(DatabaseARXML {
                            bustype: BusType::Can,
                            ..Default::default()
                        });
                    }
                    "ADMIN-DATA" if in_can_cluster => {
                        in_admin_data = true;
                    }
                    "SD" if in_admin_data => {
                        // check if SD contain GID="Version"
                        if word
                            .attributes()
                            .filter_map(Result::ok)
                            .any(|a| a.key.as_ref() == b"GID" && a.value.as_ref() == b"Version")
                        {
                            capture_version = true;
                        }
                    }
                    _ => {}
                }
            }

            // enter at every event content like <SHORT-NAME>word</SHORT-NAME>
            Ok(Event::Text(word)) => {
                let text = word.decode().unwrap_or_default().trim().to_string();

                if let Some(current) = current_tag_stack.last()
                    && let Some(ref mut db) = current_db
                {
                    match current.as_str() {
                        "SHORT-NAME" if in_can_cluster && db.name.is_empty() => {
                            db.name = text;
                        }
                        "BAUDRATE" => {
                            db.baudrate = text.parse().unwrap_or(0);
                        }
                        "CAN-FD-BAUDRATE" => {
                            db.baudrate_canfd = text.parse().unwrap_or(0);
                            db.bustype = BusType::CanFd;
                        }
                        "SD" if capture_version => {
                            db.version = text;
                            capture_version = false;
                        }
                        _ => {}
                    }
                }
            }

            // enter at every closing event like </word>
            Ok(Event::End(word)) => {
                let tag = std::str::from_utf8(word.name().as_ref())
                    .unwrap_or("")
                    .to_string();
                if tag == "CAN-CLUSTER" {
                    if let Some(db) = current_db.take() {
                        databases.push(db);
                    }
                    in_can_cluster = false;
                } else if tag == "ADMIN-DATA" {
                    in_admin_data = false;
                }

                current_tag_stack.pop();
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parsing error: {}", e)),
            _ => {}
        }

        buf.clear();
    }

    Ok(databases)
}
