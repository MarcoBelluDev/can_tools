use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct SigLog {
    pub name: String,
    pub bit_start: usize,
    pub bit_length: usize,
    /// Endianness:
    /// - `1` = little-endian (Intel)
    /// - `0` = big-endian (Motorola)
    pub endian: usize,
    pub sign: usize,
    pub factor: f64,
    pub offset: f64,
    pub min: f64,
    pub max: f64,
    pub value_table: HashMap<i32, String>,
    pub values: Vec<[f64; 2]>, // couple value / timestamp
}

impl SigLog {
    /// Clears all metadata from this `SigLog`.
    ///
    /// This method resets string fields to empty strings 
    pub fn clear(&mut self) {
        self.name.clear();
        self.bit_start = 0;
        self.bit_length = 0;
        self.endian = 0;
        self.sign = 0;
        self.factor = 0.0;
        self.offset = 0.0;
        self.min = 0.0;
        self.max = 0.0;
        self.value_table = HashMap::default();
        self.values = Vec::default();
    }
}