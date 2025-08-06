use crate::models::signal::Signal;

// BO_ <ID> <MESSAGE_NAME> : <BYTES_LENGHT> <SENDER_NODE>
#[derive(Default, Clone, PartialEq)]
pub struct Message {
    pub id: u64,
    pub id_hex: String,
    pub name: String,
    pub byte_length: usize,
    pub sender_node: String,
    pub signals: Vec<Signal>, // SG_
    pub comment: String,      // CM_ BO_
}

impl Message {
    pub fn get_signal_by_name(&self, name: &str) -> Option<&Signal> {
        self.signals
            .iter()
            .find(|sig| sig.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn build_test_message() -> Message {
        Message {
            id: 100,
            id_hex: "0x64".into(),
            name: "Motor_01".into(),
            byte_length: 8,
            sender_node: "Motor".into(),
            signals: vec![
                Signal {
                    name: "Speed".into(),
                    bit_start: 0,
                    bit_length: 16,
                    endian: 1,
                    sign: 0,
                    factor: 1.0,
                    offset: 0.0,
                    min: 0.0,
                    max: 250.0,
                    unit_of_measurement: "km/h".into(),
                    receiver_nodes: vec![],
                    comment: "Vehicle speed".into(),
                    value_table: HashMap::new(),
                },
                Signal {
                    name: "RPM".into(),
                    bit_start: 16,
                    bit_length: 16,
                    endian: 1,
                    sign: 0,
                    factor: 0.25,
                    offset: 0.0,
                    min: 0.0,
                    max: 8000.0,
                    unit_of_measurement: "rpm".into(),
                    receiver_nodes: vec![],
                    comment: "Engine RPM".into(),
                    value_table: HashMap::new(),
                },
            ],
            comment: "Test comment".into(),
        }
    }

    #[test]
    fn test_get_signal_by_name() {
        let msg = build_test_message();

        // Ricerca esatta
        let sig = msg.get_signal_by_name("Speed");
        assert!(sig.is_some());
        assert_eq!(sig.unwrap().unit_of_measurement, "km/h");

        // Ricerca case-insensitive
        let sig_lower = msg.get_signal_by_name("rpm");
        assert!(sig_lower.is_some());
        assert_eq!(sig_lower.unwrap().unit_of_measurement, "rpm");

        // Segnale inesistente
        assert!(msg.get_signal_by_name("FuelLevel").is_none());
    }
}
