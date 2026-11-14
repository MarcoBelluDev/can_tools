use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use can_tools::{
    parse, save,
    types::{
        database::CanDatabase,
        message::{MuxRole, MuxSelector},
        signal::{Endianness, Signess},
    },
};
use slotmap::Key;

type TestResult = Result<(), Box<dyn Error>>;
type ExpectedDatabase = (&'static str, usize, usize, usize);

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")
}

// Each case gets its own directory, including when Cargo runs tests in parallel.
struct TestOutput(PathBuf);

impl TestOutput {
    fn new() -> io::Result<Self> {
        static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
        loop {
            let directory = std::env::temp_dir().join(format!(
                "can_tools_examples_{}_{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&directory) {
                Ok(()) => return Ok(Self(directory)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }
}

impl Drop for TestOutput {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, PartialEq)]
struct MessageSnapshot {
    name: String,
    byte_length: u16,
    senders: BTreeSet<String>,
}

#[derive(Debug, PartialEq)]
struct SignalSnapshot {
    bit_start: u16,
    bit_length: u16,
    endian: Endianness,
    sign: Signess,
    mux_role: MuxRole,
    mux_selector: MuxSelector,
    receivers: BTreeSet<String>,
}

#[derive(Debug, PartialEq)]
struct DatabaseSnapshot {
    messages: BTreeMap<u32, MessageSnapshot>,
    signals: BTreeMap<(u32, String), SignalSnapshot>,
}

fn snapshot(db: &CanDatabase) -> DatabaseSnapshot {
    let messages = db
        .iter_messages()
        .map(|message| {
            (
                message.id,
                MessageSnapshot {
                    name: message.name.clone(),
                    byte_length: message.byte_length,
                    senders: message
                        .sender_nodes
                        .iter()
                        .map(|&key| db.get_node_by_key(key).unwrap().name.clone())
                        .collect(),
                },
            )
        })
        .collect();
    let mut signals = BTreeMap::new();
    for message in db.iter_messages() {
        for signal in message.signals(db) {
            let previous = signals.insert(
                (message.id, signal.name.clone()),
                SignalSnapshot {
                    bit_start: signal.bit_start,
                    bit_length: signal.bit_length,
                    endian: signal.endian.clone(),
                    sign: signal.sign.clone(),
                    mux_role: signal.mux_role,
                    mux_selector: signal.mux_selector.clone(),
                    receivers: signal
                        .receiver_nodes
                        .iter()
                        .map(|&key| db.get_node_by_key(key).unwrap().name.clone())
                        .collect(),
                },
            );
            assert!(previous.is_none(), "duplicate signal in {}", message.name);
        }
    }
    DatabaseSnapshot { messages, signals }
}

fn verify_structure_and_decoding(db: &CanDatabase, context: &str) {
    assert_eq!(db.iter_nodes().count(), db.nodes.len(), "{context}");
    assert_eq!(db.iter_messages().count(), db.messages.len(), "{context}");
    assert_eq!(db.iter_signals().count(), db.signals.len(), "{context}");
    assert_eq!(
        db.signals_order.iter().collect::<HashSet<_>>().len(),
        db.signals.len(),
        "{context}: duplicate signal order keys"
    );

    for (key, node) in &db.nodes {
        assert_eq!(db.get_node_key_by_name(&node.name), Some(key), "{context}");
    }
    for (signal_key, signal) in &db.signals {
        assert!(
            !signal.message.is_null(),
            "{context}: orphan {}",
            signal.name
        );
        let message = db.get_message_by_key(signal.message).unwrap();
        assert!(
            message.signals.contains(&signal_key),
            "{context}: missing back-reference for {}",
            signal.name
        );
    }

    let mut checked = 0;
    for (message_key, message) in &db.messages {
        assert_eq!(
            db.get_msg_key_by_id(message.id),
            Some(message_key),
            "{context}"
        );
        assert_eq!(
            db.get_msg_key_by_name(&message.name),
            Some(message_key),
            "{context}"
        );
        for &signal_key in &message.signals {
            let signal = db.get_sig_by_key(signal_key).unwrap();
            assert_eq!(signal.message, message_key, "{context}: {}", signal.name);
            for &node_key in &signal.receiver_nodes {
                assert!(
                    db.get_node_by_key(node_key)
                        .unwrap()
                        .rx_signals
                        .contains(&signal_key)
                );
            }
            for &node_key in &message.sender_nodes {
                let node = db.get_node_by_key(node_key).unwrap();
                assert!(node.messages_sent.contains(&message_key), "{context}");
                assert!(node.tx_signals.contains(&signal_key), "{context}");
            }

            // These fixtures currently only exercise Intel layouts. Wider signals
            // cannot be represented by the library's u64/i64 extraction API.
            if signal.endian != Endianness::Intel || !(1..=64).contains(&signal.bit_length) {
                continue;
            }
            assert!(
                usize::from(signal.bit_start) + usize::from(signal.bit_length)
                    <= usize::from(message.byte_length) * 8,
                "{context}: {} / {} exceeds the payload",
                message.name,
                signal.name
            );
            for pattern in 0..4 {
                let payload: Vec<u8> = (0..message.byte_length)
                    .map(|index| match pattern {
                        0 => 0,
                        1 => 255,
                        2 => (index as u8).wrapping_mul(73).wrapping_add(0xA5),
                        _ => (index as u8).wrapping_mul(29).wrapping_add(0x3C),
                    })
                    .collect();
                // Independent reference: read individual bits, without compiled
                // extraction steps or any other library decoding helper.
                let expected = (0..signal.bit_length).fold(0u64, |value, bit| {
                    let position = usize::from(signal.bit_start) + usize::from(bit);
                    let source_bit = (payload[position / 8] >> (position % 8)) & 1;
                    value | (u64::from(source_bit) << bit)
                });
                let expected_signed = if signal.sign == Signess::Signed
                    && expected & (1u64 << (signal.bit_length - 1)) != 0
                {
                    (i128::from(expected) - (1i128 << signal.bit_length)) as i64
                } else {
                    expected as i64
                };
                assert_eq!(
                    signal.extract_raw_u64(&payload),
                    expected,
                    "{context}: {} / {}, pattern {pattern}",
                    message.name,
                    signal.name
                );
                assert_eq!(
                    signal.extract_raw_i64(&payload),
                    expected_signed,
                    "{context}: {} / {}, pattern {pattern}",
                    message.name,
                    signal.name
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 0, "{context}: no supported signals checked");
}

fn verify_fixture(filename: &str, expected: &[ExpectedDatabase]) -> TestResult {
    let input = examples_dir().join(filename);
    let path = input.to_str().unwrap();
    let databases = if filename.to_ascii_lowercase().ends_with(".dbc") {
        vec![parse::from_dbc_file(path)?]
    } else {
        parse::from_arxml_file(path)?
    };
    assert_eq!(databases.len(), expected.len(), "{filename}: cluster count");
    let output = TestOutput::new()?;
    let mut seen = HashSet::new();

    for (index, mut db) in databases.into_iter().enumerate() {
        let context = format!("{filename}: {}", db.name);
        assert!(seen.insert(db.name.clone()), "{context}: duplicate cluster");
        let &(_, nodes, messages, signals) = expected
            .iter()
            .find(|entry| entry.0 == db.name)
            .unwrap_or_else(|| panic!("{context}: unexpected cluster"));
        assert_eq!(db.nodes.len(), nodes, "{context}: node count");
        assert_eq!(db.messages.len(), messages, "{context}: message count");
        assert_eq!(db.signals.len(), signals, "{context}: signal count");
        verify_structure_and_decoding(&db, &context);
        let before = snapshot(&db);

        let first_message = db.messages_order[0];
        let message_id = db.get_message_by_key(first_message).unwrap().id;
        let comment = "Modified by the can_tools examples integration test";
        db.get_message_by_key_mut(first_message).unwrap().comment = comment.into();
        let saved = output.0.join(format!("cluster_{index}.dbc"));
        save::save_to_file(saved.to_str().unwrap(), &db)?;
        let reread = parse::from_dbc_file(saved.to_str().unwrap())?;

        assert_eq!(reread.nodes.len(), nodes, "{context}: saved node count");
        assert_eq!(
            reread.messages.len(),
            messages,
            "{context}: saved message count"
        );
        assert_eq!(
            reread.signals.len(),
            signals,
            "{context}: saved signal count"
        );
        assert_eq!(
            reread.get_message_by_id(message_id).unwrap().comment,
            comment,
            "{context}: edited comment"
        );
        assert_eq!(snapshot(&reread), before, "{context}: DBC round trip");
        verify_structure_and_decoding(&reread, &format!("{context} after saving"));
    }
    Ok(())
}

// Adding a case here both registers a Cargo test and records fixture coverage.
macro_rules! example_tests {
    ($($name:ident: $file:literal => [$($expected:expr),+ $(,)?];)+) => {
        const TESTED_FILES: &[&str] = &[$($file),+];
        $(
            #[test]
            fn $name() -> TestResult {
                verify_fixture($file, &[$($expected),+])
            }
        )+
    };
}

example_tests! {
    abcan_meb: "ABCAN_MEB.dbc" => [("E3V_ABCANFD", 9, 430, 3705)];
    abcanfd: "E3_1_1_UNECE_E3V_ABCANFD_KMatrix_V15.03.01.02F_20220718.dbc" => [
        ("E3V_ABCANFD", 10, 431, 3705)
    ];
    klima_arxml: "E3_1_1_UNECE_Gen2_V15.04.06.02F_AR430_20250321_Klima.arxml" => [
        ("E3V_EVCANFD", 27, 218, 1760)
    ];
    sam_arxml: "E3_1_1_UNECE_Gen2_V15.04.06.03F_AR430_20250326_SAM.arxml" => [
        ("E3V_CCAN", 47, 83, 556),
        ("E3V_K2CANFD", 55, 189, 2391)
    ];
    hcp3_arxml: "HCP3_IO_Node.aRXml" => [
        ("WU_HCP3_CANFD", 3, 5, 109),
        ("HCP3_CANFD01", 18, 144, 775)
    ];
    ican_mlb: "ICAN_MLB.dBC" => [("MLBevo_ICAN", 13, 569, 4582)];
    mlbevo_k2can: "MLBevo_Gen2_MLBevo_K2CAN_KMatrix_V8.24.01F_20220711_VP.dbc" => [
        ("MLBevo_K2CAN", 8, 397, 3742)
    ];
    mqbw_icanhs: "MQBw_ICANHS.dbc" => [("ICANHS", 16, 498, 3296)];
}

#[test]
fn every_example_database_has_a_test() -> TestResult {
    let mut available = BTreeSet::new();
    for entry in fs::read_dir(examples_dir())? {
        let path = entry?.path();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if path.is_file()
            && (extension.eq_ignore_ascii_case("dbc") || extension.eq_ignore_ascii_case("arxml"))
        {
            available.insert(path.file_name().unwrap().to_str().unwrap().to_owned());
        }
    }
    let covered: BTreeSet<_> = TESTED_FILES
        .iter()
        .map(|filename| filename.to_string())
        .collect();
    assert_eq!(
        available, covered,
        "Update example_tests! when adding, renaming or removing an example database"
    );
    Ok(())
}
