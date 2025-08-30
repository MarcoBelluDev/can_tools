//! # arxml
//!
//! Parser utilities for reading **AUTOSAR ARXML** files and extracting CAN clusters
//! into the SlotMap-backed [`Database`]. Ethernet clusters are ignored by design.
//!
//! _Module docs refreshed._
//!

use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::fs::File;
use std::io::BufReader;

use crate::types::database::{BusType, Database};

// <SHORT-NAME>word</SHORT-NAME>
// triggers three events:
// Event::Start("SHORT-NAME")
// Event::Text("SHORT-NAME")
// Event::End("SHORT-NAME")

pub fn parse_from_file(path: &str) -> Result<Vec<Database>, String> {
    if !path.ends_with(".arxml") {
        return Err("Not a valid .arxml file format".to_string());
    }

    let file: File = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mut reader: Reader<BufReader<File>> = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(true);

    let mut buf: Vec<u8> = Vec::new();
    let mut databases: Vec<Database> = Vec::new();
    let mut current_tag_stack: Vec<String> = Vec::new();

    let mut current_db: Option<Database> = None;
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
                        current_db = Some(Database {
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

                if let Some(current) = current_tag_stack.last() {
                    if let Some(ref mut db) = current_db {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_parse_can_clusters() {
        // XML d'esempio con 2 CAN-CLUSTER e 1 ETHERNET-CLUSTER
        let xml = r#"
 <AUTOSAR>
 <AR-PACKAGES>
 <AR-PACKAGE>
 <ELEMENTS>
 <CAN-CLUSTER>
 <SHORT-NAME>Cluster_CAN_1</SHORT-NAME>
 <ADMIN-DATA>
 <SDGS>
 <SDG GID="ClusterVersion">
 <SD GID="Version">V1.0.0</SD>
 </SDG>
 </SDGS>
 </ADMIN-DATA>
 <CAN-CLUSTER-VARIANTS>
 <CAN-CLUSTER-CONDITIONAL>
 <BAUDRATE>500000</BAUDRATE>
 <CAN-FD-BAUDRATE>2000000</CAN-FD-BAUDRATE>
 </CAN-CLUSTER-CONDITIONAL>
 </CAN-CLUSTER-VARIANTS>
 </CAN-CLUSTER>

 <CAN-CLUSTER>
 <SHORT-NAME>Cluster_CAN_2</SHORT-NAME>
 <ADMIN-DATA>
 <SDGS>
 <SDG GID="ClusterVersion">
 <SD GID="Version">V2.3.4</SD>
 </SDG>
 </SDGS>
 </ADMIN-DATA>
 <CAN-CLUSTER-VARIANTS>
 <CAN-CLUSTER-CONDITIONAL>
 <BAUDRATE>250000</BAUDRATE>
 </CAN-CLUSTER-CONDITIONAL>
 </CAN-CLUSTER-VARIANTS>
 </CAN-CLUSTER>

 <ETHERNET-CLUSTER>
 <SHORT-NAME>Cluster_ETH</SHORT-NAME>
 </ETHERNET-CLUSTER>
 </ELEMENTS>
 </AR-PACKAGE>
 </AR-PACKAGES>
 </AUTOSAR>
 "#;

        // Scriviamo il file XML temporaneo
        let mut path = temp_dir();
        path.push("test_clusters.arxml");

        let mut file = File::create(&path).expect("Failed to create temp test file");
        file.write_all(xml.as_bytes())
            .expect("Failed to write test XML");

        // Chiamiamo la funzione da testare
        let result = parse_from_file(path.to_str().unwrap());

        assert!(result.is_ok(), "Parse failed: {:?}", result);

        let databases = result.unwrap();
        assert_eq!(databases.len(), 2); // Deve ignorare il cluster Ethernet

        // Primo cluster CAN-FD
        let db1 = &databases[0];
        assert_eq!(db1.name, "Cluster_CAN_1");
        assert_eq!(db1.version, "V1.0.0");
        assert_eq!(db1.bustype, BusType::CanFd);
        assert_eq!(db1.baudrate, 500000);
        assert_eq!(db1.baudrate_canfd, 2000000);

        // Second CAN base cluster
        let db2 = &databases[1];
        assert_eq!(db2.name, "Cluster_CAN_2");
        assert_eq!(db2.version, "V2.3.4");
        assert_eq!(db2.bustype, BusType::Can);
        assert_eq!(db2.baudrate, 250000);
        assert_eq!(db2.baudrate_canfd, 0);
    }
}
