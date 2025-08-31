use crate::{Database, MessageDB, SignalKey};

/// Parse a signal-level comment:
/// `CM_ SG_ <MessageID> <SignalName> "Comment...";`
pub(crate) fn decode(db: &mut Database, text: &str) {
    let lower: String = text.to_ascii_lowercase();
    if !lower.starts_with("cm_ sg_") {
        return;
    }
    let parts: Vec<&str> = text.split_ascii_whitespace().collect();
    if parts.len() < 4 {
        return;
    }
    let message_id: u32 = parts[2].parse::<u32>().unwrap_or(0);
    let signal_name: &str = parts[3].trim_matches('"'); // usually not quoted here

    // Risolvi il SignalKey cercando per nome *dentro il messaggio*,
    // ma chiudi il borrow immutabile di `db` in questo blocco.
    let sig_key_opt: Option<SignalKey> = {
        let msg: &MessageDB = match db.get_message_by_id(message_id) {
            Some(m) => m,
            None => return,
        };

        msg.signals.iter().copied().find(|&sig_key| {
            db.get_sig_by_key(sig_key)
                .is_some_and(|s| s.name.eq_ignore_ascii_case(signal_name))
        })
    };

    // Ora puoi prendere un borrow mutabile di `db` per aggiornare il commento.
    if let Some(sig_key) = sig_key_opt
        && let Some(s) = db.get_sig_by_key_mut(sig_key)
        && let (Some(first), Some(last)) = (text.find('"'), text.rfind('"'))
        && last > first
    {
        s.comment = text[first + 1..last].to_string();
    }
}
