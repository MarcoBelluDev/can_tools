use crate::types::{
    database::{CanDatabase, CanSignalKey},
    message::CanMessage,
    signal::Signess,
};

/// Decodes a `SIG_VALTYPE_` line assigning floating-point encodings to a signal.
///
/// Shape: `SIG_VALTYPE_ <MsgID> <SignalName> : <Value>;`
/// where `<Value>` is `1` (IEEE float, 32-bit) or `2` (IEEE double, 64-bit).
pub(crate) fn decode(db: &mut CanDatabase, line: &str) {
    let mut parts = line.trim_end_matches(';').split_ascii_whitespace();

    // 1) "SIG_VALTYPE_"
    match parts.next() {
        Some("SIG_VALTYPE_") => {}
        _ => return,
    }

    // 2) Message ID
    let Some(msg_id_tok) = parts.next() else {
        return;
    };
    let Ok(msg_id) = msg_id_tok.parse::<u32>() else {
        return;
    };

    // 3) <SignalName>
    let signal_name: &str = match parts.next() {
        Some(name) => name,
        None => return,
    };

    // 4) skip ':'
    match parts.next() {
        Some(":") => {}
        _ => return,
    }

    // 5) <Value>
    let value: &str = match parts.next() {
        Some(val) => val,
        None => return,
    };

    // 6) assign the Sign property to specific sisignal
    let sig_key_opt: Option<CanSignalKey> = {
        let msg: &CanMessage = match db.get_message_by_id(msg_id) {
            Some(m) => m,
            None => return,
        };
        msg.signals.iter().copied().find(|&sk| {
            db.get_sig_by_key(sk)
                .is_some_and(|s| s.name.eq_ignore_ascii_case(signal_name))
        })
    };

    if let Some(sk) = sig_key_opt
        && let Some(sig) = db.get_sig_by_key_mut(sk)
    {
        match value {
            "2" => {
                sig.sign = Signess::IeeeDouble;
                sig.bit_length = 64;
            }
            "1" => {
                sig.sign = Signess::IeeeFloat;
                sig.bit_length = 32;
            }
            _ => {}
        }
    }
}
