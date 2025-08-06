use crate::models::database::Database;

use std::fs::File;
use std::io::{BufRead, BufReader};

/// read file dbc and populate Database, Messages, Signals and Node structs
pub fn parse(path: &str) -> Result<Database, String> {
    let file: File = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    let reader: BufReader<File> = BufReader::new(file);

    // read and collect all the lines in the reader
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let mut db: Database = Database::default();
    let mut i: usize = 0; // row number of the .dbc file

    while i < lines.len() {
        let line = lines[i].trim();

        // skip comments and empty lines
        if line.is_empty() || line.starts_with("//") {
            i += 1;
            continue;
        }

        if line.to_lowercase().starts_with("version") {
            db.parse_version(line);
        } else if line.to_lowercase().starts_with("bs_") {
            db.parse_bit_timing(line);
        } else if line.to_lowercase().starts_with("bu_") {
            db.parse_nodes(line);
        } else if line.to_lowercase().starts_with("bo_ ") {
            if line.split_whitespace().count() >= 4 {
                db.parse_messages(line);
            }
        } else if line.to_lowercase().starts_with("sg_") {
            if line.split_whitespace().count() >= 5 {
                db.parse_signal(line);
            }
        } else if line.to_lowercase().starts_with("bo_tx_bu_") {
            if line.split_whitespace().count() >= 2 {
                db.parse_add_nodes(line);
            }
        } else if line.to_lowercase().starts_with("cm_ bo_") {
            if line.split_whitespace().count() >= 2 {
                db.parse_message_comments(line);
            }
        } else if line.to_lowercase().starts_with("cm_ sg_") {
            let mut full_comment_line = line.to_string();
            while full_comment_line.matches('"').count() < 2 && i + 1 < lines.len() {
                i += 1;
                full_comment_line.push('\n');
                full_comment_line.push_str(lines[i].trim());
            }
            db.parse_signal_comments(&full_comment_line);
        } else if line.to_lowercase().starts_with("val_") {
            if line.split_whitespace().count() >= 3 {
                db.parse_value_table(line);
            }
        }

        i += 1;
    }

    Ok(db)
}

#[test]
fn test_parse_simple_dbc_extended() {
    use crate::models::signal::Signal;
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

BS_: 125000

BU_: Motor Infotainment Gateway

BO_ 2527679645 Motor_01: 8 Motor
 SG_ Status : 61|1@1+ (1,0) [0|1] ""  Infotainment,Gateway
 SG_ Overheat : 62|1@1+ (1,0) [0|1] ""  Gateway
 SG_ Engine_Speed : 48|8@1+ (1,0) [0|255] "km/h" Infotainment
 SG_ Failure : 63|1@1+ (1,0) [0|1] "" Infotainment,Gateway

BO_TX_BU_ 2527679645 : Backup_Motor;
CM_ BO_ 2527679645 "Funny comment about Motor_01";
CM_ SG_ 2527679645 Engine_Speed "This comment tells you everything about Engine Speed."
CM_ SG_ 2527679645 Overheat "This comment tells you everything about Overheat."
CM_ SG_ 2527679645 Status "This comment tells you everything about Motor Status."
CM_ SG_ 2527679645 Failure "This comment tells you everything about Motor Failure."

VAL_ 2527679645 Status 1 "On" 0 "Off" ;
VAL_ 2527679645 Overheat 1 "Overheat failure" 0 "No Overheat" ;
VAL_ 2527679645 Engine_Speed 255 "Error";
VAL_ 2527679645 Failure 1 "Generic Failure" 0 "No Failures" ;
"#;

    // Temporaneamente salvo il file
    let tmp_path = std::env::temp_dir().join("test.dcb");
    std::fs::write(&tmp_path, dbc_content).unwrap();

    // Parsing
    let db: Database = parse(tmp_path.to_str().unwrap()).expect("Failed to parse DBC");

    // --- Controlli base ---
    assert_eq!(db.version, "1.0.2");
    assert_eq!(db.bit_timing, "125000");

    // --- Nodi ---
    let expected_nodes = vec!["Motor", "Infotainment", "Gateway"];
    assert_eq!(db.nodes.len(), expected_nodes.len());
    for (i, node) in expected_nodes.iter().enumerate() {
        assert_eq!(&db.nodes[i].name, node);
    }

    // --- Message ---
    assert_eq!(db.messages.len(), 1);
    let msg = &db.messages[0];
    assert_eq!(msg.id, 2527679645);
    assert_eq!(msg.id_hex, "0x96A9549D");
    assert_eq!(msg.name, "Motor_01");
    assert_eq!(msg.byte_length, 8);
    assert_eq!(msg.sender_nodes.len(), 2);
    assert_eq!(msg.sender_nodes[0].name, "Motor");
    assert_eq!(msg.sender_nodes[1].name, "Backup_Motor");
    assert_eq!(msg.comment, "Funny comment about Motor_01");
    assert_eq!(msg.signals.len(), 4);

    // --- closure to quickly check signals ---
    let check_signal = |sig: &Signal,
                        expected_name: &str,
                        bit_start: usize,
                        bit_length: usize,
                        endian: usize,
                        sign: usize,
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

        // Receivers
        let recv_names: Vec<&str> = sig.receiver_nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(recv_names, receivers);

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
}
