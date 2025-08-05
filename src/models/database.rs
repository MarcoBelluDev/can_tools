use std::collections::HashMap;

use crate::models::message::Message;
use crate::models::node::Node;
use crate::models::signal::Signal;

#[derive(Default, Clone)]
pub struct Database {
    pub version: String,        // VERSION
    pub bit_timing: String,     // BS_
    pub nodes: Vec<Node>,       // BU_
    pub messages: Vec<Message>, // BO_
}

impl Database {
    pub fn parse_version(&mut self, line: &str) {
        // Example: VERSION "1.0"
        self.version = line
            .replace("version", "") // delete version text
            .trim() // delete whitespaces
            .trim_matches('"') // delete "
            .to_string() // convert in string
    }

    pub fn parse_bit_timing(&mut self, line: &str) {
        // Example: BS_: 125000

        self.bit_timing = line
            .replace("bs_", "") // delete "bs_"
            .trim() // delete whitespaces
            .to_string();
    }

    pub fn parse_nodes(&mut self, line: &str) {
        // Example: BU_: ECU1 ECU2 ECU3 ECU4 etc...

        // Split the lines in part dividere by whitespaces
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Skip "BU_:"
        for part in parts.iter().skip(1) {
            self.nodes.push(Node {
                name: part.to_string(),
            });
        }
    }

    pub fn parse_messages(&mut self, line: &str) {
        // BO_ <ID> <MESSAGE_NAME> : <BYTES_LENGHT> <SENDER_NODE>
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 5 {
            // Too short line are not considered.
            return;
        }

        let id: u32 = parts[1].parse::<u32>().unwrap_or(0);  // decimal id
        let id_hex: String = format!("0x{:X}", id);  // hexadecimal id
        let name: String = parts[2].trim_end_matches(':').to_string();
        let byte_length: usize = parts[3].parse::<usize>().unwrap_or(0);
        let sender_node: String = parts[4].to_string();

        let msg: Message = Message {
            id,
            id_hex,
            name,
            byte_length,
            sender_node,
            signals: Vec::new(),
            comment: String::new(),
        };

        self.messages.push(msg);
    }

    pub fn parse_signal(&mut self, line: &str) {
        if self.messages.is_empty() {
            return;
        }

        // remove whitespace at end and beginning
        let line: &str = line.trim_start();

        // Split line in two part: before ":" and after
        let mut split_colon = line.splitn(2, ':');
        let left = split_colon.next().unwrap().trim();
        let right = split_colon.next().unwrap_or("").trim();

        // Signal Name
        let name: String = left
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .to_string();

        // Bit start / length / endian / sign
        let mut right_parts = right.split_whitespace();
        let bit_info = right_parts.next().unwrap_or(""); // "63|1@1+"
        let mut bit_and_rest = bit_info.split('@');
        let bit_pos_len = bit_and_rest.next().unwrap_or(""); // "63|1"
        let endian_sign = bit_and_rest.next().unwrap_or(""); // "1+"

        let mut pos_len_parts = bit_pos_len.split('|');
        let bit_start = pos_len_parts.next().unwrap_or("0").parse::<usize>().unwrap_or(0);
        let bit_length = pos_len_parts.next().unwrap_or("0").parse::<usize>().unwrap_or(0);

        let endian = endian_sign.chars().nth(0).unwrap_or('1').to_digit(10).unwrap_or(1) as usize;
        let sign = if endian_sign.contains('-') { 1 } else { 0 };


        // Scale and offset
        let factor_offset_raw: &str = right_parts.next().unwrap_or("(1,0)").trim_matches(|c| c == '(' || c == ')');
        let mut so_parts = factor_offset_raw.split(',');
        let factor: f64 = so_parts.next().unwrap_or("1").parse::<f64>().unwrap_or(1.0);
        let offset: f64 = so_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

        // Min and max
        let min_max_raw: &str = right_parts.next().unwrap_or("[0|0]").trim_matches(|c| c == '[' || c == ']');
        let mut mm_parts = min_max_raw.split('|');
        let min: f64 = mm_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let max: f64 = mm_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

        // Measurement Unit
        let unit: String = right_parts.next().unwrap_or("").trim_matches('"').to_string();

        // Receiver nodes (possono essere separati da virgole)
        let receivers_str = right_parts.collect::<Vec<&str>>().join(" ");
        let receivers: Vec<Node> = receivers_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| Node { name: s.trim().to_string() })
            .collect();

        // Generate the Signal
        let signal: Signal = Signal {
            name,
            bit_start,
            bit_length,
            endian,
            sign,
            factor,
            offset,
            min,
            max,
            unit_of_measurement: unit,
            receiver_nodes: receivers,
            comment: String::new(),
            value_table: Default::default(),
        };

        // Add to last message
        if let Some(last_msg) = self.messages.last_mut() {
            last_msg.signals.push(signal);
        }
    }

    pub fn parse_value_table(&mut self, line: &str) {
        // remove whitespace at end and beginning
        let line: &str = line.trim_start();

        // Example: VAL_ <message_id> <signal_name> <val1> "<descr1>" <val2> "<descr2>" ... ;
        let mut parts = line.split_whitespace();

        // Skip "VAL_"
        parts.next();

        // Message ID come numero decimale
        let message_id: u32 = parts.next().unwrap_or("").parse::<u32>().unwrap_or(0);

        // Signal Name
        let signal_name: String = parts.next().unwrap_or("").to_string();

        // Rest of row contains couples: <value> "<description>"
        let mut remaining: String = parts.collect::<Vec<_>>().join(" ");

        // Remove final ";"
        remaining = remaining.trim_end_matches(';').trim().to_string();

        // Parsing couples: <value> "<description>"
        let mut value_table: HashMap<i32, String> = HashMap::new();
        let mut tokens = remaining.split('"').map(|s| s.trim());

        while let Some(before) = tokens.next() {
            let before: &str = before.trim();
            if before.is_empty() {
                continue;
            }
            if let Some(num_str) = before.split_whitespace().last() {
                if let Ok(val) = num_str.parse::<i32>() {
                    if let Some(desc) = tokens.next() {
                        value_table.insert(val, desc.to_string());
                    }
                }
            }
        }
        
        // Add value table to right message and signal
        if let Some(msg) = self.messages.iter_mut().find(|m| m.id == message_id) {
            if let Some(signal) = msg.signals.iter_mut().find(|s| s.name == signal_name) {
                signal.value_table = value_table;
            }
        }
    }
    
}
