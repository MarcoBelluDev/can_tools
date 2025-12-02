use autosar_data::{AttributeName, AutosarModel, CharacterData, Element, ElementName, EnumItem};
use std::fs::File;
use std::io::{self, BufRead, BufReader};

use encoding_rs::WINDOWS_1252;

use crate::core;
use crate::types::{
    database::{BusType, CanDatabase, CanMessageKey, CanNodeKey},
    errors::{ArxmlConvertError, DatabaseError, DbcParseError},
    message::MuxRole,
    signal::{Endianness, Signess},
};

/// Parses a DBC file and returns a populated [`CanDatabase`] instance.
///
/// This function reads a DBC file from disk, parses its content line by line,
/// and fills the [`CanDatabase`] structure with all parsed information:
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
/// - `Ok(CanDatabase)` if the file was successfully read and parsed.
/// - `Err(DbcParseError)` detailing why the file could not be opened or read.
///
/// # Errors
/// Returns an `Err(DbcParseError)` if:
/// - The file cannot be opened.
/// - There are I/O errors while reading.
/// - The path does not end in `.dbc`.
///
/// # Notes
/// - This function is the main entry point for converting a DBC file into a structured [`CanDatabase`].
/// - Internal parsing details are handled by [`CanDatabase`] methods and are **not** part of the public API.
/// - Parsing stops only at the end of the file; malformed lines are skipped.
///
pub fn from_dbc_file(path: &str) -> Result<CanDatabase, DbcParseError> {
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

    // Initialize CanDatabase
    let mut db: CanDatabase = CanDatabase::default();

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
        let (decoded, _, _) = WINDOWS_1252.decode(buf);
        let decoded_ref: &str = decoded.as_ref();
        let mut replaced: Option<String> = None;

        for (idx, ch) in decoded_ref.char_indices() {
            match ch {
                'ü' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('u');
                }
                'ö' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('o');
                }
                'ä' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('a');
                }
                'ß' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('s');
                    buf.push('s');
                }
                'Ü' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('U');
                }
                'Ö' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('O');
                }
                'Ä' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('A');
                }
                '¿' => {
                    let buf = replaced.get_or_insert_with(|| {
                        let mut s = String::with_capacity(decoded_ref.len());
                        s.push_str(&decoded_ref[..idx]);
                        s
                    });
                    buf.push('?');
                }
                _ => {
                    if let Some(buf) = replaced.as_mut() {
                        buf.push(ch);
                    }
                }
            }
        }

        let mut line = match replaced {
            Some(s) => s,
            None => decoded.into_owned(),
        };
        // trim trailing CR/LF to behave like .lines()
        while line.ends_with(['\n', '\r']) {
            line.pop();
        }
        Ok(Some(line))
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
    CanDatabase::sort_attribute_map(&mut db.attributes);
    db.sort_db_nodes_by_name();
    db.sort_db_messages_by_name();
    db.sort_db_signals_by_name();
    db.sort_all_node_fields();
    db.sort_all_message_fields();
    db.sort_all_signal_fields();

    Ok(db)
}

/// Extracts one or more [`CanDatabase`] objects from a `.arxml` file by walking all
/// defined `CAN-CLUSTER`s. Each cluster becomes its own database, populated with
/// known messages, signals, and nodes derived from the frame ports.
pub fn from_arxml_to_dbc(path: &str) -> Result<Vec<CanDatabase>, ArxmlConvertError> {
    if !path.ends_with(".arxml") {
        return Err(ArxmlConvertError::InvalidExtension {
            path: path.to_string(),
        });
    }

    let model: AutosarModel = AutosarModel::new();
    let path_owned: String = path.to_string();

    model
        .load_file(path, false)
        .map_err(|source| ArxmlConvertError::OpenFile {
            path: path_owned.clone(),
            source: io::Error::other(source),
        })?;

    let mut databases: Vec<CanDatabase> = Vec::new();

    for element in model
        .identifiable_elements()
        .filter_map(|(_, weak)| weak.upgrade())
    {
        if element.element_name() == ElementName::CanCluster
            && let Some(mut db) = build_can_database(&element)
        {
            // re-order
            CanDatabase::sort_attribute_map(&mut db.attributes);
            db.sort_db_nodes_by_name();
            db.sort_db_messages_by_name();
            db.sort_db_signals_by_name();
            db.sort_all_node_fields();
            db.sort_all_message_fields();
            db.sort_all_signal_fields();
            databases.push(db);
        }
    }

    Ok(databases)
}

/// Converte un singolo `CAN-CLUSTER` in un [`CanDatabase`].
fn build_can_database(cluster: &Element) -> Option<CanDatabase> {
    let mut db: CanDatabase = CanDatabase {
        name: cluster.item_name().unwrap_or_default(),
        ..Default::default()
    };

    let ccc = cluster
        .get_sub_element(ElementName::CanClusterVariants)
        .and_then(|ccv| ccv.get_sub_element(ElementName::CanClusterConditional))?;

    if ccc
        .get_sub_element(ElementName::CanFdBaudrate)
        .and_then(|elem| elem.character_data())
        .is_some()
    {
        db.bustype = BusType::CanFd;
    } else {
        db.bustype = BusType::Can;
    }

    if let Some(channels) = ccc.get_sub_element(ElementName::PhysicalChannels) {
        for phys_channel in channels
            .sub_elements()
            .filter(|se| se.element_name() == ElementName::CanPhysicalChannel)
        {
            if let Some(frame_triggerings) =
                phys_channel.get_sub_element(ElementName::FrameTriggerings)
            {
                for ft in frame_triggerings.sub_elements() {
                    process_can_frame_triggering(&mut db, &ft);
                }
            }
        }
    }

    Some(db)
}

/// Estrae messaggio, segnali e relazioni da un `<CAN-FRAME-TRIGGERING>`.
fn process_can_frame_triggering(db: &mut CanDatabase, frame_triggering: &Element) {
    let frame = match frame_triggering
        .get_sub_element(ElementName::FrameRef)
        .and_then(|elem| elem.get_reference_target().ok())
    {
        Some(f) => f,
        None => return,
    };

    let frame_name: String = frame.item_name().unwrap_or_else(|| "CAN_Frame".to_string());
    let can_id: u32 = frame_triggering
        .get_sub_element(ElementName::Identifier)
        .and_then(|elem| elem.character_data())
        .and_then(|cdata| cdata.parse_integer::<u32>())
        .unwrap_or(0);
    let byte_length: u16 = frame
        .get_sub_element(ElementName::FrameLength)
        .and_then(|elem| elem.character_data())
        .and_then(|cdata| cdata.parse_integer::<u16>())
        .unwrap_or(0);

    let msg_key: CanMessageKey = ensure_message(db, &frame_name, can_id, byte_length);

    // Sender/receiver nodes
    let frame_ports: Vec<Element> = frame_triggering
        .get_sub_element(ElementName::FramePortRefs)
        .map(|elem| {
            elem.sub_elements()
                .filter(|se| se.element_name() == ElementName::FramePortRef)
                .filter_map(|fpr| fpr.get_reference_target().ok())
                .collect()
        })
        .unwrap_or_default();
    let (sender_ecus, receiver_ecus) = get_rx_tx_ecus(frame_ports);
    for ecu in sender_ecus {
        if let Some(nk) = ensure_node(db, &ecu) {
            let _ = db.add_sender_relation(msg_key, nk);
        }
    }

    // Signals mapped to this frame through its PDU mappings
    if let Some(mappings) = frame.get_sub_element(ElementName::PduToFrameMappings) {
        for pdu_mapping in mappings.sub_elements() {
            if let Some(pdu) = pdu_mapping
                .get_sub_element(ElementName::PduRef)
                .and_then(|pduref| pduref.get_reference_target().ok())
            {
                collect_isignal_mappings(db, msg_key, &pdu, &receiver_ecus);
            }
        }
    }
}

/// Converte un `<I-SIGNAL-I-PDU>` (o contenitori annidati) in segnali DBC.
fn collect_isignal_mappings(
    db: &mut CanDatabase,
    msg_key: CanMessageKey,
    pdu: &Element,
    receiver_ecus: &[String],
) {
    for native_sender in native_senders_of_pdu(pdu) {
        if let Some(nk) = ensure_node(db, &native_sender) {
            let _ = db.add_sender_relation(msg_key, nk);
        }
    }

    if pdu.element_name() == ElementName::ISignalIPdu || pdu.element_name() == ElementName::NmPdu {
        // NM-PDU condivide la stessa struttura di mapping degli I-SIGNAL-I-PDU
        process_isignal_ipdu(db, msg_key, pdu, receiver_ecus);
    } else if pdu.element_name() == ElementName::NPdu {
        process_npdu(db, msg_key, pdu);
    }
}

fn process_isignal_ipdu(
    db: &mut CanDatabase,
    msg_key: CanMessageKey,
    pdu: &Element,
    receiver_ecus: &[String],
) {
    let Some(mappings) = pdu
        .get_sub_element(ElementName::ISignalToPduMappings)
        .or_else(|| pdu.get_sub_element(ElementName::ISignalToIPduMappings))
    else {
        return;
    };

    for mapping in mappings.sub_elements() {
        let Some(signal_elem) = mapping
            .get_sub_element(ElementName::ISignalRef)
            .and_then(|elem| elem.get_reference_target().ok())
        else {
            continue;
        };

        let sig_name: String = signal_elem.item_name().unwrap_or_default();
        let bit_start: u16 = mapping
            .get_sub_element(ElementName::StartPosition)
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| cdata.parse_integer::<u16>())
            .unwrap_or(0);
        let bit_length: u16 = signal_elem
            .get_sub_element(ElementName::Length)
            .and_then(|elem| elem.character_data())
            .and_then(|cdata| cdata.parse_integer::<u16>())
            .unwrap_or(0);
        let endian: Endianness = match mapping
            .get_sub_element(ElementName::PackingByteOrder)
            .and_then(|elem| elem.character_data())
        {
            Some(CharacterData::Enum(EnumItem::MostSignificantByteFirst)) => Endianness::Motorola,
            Some(CharacterData::Enum(EnumItem::MostSignificantByteLast)) => Endianness::Intel,
            Some(CharacterData::Enum(EnumItem::Opaque)) => Endianness::Intel, // treat opaque byte order as linear/Intel for fitting check
            _ => Endianness::Motorola,
        };

        let min: f64 = 0.0;
        let mut max: f64 = 0.0;
        if bit_length > 0 {
            // intervallo massimo assumendo segnale unsigned
            let max_raw: u64 = if bit_length < 64 {
                (1u64 << bit_length) - 1
            } else {
                u64::MAX
            };
            max = max_raw as f64;
        }

        let comment: Option<String> = extract_desc(&signal_elem);

        let sig_key = db.add_signal(&sig_name, endian, Signess::Unsigned, 1.0, 0.0, min, max, "");
        if let Some(signal) = db.get_sig_by_key_mut(sig_key) {
            signal.bit_start = bit_start;
            signal.bit_length = bit_length;
            if let Some(desc) = comment {
                signal.comment = desc;
            }
            signal.steps.clear();
            signal.compile_inline();
        }

        if db
            .add_msg_sig_relation(sig_key, msg_key, MuxRole::None, None)
            .is_ok()
        {
            for ecu in receiver_ecus {
                if let Some(nk) = ensure_node(db, ecu) {
                    let _ = db.add_sig_receiver_node(sig_key, nk);
                }
            }
        }
    }
}

/// Ricava le ECU trasmettenti/riceventi dai `<FRAME-PORT-REF>`.
fn get_rx_tx_ecus(frame_ports: Vec<Element>) -> (Vec<String>, Vec<String>) {
    let mut sender_ecus = Vec::new();
    let mut receiver_ecus = Vec::new();
    for fp in frame_ports {
        if let Some(CharacterData::Enum(direction)) = fp
            .get_sub_element(ElementName::CommunicationDirection)
            .and_then(|elem| elem.character_data())
        {
            match direction {
                EnumItem::In => {
                    if let Some(name) = ecu_of_frame_port(&fp) {
                        receiver_ecus.push(name);
                    }
                }
                EnumItem::Out => {
                    if let Some(name) = ecu_of_frame_port(&fp) {
                        sender_ecus.push(name);
                    }
                }
                _ => {}
            }
        }
    }
    (sender_ecus, receiver_ecus)
}

/// Risale l'arborescenza del frame port per ottenere il nome dell'ECU.
fn ecu_of_frame_port(frame_port: &Element) -> Option<String> {
    let ecu_comm_port_instance = frame_port.parent().ok()??;
    let comm_connector = ecu_comm_port_instance.parent().ok()??;
    let connectors = comm_connector.parent().ok()??;
    let ecu_instance = connectors.parent().ok()??;
    ecu_instance.item_name()
}

fn ensure_node(db: &mut CanDatabase, name: &str) -> Option<CanNodeKey> {
    if let Some(nk) = db.get_node_key_by_name(name) {
        return Some(nk);
    }
    match db.add_node(name) {
        Ok(nk) => Some(nk),
        Err(DatabaseError::NodeAlreadyExists { .. }) => db.get_node_key_by_name(name),
        Err(_) => None,
    }
}

fn ensure_message(db: &mut CanDatabase, name: &str, id: u32, dlc: u16) -> CanMessageKey {
    if let Some(k) = db.get_msg_key_by_id(id) {
        return k;
    }
    if let Some(k) = db.get_msg_key_by_name(name) {
        return k;
    }

    match db.add_message(name, id, dlc) {
        Ok(k) => k,
        Err(_) => {
            let fallback_name = format!("{name}_{id}");
            db.add_message(&fallback_name, id, dlc)
                .expect("fallback message creation failed")
        }
    }
}

fn native_senders_of_pdu(pdu: &Element) -> Vec<String> {
    let Some(admin) = pdu.get_sub_element(ElementName::AdminData) else {
        return Vec::new();
    };
    let Some(sdgs) = admin.get_sub_element(ElementName::Sdgs) else {
        return Vec::new();
    };

    let mut senders: Vec<String> = Vec::new();

    for sdg in sdgs
        .sub_elements()
        .filter(|se| se.element_name() == ElementName::Sdg)
    {
        let gid = sdg
            .attribute_value(AttributeName::Gid)
            .and_then(text_from_cdata);
        if gid.as_deref() != Some("NativeSender") {
            continue;
        }
        for sd in sdg
            .sub_elements()
            .filter(|se| se.element_name() == ElementName::Sd)
        {
            let sd_gid = sd
                .attribute_value(AttributeName::Gid)
                .and_then(text_from_cdata);
            if sd_gid.as_deref() != Some("ECU") {
                continue;
            }
            if let Some(CharacterData::String(ecu_list)) = sd.character_data() {
                for entry in ecu_list.split(',') {
                    let trimmed = entry.trim();
                    if !trimmed.is_empty() {
                        senders.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    senders
}

fn extract_desc(elem: &Element) -> Option<String> {
    let desc = elem.get_sub_element(ElementName::Desc)?;
    let mut parts: Vec<String> = Vec::new();

    if let Some(text) = desc.character_data().and_then(text_from_cdata) {
        parts.push(text);
    }

    for child in desc.sub_elements() {
        if let Some(text) = child.character_data().and_then(text_from_cdata) {
            parts.push(text);
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn text_from_cdata(cdata: CharacterData) -> Option<String> {
    match cdata {
        CharacterData::String(s) => Some(s),
        _ => None,
    }
}

fn process_npdu(db: &mut CanDatabase, msg_key: CanMessageKey, pdu: &Element) {
    let msg_name = db
        .get_message_by_key(msg_key)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| pdu.item_name().unwrap_or_default());

    let msg_dlc: u16 = db
        .get_message_by_key(msg_key)
        .map(|m| m.byte_length)
        .unwrap_or(0);

    let pdu_len_bytes: u16 = pdu
        .get_sub_element(ElementName::Length)
        .and_then(|elem| elem.character_data())
        .and_then(|cdata| cdata.parse_integer::<u16>())
        .unwrap_or(msg_dlc);

    let byte_len: u16 = if pdu_len_bytes > 0 {
        pdu_len_bytes
    } else {
        msg_dlc
    };
    let bit_length: u16 = byte_len.saturating_mul(8);
    let max: f64 = if bit_length == 0 {
        0.0
    } else if bit_length < 64 {
        ((1u64 << bit_length) - 1) as f64
    } else {
        u64::MAX as f64
    };

    let sig_key = db.add_signal(
        &msg_name,
        Endianness::Intel,
        Signess::Unsigned,
        1.0,
        0.0,
        0.0,
        max,
        "",
    );
    if let Some(signal) = db.get_sig_by_key_mut(sig_key) {
        signal.bit_start = 0;
        signal.bit_length = bit_length;
        signal.steps.clear();
        signal.compile_inline();
    }

    let _ = db.add_msg_sig_relation(sig_key, msg_key, MuxRole::None, None);
}
