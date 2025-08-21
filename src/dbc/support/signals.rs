use crate::types::database::{Database, MessageDB, NodeKey, SignalKey};
use std::collections::HashMap;

/// Decode a `SG_` line belonging to the **current message** (the last parsed BO_).
/// Format (typical):
/// SG_ <name> : <bit_start>|<bit_length>@<endian><sign> (<factor>,<offset>) [<min>|<max>] "<unit>" <receivers...>
pub(crate) fn decode(db: &mut Database, line: &str) {
    if db.messages.is_empty() {
        return;
    }

    let line: &str = line.trim_start();
    let mut split_colon = line.splitn(2, ':');
    let left: &str = split_colon.next().unwrap().trim(); // "SG_ NAME"
    let right: &str = split_colon.next().unwrap_or("").trim();

    let name: String = left
        .split_ascii_whitespace()
        .nth(1)
        .unwrap_or("")
        .to_string();
    if name.is_empty() {
        return;
    }

    let mut it = right.split_ascii_whitespace();

    // 1) bit info: "63|1@1+"
    let bit_info: &str = it.next().unwrap_or("");
    let mut bit_and_rest = bit_info.split('@');
    let bit_pos_len: &str = bit_and_rest.next().unwrap_or(""); // "63|1"
    let es: &str = bit_and_rest.next().unwrap_or(""); // "1+"
    let mut pos_len = bit_pos_len.split('|');
    let bit_start: u16 = pos_len.next().unwrap_or("0").parse().unwrap_or(0);
    let bit_length: u16 = pos_len.next().unwrap_or("0").parse().unwrap_or(0);
    let endian: u8 = es.chars().next().unwrap_or('1').to_digit(10).unwrap_or(1) as u8;
    let sign: u8 = if es.chars().nth(1).unwrap_or('+') == '-' {
        1
    } else {
        0
    };

    // 2) "(factor,offset)"
    let mut factor: f64 = 1.0;
    let mut offset: f64 = 0.0;
    if let Some(paren) = it.next() {
        if paren.starts_with('(') {
            let mut acc = String::from(paren);
            // Might be split across tokens; gather until ')'
            while !acc.ends_with(')') {
                if let Some(tok) = it.next() {
                    acc.push(' ');
                    acc.push_str(tok);
                } else {
                    break;
                }
            }
            let inner: &str = acc.trim_start_matches('(').trim_end_matches(')');
            let mut nums = inner.split(',').map(|s| s.trim());
            factor = nums.next().unwrap_or("1").parse().unwrap_or(1.0);
            offset = nums.next().unwrap_or("0").parse().unwrap_or(0.0);
        }
    }

    // 3) "[min|max]"
    let mut min: f64 = f64::MIN;
    let mut max: f64 = f64::MAX;
    // There might be a dedicated token like "[0|100]"
    let bounds_token = it.next().unwrap_or("");
    let (mut seen_bounds, mut next_tok_cache) = (false, String::new());
    if bounds_token.starts_with('[') && bounds_token.contains('|') {
        seen_bounds = true;
        let mut b = String::from(bounds_token);
        while !b.ends_with(']') {
            if let Some(tok) = it.next() {
                b.push(' ');
                b.push_str(tok);
            } else {
                break;
            }
        }
        let inner: &str = b.trim_start_matches('[').trim_end_matches(']');
        let mut nums = inner.split('|').map(|s| s.trim());
        min = nums.next().unwrap_or("0").parse().unwrap_or(0.0);
        max = nums.next().unwrap_or("0").parse().unwrap_or(0.0);
    } else {
        next_tok_cache = bounds_token.to_string();
    }

    // 4) "unit"
    let unit_token: Option<&str> = if seen_bounds {
        it.next()
    } else {
        Some(next_tok_cache.as_str())
    };
    let unit_raw: &str = unit_token.unwrap_or("").trim();
    let unit: String = if unit_raw.starts_with('"') {
        // gather full quoted
        let mut acc: String = String::from(unit_raw);
        while !acc.ends_with('"') {
            if let Some(tok) = it.next() {
                acc.push(' ');
                acc.push_str(tok);
            } else {
                break;
            }
        }
        acc.trim_matches('"').to_string()
    } else {
        unit_raw.trim_matches('"').to_string()
    };

    // 5) receivers (space-separated)
    let mut receiver_nodes: Vec<NodeKey> = Vec::new();

    // Spezza anche i token contenenti virgole
    for name in it
        .flat_map(|chunk| chunk.split(',')) // <- split su virgola dentro al token
        .map(|s| s.trim().trim_matches(|c| c == ',' || c == ';')) // pulisci virgole/; residui
        .filter(|s| !s.is_empty())
    {
        if let Some(rif) = db.get_node_key_by_name(name) {
            receiver_nodes.push(rif);
        }
    }

    let sig_rif: SignalKey = db.add_signal_if_absent(
        &name,
        bit_start,
        bit_length,
        endian,
        sign,
        factor,
        offset,
        min,
        max,
        &unit,
        receiver_nodes,
    );

    // link the signal to current message
    if let Some(msg_key) = db.current_msg {
        if let Some(msg) = db.messages.get_mut(msg_key) {
            if !msg.signals.contains(&sig_rif) {
                msg.signals.push(sig_rif);
            }
        }
    }
}

/// Parse a signal-level comment:
/// `CM_ SG_ <MessageID> <SignalName> "Comment...";`
pub(crate) fn comments(db: &mut Database, text: &str) {
    let lower: String = text.to_ascii_lowercase();
    if !lower.starts_with("cm_ sg_") {
        return;
    }
    let parts: Vec<&str> = text.split_ascii_whitespace().collect();
    if parts.len() < 4 {
        return;
    }
    let message_id: u64 = parts[2].parse::<u64>().unwrap_or(0);
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
    if let Some(sig_key) = sig_key_opt {
        if let Some(s) = db.get_sig_by_key_mut(sig_key) {
            if let (Some(first), Some(last)) = (text.find('"'), text.rfind('"')) {
                if last > first {
                    s.comment = text[first + 1..last].to_string();
                }
            }
        }
    }
}

/// Parse a VAL_ line that defines a value table for a specific signal:
/// `VAL_ <MessageID> <SignalName> <value> "<desc>" ... ;`
pub(crate) fn value_table(db: &mut Database, line: &str) {
    let mut tokens = line.split_ascii_whitespace();
    if tokens.next().map(|s| s.to_ascii_lowercase()) != Some("val_".into()) {
        return;
    }
    let message_id: u64 = tokens
        .next()
        .and_then(|t| t.parse::<u64>().ok())
        .unwrap_or(0);
    let signal_name = match tokens.next() {
        Some(n) => n,
        None => return,
    };

    // Collect pairs value "desc"
    let mut table: HashMap<i32, String> = HashMap::new();
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

    if let Some(msg) = db.get_message_by_id(message_id) {
        if let Some(&sig_key) = msg.signals.iter().find(|&&sig_key| {
            db.get_sig_by_key(sig_key)
                .is_some_and(|s| s.name == signal_name)
        }) {
            if let Some(s) = db.get_sig_by_key_mut(sig_key) {
                s.value_table = table;
            }
        }
    }
}
