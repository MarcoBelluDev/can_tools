use crate::SigLog;

#[derive(Default, Clone)]
pub struct MsgLog {
    pub name: String,
    pub id_hex: String,
    pub signals: Vec<SigLog>,
}

impl MsgLog {
    /// Clears all metadata from this `MsgLog`.
    ///
    /// This method resets string fields to empty strings 
    pub fn clear(&mut self) {
        self.name.clear();
        self.id_hex.clear();
        self.signals = Vec::new();
    }
}