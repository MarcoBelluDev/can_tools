use crate::dbc::types::{
    database::{DatabaseDBC, NodeKey, SignalKey, MessageKey},
    message::{MuxRole, MuxSelector},
    signal::{Endianness, Signess},
};

/// Decode a `SG_` line belonging to the **current message** (the last parsed BO_).
/// Format (typical):
/// SG_ <name> [M|mX]: <bit_start>|<bit_length>@<endian><sign> (<factor>,<offset>) [<min>|<max>] "<unit>" <receivers...>
pub(crate) fn decode(db: &mut DatabaseDBC, line: &str) {
    if db.messages.is_empty() {
        return;
    }

    let line: &str = line.trim_start().trim_end_matches(";");
    let mut split_colon = line.splitn(2, ':');
    let left: &str = split_colon.next().unwrap().trim(); // "SG_ NAME [M|mX]"
    let right: &str = split_colon.next().unwrap_or("").trim();

    // Left part analysis SG_ NAME [M|mX]
    let mut left_it = left.split_ascii_whitespace();
    let _sg: &str = left_it.next().unwrap_or(""); // "SG_"
    let name_token: &str = left_it.next().unwrap_or("");
    let after_name: &str = left_it.next().unwrap_or(""); // can be "", "M", "m0", etc.

    let name: String = name_token.to_string();
    if name.is_empty() {
        return;
    }

    // multiplexing tag decoding (if present)
    let mut mux_role: MuxRole = MuxRole::None;
    let mut mux_selector: Option<MuxSelector> = None;
    if !after_name.is_empty() {
        let tag: &str = after_name.trim_end_matches(':');
        if tag == "M" {
            mux_role = MuxRole::Multiplexor;
        } else if let Some(rest) = tag.strip_prefix('m')
            && let Ok(v) = rest.parse::<u32>()
        {
            mux_role = MuxRole::Multiplexed;
            mux_selector = Some(MuxSelector::Value(v));
        }
    }

    // right part analysis <bit_start>|<bit_length>@<endian><sign> (<factor>,<offset>) [<min>|<max>] "<unit>" <receivers...>
    let mut it = right.split_ascii_whitespace();

    // 1) bit info: "63|1@1+"
    let bit_info: &str = it.next().unwrap_or("");
    let mut bit_and_rest = bit_info.split('@');
    let bit_pos_len: &str = bit_and_rest.next().unwrap_or(""); // "63|1"
    let es: &str = bit_and_rest.next().unwrap_or(""); // "1+"
    let mut pos_len = bit_pos_len.split('|');
    let bit_start: u16 = pos_len.next().unwrap_or("0").parse().unwrap_or(0);
    let bit_length: u16 = pos_len.next().unwrap_or("0").parse().unwrap_or(0);
    let endian_value: u8 = es.chars().next().unwrap_or('1').to_digit(10).unwrap_or(1) as u8;
    let sign_value: u8 = if es.chars().nth(1).unwrap_or('+') == '-' {
        1
    } else {
        0
    };

    let endian= if endian_value == 1 {
        Endianness::Intel
    } else {
        Endianness::Motorola
    };
    let sign= if sign_value == 1 {
        Signess::Signed
    } else {
        Signess::Unsigned
    };

    // 2) "(factor,offset)"
    let mut factor: f64 = 1.0;
    let mut offset: f64 = 0.0;
    if let Some(paren) = it.next()
        && paren.starts_with('(')
    {
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
    let unit_of_measurement: String = if unit_raw.starts_with('"') {
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

    let unit: String = unit_of_measurement
        .strip_prefix("Unit_")
        .unwrap_or(&unit_of_measurement)
        .to_string();

    // 5) receivers (space-separated)
    let mut receiver_nodes: Vec<NodeKey> = Vec::new();
    let recv_opt: Option<&str> = it.next();
    if let Some(recv) = recv_opt {
        for node_name in recv.split(",") {
            if let Some(key) = db.get_node_key_by_name(node_name) {
                receiver_nodes.push(key);
            }
        }
    }

    // create the signal
    let sig_key: SignalKey = db.add_signal(
        &name,
        endian,
        sign,
        factor,
        offset,
        min,
        max,
        &unit,
    );

    // add receiver nodes
    for node_key in receiver_nodes.iter().copied() {
        let _ = db.add_sig_receiver_node(sig_key, node_key);
    }

    // add Message relation and multiplexing info
    let msg_key: MessageKey = match db.current_msg {
        Some(k) => k,
        // Create a fallback message if an SG_ appears before any BO_ (rare).
        None => match db.add_message("_Independent_Signal_", 0, 8) {
            Ok(k) => k,
            Err(_) => match db.get_msg_key_by_name("_Independent_Signal_") {
                Some(existing) => existing,
                None => return,
            },
        },
    };
    db.current_msg = Some(msg_key);

    let _ = db.add_msg_sig_relation(sig_key, msg_key, bit_start, bit_length, mux_role, mux_selector);

    // Back-link: for each receiver node, add this SignalKey in the signals_read vector
    for nk in receiver_nodes.iter().copied() {
        if let Some(node) = db.get_node_by_key_mut(nk)
            && !node.signals_read.contains(&sig_key)
        {
            node.signals_read.push(sig_key);
        }
    }
}
