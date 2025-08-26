use crate::{Database, MessageKey, NodeDB, NodeKey, SignalLog, MuxInfo};
use std::collections::HashMap;

/// Elementary step for extracting a bit field from a payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Step {
    /// Source byte index.
    pub(crate) byte_index: u8,
    /// LSB within the source byte (0..7).
    pub(crate) src_lsb: u8,
    /// Number of bits to take (1..8).
    pub(crate) width: u8,
    /// Destination LSB in the final value (LSB-first).
    pub(crate) dst_lsb: u16,
}

/// Definition of a signal within a CAN message (DBC).
///
/// Describes position/bit-length, endianness, sign, scaling (factor/offset),
/// valid range, unit of measure, value tables, and receiver nodes.
#[derive(Default, Clone, PartialEq, Debug)]
pub struct SignalDB {
    /// Parent message key.
    pub message: MessageKey,
    /// Signal name.
    pub name: String,
    /// Bit start in the payload (bit 0 = LSB of the first byte).
    pub bit_start: u16,
    /// Bit length.
    pub bit_length: u16,
    /// Endianness: `1` = little-endian (Intel), `0` = big-endian (Motorola).
    pub endian: u8,
    /// Sign: `1` = signed, `0` = unsigned.
    pub sign: u8,
    /// Scaling factor.
    pub factor: f64,
    /// Scaling offset.
    pub offset: f64,
    /// Minimum physical value.
    pub min: f64,
    /// Maximum physical value.
    pub max: f64,
    /// Unit of measure (normalized elsewhere by removing the optional `"Unit_"` prefix).
    pub unit_of_measurement: String,
    /// Receiver nodes.
    pub receiver_nodes: Vec<NodeKey>,
    /// Associated comment (DBC `CM_ SG_` section).
    pub comment: String,
    /// Value-to-text mapping (value table).
    pub value_table: HashMap<i32, String>,
    // Precomputed extraction steps for fast decoding.
    pub(crate) steps: Vec<Step>,
    /// Multiplexing metadata (None if not multiplexed).
    pub mux: Option<MuxInfo>,
}

impl SignalDB {
    /// Returns an immutable reference to a receiver node by name (case-insensitive).
    pub fn get_receiver_nodes_by_name<'a>(
        &self,
        db: &'a Database,
        name: &str,
    ) -> Option<&'a NodeDB> {
        let key = name.to_lowercase();
        self.receiver_nodes
            .iter()
            .filter_map(|&node_key| db.get_node_by_key(node_key))
            .find(|node| node.name.to_lowercase() == key)
    }

    /// Returns a mutable reference to a receiver node by name (case-insensitive).
    pub fn get_receiver_nodes_by_name_mut<'a>(
        &self,
        db: &'a mut Database,
        name: &str,
    ) -> Option<&'a mut NodeDB> {
        let input_name: String = name.to_lowercase();
        let nkey = self.receiver_nodes.iter().copied().find(|&node_key| {
            db.get_node_by_key(node_key)
                .map(|n| n.name.to_lowercase() == input_name)
                .unwrap_or(false)
        })?;
        db.get_node_by_key_mut(nkey)
    }

    /// Precomputes bit → value extraction steps to speed up decoding.
    pub fn compile_inline(&mut self) {
        if !self.steps.is_empty() {
            return;
        }
        // ceil((bit_len + (bit_start % 8)) / 8)
        let n_steps: usize = (self.bit_length as usize + (self.bit_start as usize & 7))
            .div_ceil(8)
            .max(1);
        self.steps.reserve_exact(n_steps);

        if self.endian == 1 {
            self.compile_intel();
        } else {
            self.compile_motorola();
        }
    }

    #[inline]
    fn push_step(&mut self, st: Step) {
        self.steps.push(st);
    }

    /// Step compilation for little-endian (Intel) signals.
    fn compile_intel(&mut self) {
        let mut remaining: u16 = self.bit_length;
        let mut bit: u16 = self.bit_start;
        let mut dst: u16 = 0u16;

        while remaining > 0 {
            let byte_idx: u8 = (bit / 8) as u8;
            let bit_off: u8 = (bit % 8) as u8;
            let avail: u8 = 8 - bit_off;
            let take: u8 = remaining.min(avail as u16) as u8;

            self.push_step(Step {
                byte_index: byte_idx,
                src_lsb: bit_off,
                width: take,
                dst_lsb: dst,
            });

            bit += take as u16;
            dst += take as u16;
            remaining -= take as u16;
        }
    }

    /// Step compilation for big-endian (Motorola) signals.
    fn compile_motorola(&mut self) {
        // In DBC, @0: the start bit is the MSB of the signal; we advance MSB-first.
        let mut remaining: u16 = self.bit_length;
        let mut byte: usize = (self.bit_start / 8) as usize;
        let mut bit_msb: u8 = 7 - (self.bit_start % 8) as u8;

        while remaining > 0 {
            let can_take: u16 = (bit_msb as u16 + 1).min(remaining);
            let src_lsb: u8 = bit_msb + 1 - can_take as u8;
            let dst_lsb: u16 = remaining - can_take;

            self.push_step(Step {
                byte_index: byte as u8,
                src_lsb,
                width: can_take as u8,
                dst_lsb,
            });

            remaining -= can_take;
            if src_lsb == 0 {
                byte += 1;
                bit_msb = 7;
            } else {
                bit_msb = src_lsb - 1;
            }
        }
    }

    /// Extracts the **unsigned** raw value (LSB-first accumulation) from the payload.
    #[inline]
    pub fn extract_raw_u64(&self, bytes: &[u8]) -> u64 {
        let mut out: u64 = 0;
        for st in &self.steps {
            if let Some(&b) = bytes.get(st.byte_index as usize) {
                let mask: u8 = if st.width == 8 {
                    0xFF
                } else {
                    ((1u16 << st.width) - 1) as u8
                };
                let chunk = ((b >> st.src_lsb) & mask) as u64;
                out |= chunk << st.dst_lsb;
            }
        }
        out
    }

    /// Extracts the **signed** raw value from the payload, performing sign extension if needed.
    #[inline]
    pub fn extract_raw_i64(&self, bytes: &[u8]) -> i64 {
        let raw_u: u64 = self.extract_raw_u64(bytes);
        let n: u16 = self.bit_length.min(64);
        if self.sign == 1 && n > 0 {
            let sign_bit = 1u64 << (n - 1);
            if (raw_u & sign_bit) != 0 {
                let mask = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
                (raw_u | !mask) as i64
            } else {
                raw_u as i64
            }
        } else {
            raw_u as i64
        }
    }

    /// Converts a raw value into an "instantaneous" `SignalLog` with physical value, text, and metadata.
    ///
    /// *Note*: the unit is normalized by removing an optional `"Unit_"` prefix.
    #[inline]
    pub fn to_sigframe(&self, raw_i: i64) -> SignalLog {
        let value: f64 = (raw_i as f64) * self.factor + self.offset;
        let text: String = self
            .value_table
            .get(&(raw_i as i32))
            .cloned()
            .unwrap_or_default();
        SignalLog {
            message: 0,
            name: self.name.clone(),
            factor: self.factor,
            offset: self.offset,
            channel: 0,
            raw: raw_i,
            value,
            unit: self
                .unit_of_measurement
                .strip_prefix("Unit_")
                .unwrap_or(&self.unit_of_measurement)
                .to_string(),
            text,
            comment: self.comment.clone(),
            value_table: self.value_table.clone(),
            values: Vec::new(),
        }
    }
}
