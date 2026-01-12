use crate::types::{
    attributes::AttributeValue,
    database::{CanDatabase, CanMessageKey, CanNodeKey, CanSignalKey},
    message::{MuxRole, MuxSelector},
    node::CanNode,
};
use std::cmp::Ordering;
use std::{collections::BTreeMap, fmt};

/// Elementary step for extracting a bit field from a payload.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
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
#[derive(Default, Clone, PartialEq)]
pub struct CanSignal {
    /// Parent message key.
    pub message: CanMessageKey,
    /// Signal name.
    pub name: String,
    /// Bit start in the payload (bit 0 = LSB of the first byte).
    pub bit_start: u16,
    /// Bit length.
    pub bit_length: u16,
    /// Endianness.
    pub endian: Endianness,
    /// Sign.
    pub sign: Signess,
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
    pub receiver_nodes: Vec<CanNodeKey>,
    /// Associated comment (DBC `CM_ SG_` section).
    pub comment: String,
    /// Value-to-text mapping (value table).
    pub value_table: BTreeMap<i32, String>,
    // Precomputed extraction steps for fast decoding.
    pub(crate) steps: Vec<Step>,
    /// Multiplexing role (`MuxRole::None` when unused).
    pub mux_role: MuxRole,
    /// Optional group index (extended multiplexing). `0` if unused.
    pub mux_group: u8,
    /// For multiplexed signals, the controlling multiplexer switch.
    pub mux_switch: Option<CanSignalKey>,
    /// Selector for the multiplexer switch value/range (meaningful when multiplexed).
    pub mux_selector: MuxSelector,

    // --- Signal Attribute Entry ---
    pub attributes: BTreeMap<String, AttributeValue>,

    /// Raw time series of `[timestamp, raw]` pairs (timestamp in seconds).
    pub raws: Vec<(f64, i64)>,

    /// Value time series of `[timestamp, value]` pairs (timestamp in seconds).
    /// value: raw * sig.factor + sig.offset;
    pub values: Vec<(f64, f64)>,
}

impl CanSignal {
    const TIMESTAMP_MATCH_EPSILON: f64 = 1e-3;

    #[inline]
    fn sample_at_timestamp<T: Copy>(series: &[(f64, T)], timestamp: f64) -> Option<T> {
        if !timestamp.is_finite() {
            return None;
        }
        series.iter().find_map(|(ts, value)| {
            if ts.is_finite() && (*ts - timestamp).abs() <= Self::TIMESTAMP_MATCH_EPSILON {
                Some(*value)
            } else {
                None
            }
        })
    }

    #[inline]
    fn sample_at_timestamp_relaxed<T: Copy>(series: &[(f64, T)], timestamp: f64) -> Option<T> {
        if !timestamp.is_finite() {
            return None;
        }

        series
            .iter()
            // tieni solo i timestamp finiti
            .filter(|(ts, _)| ts.is_finite())
            // trova quello con distanza minima dal timestamp richiesto
            .min_by(|(ts_a, _), (ts_b, _)| {
                let da: f64 = (*ts_a - timestamp).abs();
                let db: f64 = (*ts_b - timestamp).abs();
                da.partial_cmp(&db).unwrap_or(Ordering::Equal)
            })
            // prendi solo il value
            .map(|(_, value)| *value)
    }

    /// Returns the stored raw value that matches the provided timestamp.
    pub fn raw_value_at(&self, timestamp: f64) -> Option<i64> {
        Self::sample_at_timestamp(&self.raws, timestamp)
    }

    /// Returns the stored physical value that matches the provided timestamp.
    pub fn value_at(&self, timestamp: f64) -> Option<f64> {
        Self::sample_at_timestamp(&self.values, timestamp)
    }

    /// Returns the stored relaxed raw value nearest to the provided timestamp.
    pub fn raw_value_at_relaxed(&self, timestamp: f64) -> Option<i64> {
        Self::sample_at_timestamp_relaxed(&self.raws, timestamp)
    }

    /// Returns the stored relaxed physical value nearest to the provided timestamp.
    pub fn value_at_relaxed(&self, timestamp: f64) -> Option<f64> {
        Self::sample_at_timestamp_relaxed(&self.values, timestamp)
    }

    /// Returns an immutable reference to a receiver node by name (case-insensitive).
    pub fn get_receiver_nodes_by_name<'a>(
        &self,
        db: &'a CanDatabase,
        name: &str,
    ) -> Option<&'a CanNode> {
        let key = name.to_ascii_lowercase();
        self.receiver_nodes
            .iter()
            .filter_map(|&node_key| db.get_node_by_key(node_key))
            .find(|node| node.name.to_ascii_lowercase() == key)
    }

    /// Returns a mutable reference to a receiver node by name (case-insensitive).
    pub fn get_receiver_nodes_by_name_mut<'a>(
        &self,
        db: &'a mut CanDatabase,
        name: &str,
    ) -> Option<&'a mut CanNode> {
        let input_name: String = name.to_ascii_lowercase();
        let nkey = self.receiver_nodes.iter().copied().find(|&node_key| {
            db.get_node_by_key(node_key)
                .map(|n| n.name.to_ascii_lowercase() == input_name)
                .unwrap_or(false)
        })?;
        db.get_node_by_key_mut(nkey)
    }

    /// Precomputes bit → value extraction steps to speed up decoding.
    ///
    /// The compilation is idempotent: subsequent calls exit early once steps
    /// are already available.
    pub fn compile_inline(&mut self) {
        if !self.steps.is_empty() {
            return;
        }
        // ceil((bit_len + (bit_start % 8)) / 8)
        let n_steps: usize = (self.bit_length as usize + (self.bit_start as usize & 7))
            .div_ceil(8)
            .max(1);
        self.steps.reserve_exact(n_steps);

        if matches!(self.endian, Endianness::Intel) {
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
                if st.dst_lsb >= 64 {
                    continue; // non possiamo rappresentare più di 64 bit
                }
                let bits_left: u16 = 64 - st.dst_lsb;
                let take: u8 = st.width.min(bits_left as u8);
                if take == 0 {
                    continue;
                }
                let mask: u8 = if take == 8 {
                    0xFF
                } else {
                    ((1u16 << take) - 1) as u8
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
        if matches!(self.sign, Signess::Signed) && n > 0 {
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

    // Note: signal-to-frame conversion is implemented in `asc::core::signal_conversion`.

    /// Resets all fields to their default values.
    pub fn clear(&mut self) {
        *self = CanSignal::default();
    }
}

/// Byte order used to interpret signal bits inside a CAN frame.
#[derive(Default, Clone, PartialEq, Debug)]
pub enum Endianness {
    #[default]
    Motorola, // 0
    Intel, // 1
}

impl fmt::Display for Endianness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Endianness::Motorola => f.write_str("Motorola"),
            Endianness::Intel => f.write_str("Intel"),
        }
    }
}

/// Sign/encoding of the signal raw value.
#[derive(Default, Clone, PartialEq, Debug)]
pub enum Signess {
    #[default]
    Unsigned, // -
    Signed,     // +
    IeeeFloat,  // SIG_VALTYPE = 1
    IeeeDouble, // SIG_VALTYPE = 1
}

impl fmt::Display for Signess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Signess::Unsigned => f.write_str("Unsigned"),
            Signess::Signed => f.write_str("Signed"),
            Signess::IeeeFloat => f.write_str("IEEE Float"),
            Signess::IeeeDouble => f.write_str("IEEE Double"),
        }
    }
}
