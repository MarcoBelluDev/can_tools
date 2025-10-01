/// In-memory representation of a CAN database (ARXML).
///
/// Holds metadata (name, bus type, baud rates, version), the arenas of nodes/messages/signals
/// (SlotMaps with stable keys), optional order vectors to control iteration order, and
/// several normalized lookup maps for efficient queries.
#[derive(Default, Clone, Debug)]
pub struct DatabaseARXML {
    // --- General information ---
    /// Logical name of the database (if available).
    pub name: String,
    /// Bus type (e.g., `"CAN"`).
    pub bustype: BusType,
    /// Classic baud rate (bit/s). `0` if unspecified.
    pub baudrate: u32,
    /// CAN FD baud rate (bit/s). `0` if unspecified.
    pub baudrate_canfd: u32,
    /// Database version string.
    pub version: String,
    /// Database comment.
    pub comment: String,
}

/// Bus type for an ARXML-extracted database.
#[derive(Default, Clone, PartialEq, Debug)]
pub enum BusType {
    #[default]
    Can,
    CanFd,
}

impl BusType {
    /// Returns a user-friendly string (e.g., `"CAN"`, `"CAN FD"`).
    pub fn to_str(&self) -> String {
        match self {
            BusType::Can => "CAN".to_string(),
            BusType::CanFd => "CAN FD".to_string(),
        }
    }
}
