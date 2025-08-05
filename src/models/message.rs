use crate::models::signal::Signal;

#[derive(Default, Clone)]
// BO_ <ID> <MESSAGE_NAME> : <BYTES_LENGHT> <SENDER_NODE>
pub struct Message {
    pub id: u32,
    pub id_hex: String,
    pub name: String,
    pub byte_length: usize,
    pub sender_node: String,
    pub signals: Vec<Signal>, // SG_
    pub comment: String,      // CM_ BO_
}
