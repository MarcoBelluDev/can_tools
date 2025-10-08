use crate::dbc::types::signal::Endianness;

/// Verify that (bit_start, bit_length) fits within the frame defined by DLC.
/// Returns Ok(()) if the signal fits; Err(...) with the reason otherwise.
///
/// DBC assumptions:
/// - Intel: the field occupies bits [start, start + len - 1] on a linear 0..(8*bytes-1) plane.
/// - Motorola: map DBC bit_start to linear index `lin = (start & !7) + (7 - (start & 7))`,
///   then the field advances backwards: [lin - (len-1) .. lin].
pub fn check_signal_fits(
    dlc: u16,
    bit_start: u16,
    bit_length: u16,
    endianness: Endianness,
) -> Result<(), String> {
    let dlc_bytes: Option<usize> = match dlc {
        0..=8 => Some(dlc as usize),
        9 => Some(12),
        10 => Some(16),
        11 => Some(20),
        12 => Some(24),
        13 => Some(32),
        14 => Some(48),
        15 => Some(64),
        _ => None,
    };
    let bytes: usize = dlc_bytes.ok_or_else(|| "Invalid DLC".to_string())?;
    if bit_length == 0 {
        return Err("bit_length cannot be 0.".into());
    }
    let total_bits: usize = bytes * 8;

    match endianness {
        Endianness::Intel => {
            let start: usize = bit_start as usize;
            let end: usize = start + (bit_length as usize) - 1;
            if end < total_bits {
                Ok(())
            } else {
                Err(format!(
                    "Out of bounds (Intel): end={} ≥ total_bits={} (bytes={}, dlc={}).",
                    end, total_bits, bytes, dlc
                ))
            }
        }
        Endianness::Motorola => {
            // Map DBC start (MSB-first within a byte) to linear LSB-first index
            let s: usize = bit_start as usize;
            let linearized_start: usize = (s & !7) + (7 - (s & 7)); // e.g., start=0 -> 7, start=7 -> 0, start=8 -> 15, etc.
            let linearized_end: isize = linearized_start as isize - (bit_length as isize - 1);

            if linearized_start >= total_bits {
                return Err(format!(
                    "Out of bounds (Motorola): lin_start={} ≥ total_bits={} (bytes={}, dlc={}).",
                    linearized_start, total_bits, bytes, dlc
                ));
            }
            if linearized_end < 0 {
                return Err(format!(
                    "Out of bounds (Motorola): lin_end={} < 0 (bytes={}, dlc={}).",
                    linearized_end, bytes, dlc
                ));
            }
            // Safe: (linearized_end as usize) < total_bits because linearized_end >= 0 and linearized_end <= linearized_start < total_bits
            Ok(())
        }
    }
}
