use crate::types::database::DatabaseDBC;
use std::collections::BTreeMap;

/// Parse a VAL_ line that defines a value table for a specific signal:
/// `VAL_ <MessageID> <SignalName> <value> "<desc>" ... ;`
pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    let mut tokens = line.split_ascii_whitespace();
    if tokens.next().map(|s| s.to_ascii_lowercase()) != Some("val_".into()) {
        return;
    }
    let message_id: u32 = tokens
        .next()
        .and_then(|t| t.parse::<u32>().ok())
        .unwrap_or(0);
    let signal_name = match tokens.next() {
        Some(n) => n,
        None => return,
    };

    // Collect pairs: numeric value followed by quoted description
    let mut table: BTreeMap<i32, String> = BTreeMap::new();
    let mut t = tokens.peekable();
    while let Some(val_tok) = t.next() {
        if val_tok.ends_with(';') {
            break;
        } // sanity
        let val = match val_tok.parse::<i32>() {
            Ok(v) => v,
            Err(_) => break,
        };
        // desc may be a multi-token quoted string
        let mut desc = String::new();
        if let Some(d) = t.next() {
            if d.starts_with('"') {
                desc.push_str(d);
                while !desc.ends_with('"') {
                    if let Some(nxt) = t.next() {
                        desc.push(' ');
                        desc.push_str(nxt);
                    } else {
                        break;
                    }
                }
                desc = desc.trim_matches('"').to_string();
            } else {
                // unexpected token; stop
                break;
            }
        }
        table.insert(val, desc);
    }

    if let Some(msg) = db.get_message_by_id(message_id)
        && let Some(&sig_key) = msg.signals.iter().find(|&&sig_key| {
            db.get_sig_by_key(sig_key)
                .is_some_and(|s| s.name == signal_name)
        })
        && let Some(s) = db.get_sig_by_key_mut(sig_key)
    {
        s.value_table = table;
    }
}
