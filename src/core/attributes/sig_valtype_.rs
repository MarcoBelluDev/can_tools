use crate::types::{
    database::{DatabaseDBC, SignalKey},
    message::MessageDBC,
    signal::Signess,
};

/// `SIG_VALTYPE_ "MsgID" <SignalName> : <Value>;`
pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
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
    let sig_key_opt: Option<SignalKey> = {
        let msg: &MessageDBC = match db.get_message_by_id(msg_id) {
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
