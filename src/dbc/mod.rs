//! # dbc
//!
//! `dbc` is the module to work with .dbc files

pub(crate) mod version;
pub(crate) mod basic_info;
pub(crate) mod nodes;
pub(crate) mod messages;
pub(crate) mod signals;

use crate::types::database::Database;
use crate::dbc;

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
/// # Example
/// ```no_run
/// use can_tools::dbc;
///
/// let db = dbc::parse_from_file("example.dbc").expect("Failed to parse DBC file");
/// println!("Parsed {} messages", db.messages.len());
/// ```
///
/// # Notes
/// - This function is the main entry point for converting a DBC file into a structured [`Database`].
/// - Internal parsing details are handled by [`Database`] methods and are **not** part of the public API.
/// - Parsing stops only at the end of the file; malformed lines are skipped.
///
pub fn parse_from_file(path: &str) -> Result<Database, String> {
    // check if provided file has .asc format
    if !path.ends_with(".dbc") {
        return Err(format!("Not a valid .dbc file format"));
    }

    let file: File = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    let mut reader: BufReader<File> = BufReader::new(file);

    // read raw byted
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read error: {}", e))?;

    // Decode in Windows-1252
    let (text, _, _) = WINDOWS_1252.decode(&bytes);

    // Swap german chars with utf-8 chars
    let mut text = text.into_owned();
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
        let line = lines[i].trim();

        // skip comments and empty lines
        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        if line.to_lowercase().starts_with("version") {
            dbc::version::fct(&mut db, line);
        } else if line.to_lowercase().starts_with("ba_ ") {
            dbc::basic_info::fct(&mut db, line);
        } else if line.to_lowercase().starts_with("bu_") {
            dbc::nodes::fct(&mut db, line);
        } else if line.to_lowercase().starts_with("bo_ ") {
            if line.split_whitespace().count() >= 4 {
                dbc::messages::fct(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("sg_") {
            if line.split_whitespace().count() >= 5 {
                dbc::signals::fct(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("bo_tx_bu_") {
            if line.split_whitespace().count() >= 2 {
                dbc::messages::tx_nodes(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("cm_ bo_") {
            if line.split_whitespace().count() >= 2 {
                dbc::messages::comments(&mut db, line);
            }
        } else if line.to_lowercase().starts_with("cm_ sg_") {
            let mut full_comment_line: String = line.to_string();
            while full_comment_line.matches('"').count() < 2 && i + 1 < lines.len() {
                i += 1;
                full_comment_line.push('\n');
                full_comment_line.push_str(lines[i].trim());
            }
            dbc::signals::comments(&mut db, line);
        } else if line.to_lowercase().starts_with("cm_ bu_") {
            let mut full_comment_line: String = line.to_string();
            while full_comment_line.matches('"').count() < 2 && i + 1 < lines.len() {
                i += 1;
                full_comment_line.push('\n');
                full_comment_line.push_str(lines[i].trim());
            }
            dbc::nodes::comments(&mut db, line);
        } else if line.to_lowercase().starts_with("val_") && line.split_whitespace().count() >= 3 {
            dbc::signals::value_table(&mut db, line);
        }

        i += 1;
    }

    Ok(db)
}

#[test]
fn test_parse_from_file() {
    use crate::types::signal::Signal;
    use std::collections::HashMap;

    let dbc_content = r#"
VERSION "1.0.2"

NS_ : 
	NS_DESC_
	CM_
	BA_DEF_
	BA_
	VAL_
	CAT_DEF_
	CAT_
	FILTER
	BA_DEF_DEF_
	EV_DATA_
	ENVVAR_DATA_
	SGTYPE_
	SGTYPE_VAL_
	BA_DEF_SGTYPE_
	BA_SGTYPE_
	SIG_TYPE_REF_
	VAL_TABLE_
	SIG_GROUP_
	SIG_VALTYPE_
	SIGTYPE_VALTYPE_
	BO_TX_BU_
	BA_DEF_REL_
	BA_REL_
	BA_DEF_DEF_REL_
	BU_SG_REL_
	BU_EV_REL_
	BU_BO_REL_
	SG_MUL_VAL_

BU_: Motor Infotainment Gateway

BO_ 2527679645 Motor_01: 8 Motor
 SG_ Status : 61|1@1+ (1,0) [0|1] ""  Infotainment,Gateway
 SG_ Overheat : 62|1@1+ (1,0) [0|1] ""  Gateway
 SG_ Engine_Speed : 48|8@1+ (1,0) [0|255] "km/h" Infotainment
 SG_ Failure : 63|1@1+ (1,0) [0|1] "" Infotainment,Gateway

BO_ 708 ZV_04: 8 ICAS1_X_Gateway
 SG_ UHF_FFB_SKC_anlernen : 61|1@1+ (1.0,0.0) [0.0|1] ""  Vector__XXX
 SG_ ZV_HW_Motor_Safe_Hinten : 62|1@1+ (1.0,0.0) [0.0|1] ""  Vector__XXX
 SG_ ZV_HW_Motor_Lock_HK : 63|1@1+ (1.0,0.0) [0.0|1] ""  Vector__XXX

BO_TX_BU_ 2527679645 : Backup_Motor;

CM_ BO_ 2527679645 "Funny comment about Motor_01";
CM_ SG_ 2527679645 Engine_Speed "This comment tells you everything about Engine Speed."
CM_ SG_ 2527679645 Overheat "This comment tells you everything about Overheat."
CM_ SG_ 2527679645 Status "This comment tells you everything about Motor Status."
CM_ SG_ 2527679645 Failure "This comment tells you everything about Motor Failure."
CM_ BU_ Motor "Motor ECU is really important for vehicle motion."
CM_ BU_ Gateway "Gatway ECU must forward frames between vehicle networks."

BA_ "Baudrate" 500000;
BA_ "BusType" "CAN FD";
BA_ "DBName" "TestCAN";
BA_ "BaudrateCANFD" 2000000;

VAL_ 2527679645 Status 1 "On" 0 "Off" ;
VAL_ 2527679645 Overheat 1 "Overheat failure" 0 "No Overheat" ;
VAL_ 2527679645 Engine_Speed 255 "Error";
VAL_ 2527679645 Failure 1 "Generic Failure" 0 "No Failures" ;
"#;

    // Temporaneamente salvo il file
    let tmp_path = std::env::temp_dir().join("test.dbc");
    std::fs::write(&tmp_path, dbc_content).unwrap();

    // Parsing
    let db: Database = parse_from_file(tmp_path.to_str().unwrap()).expect("Failed to parse DBC");

    // --- Database first checks ---
    assert_eq!(db.version, "1.0.2");
    assert_eq!(db.baudrate, 500000);
    assert_eq!(db.bustype, "CAN FD");
    assert_eq!(db.baudrate_canfd, 2000000);
    assert_eq!(db.name, "TestCAN");

    // --- Nodes ---
    let expected_node_names = vec!["Motor", "Infotainment", "Gateway"];
    let expected_comments = vec![
        "Motor ECU is really important for vehicle motion.",
        "",
        "Gatway ECU must forward frames between vehicle networks.",
    ];
    assert_eq!(db.nodes.len(), 3);
    for (i, expected_name) in expected_node_names.iter().enumerate() {
        assert_eq!(&db.nodes[i].name, expected_name);
    }
    for (i, expected_comment) in expected_comments.iter().enumerate() {
        assert_eq!(&db.nodes[i].comment, expected_comment);
    }

    // --- Message ---
    assert_eq!(db.messages.len(), 2);
    let msg = &db.messages[0];
    assert_eq!(msg.id, 2527679645);
    assert_eq!(msg.id_hex, "0x16A9549D");
    assert_eq!(msg.name, "Motor_01");
    assert_eq!(msg.byte_length, 8);
    assert_eq!(msg.msgtype, "CAN");
    assert_eq!(msg.sender_nodes.len(), 2);
    assert_eq!(msg.sender_nodes[0].name, "Motor");
    assert_eq!(
        msg.sender_nodes[0].comment,
        "Motor ECU is really important for vehicle motion."
    );
    assert_eq!(msg.sender_nodes[1].name, "Backup_Motor");
    assert_eq!(msg.sender_nodes[1].comment, "");
    assert_eq!(msg.comment, "Funny comment about Motor_01");
    assert_eq!(msg.signals.len(), 4);

    assert_eq!(db.messages[1].name, "ZV_04");
    assert_eq!(db.messages[1].id, 708);

    // --- closure to quickly check signals ---
    let check_signal = |sig: &Signal,
                        expected_name: &str,
                        bit_start: u16,
                        bit_length: u16,
                        endian: u8,
                        sign: u8,
                        factor: f64,
                        offset: f64,
                        min: f64,
                        max: f64,
                        unit: &str,
                        receivers: Vec<&str>,
                        expected_values: HashMap<i32, &str>,
                        expected_comment: &str| {
        assert_eq!(sig.name, expected_name);
        assert_eq!(sig.bit_start, bit_start);
        assert_eq!(sig.bit_length, bit_length);
        assert_eq!(sig.endian, endian);
        assert_eq!(sig.sign, sign);
        assert_eq!(sig.factor, factor);
        assert_eq!(sig.offset, offset);
        assert_eq!(sig.min, min);
        assert_eq!(sig.max, max);
        assert_eq!(sig.unit_of_measurement, unit);
        assert_eq!(sig.comment, expected_comment); //

        // Receivers Nodes
        let recv_names: Vec<&str> = sig.receiver_nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(recv_names, receivers);
        assert_eq!(
            msg.signals[1].receiver_nodes[0].comment,
            "Gatway ECU must forward frames between vehicle networks."
        );

        // Value table
        for (val, desc) in expected_values {
            assert_eq!(sig.value_table.get(&val).map(|s| s.as_str()), Some(desc));
        }
    };

    // Signal: Status
    check_signal(
        &msg.signals[0],
        "Status",
        61,
        1,
        1,
        0,
        1.0,
        0.0,
        0.0,
        1.0,
        "",
        vec!["Infotainment", "Gateway"],
        HashMap::from([(1, "On"), (0, "Off")]),
        "This comment tells you everything about Motor Status.",
    );

    // Signal: Overheat
    check_signal(
        &msg.signals[1],
        "Overheat",
        62,
        1,
        1,
        0,
        1.0,
        0.0,
        0.0,
        1.0,
        "",
        vec!["Gateway"],
        HashMap::from([(1, "Overheat failure"), (0, "No Overheat")]),
        "This comment tells you everything about Overheat.",
    );

    // Signal: Engine_Speed
    check_signal(
        &msg.signals[2],
        "Engine_Speed",
        48,
        8,
        1,
        0,
        1.0,
        0.0,
        0.0,
        255.0,
        "km/h",
        vec!["Infotainment"],
        HashMap::from([(255, "Error")]),
        "This comment tells you everything about Engine Speed.",
    );

    // Signal: Failure
    check_signal(
        &msg.signals[3],
        "Failure",
        63,
        1,
        1,
        0,
        1.0,
        0.0,
        0.0,
        1.0,
        "",
        vec!["Infotainment", "Gateway"],
        HashMap::from([(1, "Generic Failure"), (0, "No Failures")]),
        "This comment tells you everything about Motor Failure.",
    );

    assert_eq!(db.messages[1].signals[0].name, "UHF_FFB_SKC_anlernen");
    assert_eq!(db.messages[1].signals[2].name, "ZV_HW_Motor_Lock_HK");

    let mut keys: Vec<_> = db.messages[1].signals[2]
        .value_table
        .keys()
        .cloned()
        .collect();
    keys.sort(); // ordina per valore numerico
    let expected_values = ["Schloss_nicht_ansteuern", "Schloss_ansteuern"];
    for (count, key) in keys.iter().enumerate() {
        if let Some(value) = db.messages[1].signals[2].value_table.get(&key) {
            assert_eq!(value, expected_values[count]);
        }
    }
}
