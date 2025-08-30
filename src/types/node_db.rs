use crate::{MessageKey, Present, SignalKey};

/// Node/ECU defined in the database.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct NodeDB {
    /// Node/ECU name.
    pub name: String,
    /// Associated comment
    pub comment: String,
    /// Messages transmitted by this node.
    pub messages_sent: Vec<MessageKey>,
    /// Signals read by this node
    pub signals_sent: Vec<SignalKey>,
    /// Signals read by this node
    pub signals_read: Vec<SignalKey>,

    // --- Canoe parameter ---
    pub node_layer_modules: String,

    // --- Canoe CAPL-Generator parameters ---
    pub gen_nod_auto_gen_dsp: Present,
    pub gen_nod_auto_gen_snd: Present,
    pub gen_nod_sleep_time: u16,

    // --- Interactive Layer parameter ---
    pub int_layer_used: Present,

    // --- Network Managment parameters ---
    pub nm_node: Present,
    pub nm_station_address: u32,

    // --- Other parameters ---
    pub ecu_variant_default: Present,
    pub ecu_variant_group: String,
    pub nmh_node: Present,
    pub sample_point_canfd_max: u8,
    pub sample_point_canfd_min: u8,
    pub sample_point_max: u8,
    pub sample_point_min: u8,
    pub ssp_offset_canfd_max: u8,
    pub ssp_offset_canfd_min: u8,
    pub sync_jump_width_canfd_max: u8,
    pub sync_jump_width_canfd_min: u8,
    pub sync_jump_width_max: u8,
    pub sync_jump_width_min: u8,
    pub time_quanta_canfd_max: u8,
    pub time_quanta_canfd_min: u8,
    pub time_quanta_max: u8,
    pub time_quanta_min: u8,
    pub vag_tp20_target_address: u32,
}

impl NodeDB {
    /// Resets all fields to their default values.
    /// Clear the database
    pub fn clear(&mut self) {
        *self = NodeDB::default();
    }
}
