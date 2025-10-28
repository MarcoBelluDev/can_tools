use crate::types::errors::MessageLayoutError;
use crate::types::signal::Endianness;

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
) -> Result<(), MessageLayoutError> {
    if bit_length == 0 {
        return Err(MessageLayoutError::ZeroBitLength);
    }
    let total_bits: usize = (dlc as usize) * 8;

    match endianness {
        Endianness::Intel => {
            let start: usize = bit_start as usize;
            let end: usize = start + (bit_length as usize) - 1;
            if end < total_bits {
                Ok(())
            } else {
                Err(MessageLayoutError::IntelOutOfBounds {
                    end,
                    total_bits,
                    dlc,
                })
            }
        }
        Endianness::Motorola => {
            // Map DBC start (MSB-first within a byte) to linear LSB-first index
            let s: usize = bit_start as usize;
            let linearized_start: usize = (s & !7) + (7 - (s & 7)); // e.g., start=0 -> 7, start=7 -> 0, start=8 -> 15, etc.
            let linearized_end: isize = linearized_start as isize - (bit_length as isize - 1);

            if linearized_start >= total_bits {
                return Err(MessageLayoutError::MotorolaStartOutOfBounds {
                    start: linearized_start,
                    total_bits,
                    dlc,
                });
            }
            if linearized_end < 0 {
                return Err(MessageLayoutError::MotorolaEndOutOfBounds {
                    end: linearized_end,
                    dlc,
                });
            }
            // Safe: (linearized_end as usize) < total_bits because linearized_end >= 0 and linearized_end <= linearized_start < total_bits
            Ok(())
        }
    }
}
