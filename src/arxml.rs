use std::io;

use autosar_data::{AutosarModel, CharacterData, Element, ElementName, EnumItem};

use crate::types::{
    database::{BusType, DatabaseDBC, MessageKey, NodeKey},
    errors::{ArxmlConvertError, DatabaseError},
    message::MuxRole,
    signal::{Endianness, Signess},
};

/// Estrae uno o più [`DatabaseDBC`] da un file `.arxml` seguendo le definizioni dei vari
/// `CAN-CLUSTER` presenti. Ogni cluster viene trasformato in un database separato,
/// popolando messaggi, segnali e nodi noti (dai frame port).
pub fn from_arxml_to_dbc(path: &str) -> Result<Vec<DatabaseDBC>, ArxmlConvertError> {
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
            source: io::Error::new(io::ErrorKind::Other, source),
        })?;

    let mut databases: Vec<DatabaseDBC> = Vec::new();

    for element in model
        .identifiable_elements()
        .filter_map(|(_, weak)| weak.upgrade())
    {
        if element.element_name() == ElementName::CanCluster {
            if let Some(db) = build_can_database(&element) {
                databases.push(db);
            }
        }
    }

    Ok(databases)
}

/// Converte un singolo `CAN-CLUSTER` in un [`DatabaseDBC`].
fn build_can_database(cluster: &Element) -> Option<DatabaseDBC> {
    let mut db: DatabaseDBC = DatabaseDBC::default();
    db.name = cluster.item_name().unwrap_or_default();

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
            if let Some(frame_triggerings) = phys_channel.get_sub_element(ElementName::FrameTriggerings) {
                for ft in frame_triggerings.sub_elements() {
                    process_can_frame_triggering(&mut db, &ft);
                }
            }
        }
    }

    Some(db)
}

/// Estrae messaggio, segnali e relazioni da un `<CAN-FRAME-TRIGGERING>`.
fn process_can_frame_triggering(db: &mut DatabaseDBC, frame_triggering: &Element) {
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

    let msg_key: MessageKey = ensure_message(db, &frame_name, can_id, byte_length);

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
    db: &mut DatabaseDBC,
    msg_key: MessageKey,
    pdu: &Element,
    receiver_ecus: &[String],
) {
    if pdu.element_name() == ElementName::ISignalIPdu {
        process_isignal_ipdu(db, msg_key, pdu, receiver_ecus);
    }
}

fn process_isignal_ipdu(
    db: &mut DatabaseDBC,
    msg_key: MessageKey,
    pdu: &Element,
    receiver_ecus: &[String],
) {
    let Some(mappings) = pdu.get_sub_element(ElementName::ISignalToPduMappings) else {
        return;
    };

    for mapping in mappings.sub_elements() {
        let Some(signal_elem) = mapping
            .get_sub_element(ElementName::ISignalRef)
            .and_then(|elem| elem.get_reference_target().ok()) else {
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

        let sig_key = db.add_signal(
            &sig_name,
            endian,
            Signess::Unsigned,
            1.0,
            0.0,
            min,
            max,
            "",
        );
        if let Some(signal) = db.get_sig_by_key_mut(sig_key) {
            signal.bit_start = bit_start;
            signal.bit_length = bit_length;
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

fn ensure_node(db: &mut DatabaseDBC, name: &str) -> Option<NodeKey> {
    if let Some(nk) = db.get_node_key_by_name(name) {
        return Some(nk);
    }
    match db.add_node(name) {
        Ok(nk) => Some(nk),
        Err(DatabaseError::NodeAlreadyExists { .. }) => db.get_node_key_by_name(name),
        Err(_) => None,
    }
}

fn ensure_message(db: &mut DatabaseDBC, name: &str, id: u32, dlc: u16) -> MessageKey {
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
