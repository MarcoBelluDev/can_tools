use std::collections::HashMap;

use crate::models::node::Node;

// SG_ <name> : <bit_start>|<bit_lengths>@<endianness><signedness> (<scale>,<factor>) [<min>|<max>] "<units>" <receiver nodes...>
#[derive(Default, Clone)]
pub struct Signal {
    pub name: String,
    pub bit_start: usize,
    pub bit_length: usize,
    pub endian: usize, // 1 = little endian (Intel), 0 = big endian (Motorola)
    pub sign: usize,
    pub factor: f64,
    pub offset: f64,
    pub min: f64,
    pub max: f64,
    pub unit_of_measurement: String,
    pub receiver_nodes: Vec<Node>,
    pub comment: String,                   // CM_ SG_
    pub value_table: HashMap<i32, String>, // VAL_ <message_id> <signal_name> <val1> "<descr1>" <val2> "<descr2>" ... ;
}
