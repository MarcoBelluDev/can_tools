use crate::types::database::{Database, NodeId, SignalDB};
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
    let mut receiver_nodes: Vec<NodeId> = Vec::new();

    // Spezza anche i token contenenti virgole
    for name in it
        .flat_map(|chunk| chunk.split(','))                  // <- split su virgola dentro al token
        .map(|s| s.trim().trim_matches(|c| c == ',' || c == ';')) // pulisci virgole/; residui
        .filter(|s| !s.is_empty())
    {
        if let Some(nid) = db.get_node_id_by_name(name) {
            receiver_nodes.push(nid);
        }
    }

    // current message id is the last one
    let mid: usize = db.messages.len() - 1;

    let mut sig: SignalDB = SignalDB {
        message: crate::types::database::MessageId(mid),
        name,
        bit_start,
        bit_length,
        endian,
        sign,
        factor,
        offset,
        min,
        max,
        unit_of_measurement: unit,
        receiver_nodes,
        comment: String::new(),
        value_table: HashMap::new(),
        steps: Vec::new(),
    };

    sig.compile_inline();
    db.add_signal(sig);
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

    if let Some(msg) = db.get_message_by_id(message_id) {
        // find signal id by name within msg
        if let Some(&sid) = msg
            .signals
            .iter()
            .find(|&&sid| db.signals[sid.0].name.eq_ignore_ascii_case(signal_name))
        {
            if let Some(s) = db.signals.get_mut(sid.0) {
                // Extract quoted comment
                let first_quote = match text.find('\"') {
                    Some(p) => p,
                    None => return,
                };
                let last_quote: usize = match text.rfind('\"') {
                    Some(p) if p > first_quote => p,
                    _ => return,
                };
                s.comment = text[first_quote + 1..last_quote].to_string();
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
        if let Some(&sid) = msg
            .signals
            .iter()
            .find(|&&sid| db.signals[sid.0].name == signal_name)
        {
            if let Some(s) = db.signals.get_mut(sid.0) {
                s.value_table = table;
            }
        }
    }
}
