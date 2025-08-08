use std::collections::HashMap;

use crate::types::database::Database;
use crate::types::message::Message;
use crate::types::signal::Signal;
use crate::types::node::Node;

pub(crate) fn fct(db: &mut Database, line: &str) {
    // SG_ <name> : <bit_start>|<bit_lengths>@<endianness><signedness> (<scale>,<factor>) [<min>|<max>] "<units>" <receiver nodes...>
    if db.messages.is_empty() {
        return;
    }

    // remove whitespace at end and beginning
    let line: &str = line.trim_start();

    // Split line in two part: before ":" and after
    let mut split_colon = line.splitn(2, ':');
    let left = split_colon.next().unwrap().trim();
    let right = split_colon.next().unwrap_or("").trim();

    // Signal Name
    let name: String = left.split_whitespace().nth(1).unwrap_or("").to_string();

    // Bit start / length / endian / sign
    let mut right_parts = right.split_whitespace();
    let bit_info = right_parts.next().unwrap_or(""); // "63|1@1+"
    let mut bit_and_rest = bit_info.split('@');
    let bit_pos_len = bit_and_rest.next().unwrap_or(""); // "63|1"
    let endian_sign = bit_and_rest.next().unwrap_or(""); // "1+"

    let mut pos_len_parts = bit_pos_len.split('|');
    let bit_start = pos_len_parts
        .next()
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0);
    let bit_length = pos_len_parts
        .next()
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0);

    let endian = endian_sign
        .chars()
        .nth(0)
        .unwrap_or('1')
        .to_digit(10)
        .unwrap_or(1) as usize;
    let sign = if endian_sign.contains('-') { 1 } else { 0 };

    // Scale and offset
    let factor_offset_raw: &str = right_parts
        .next()
        .unwrap_or("(1,0)")
        .trim_matches(|c| c == '(' || c == ')');
    let mut so_parts = factor_offset_raw.split(',');
    let factor: f64 = so_parts.next().unwrap_or("1").parse::<f64>().unwrap_or(1.0);
    let offset: f64 = so_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

    // Min and max
    let min_max_raw: &str = right_parts
        .next()
        .unwrap_or("[0|0]")
        .trim_matches(|c| c == '[' || c == ']');
    let mut mm_parts = min_max_raw.split('|');
    let min: f64 = mm_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
    let max: f64 = mm_parts.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0);

    // Measurement Unit
    let unit: String = right_parts
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .to_string();

    // Receiver nodes (possono essere separati da virgole)
    let receivers_str = right_parts.collect::<Vec<&str>>().join(" ");
    let receivers: Vec<Node> = receivers_str
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| Node {
            name: s.trim().to_string(),
            comment: "".to_string(),
        })
        .collect();

    // Generate the Signal
    let signal: Signal = Signal {
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
        receiver_nodes: receivers,
        comment: String::new(),
        value_table: Default::default(),
    };

    // Add to last message
    if let Some(last_msg) = db.messages.last_mut() {
        last_msg.signals.push(signal);
    }
}

pub(crate) fn comments(db: &mut Database, line: &str) {
    // Example: CM_ SG_ 1635 SignalName "Comment..."

    let mut parts = line.split_whitespace();
    parts.next(); // skip CM_
    parts.next(); // skip SG_

    let id_str = match parts.next() {
        Some(id) => id,
        None => return,
    };

    let id: u64 = match id_str.parse() {
        Ok(v) => v,
        Err(_) => return,
    };

    let signal_name = match parts.next() {
        Some(name) => name,
        None => return,
    };

    // Find Message
    let msg: &mut Message = match db.get_message_by_id_mut(id) {
        Some(m) => m,
        None => return,
    };

    // Find Signal
    let sig: &mut Signal = match msg.get_signal_by_name_mut(signal_name) {
        Some(s) => s,
        None => return,
    };

    // Extract the comment
    let first_quote: usize = match line.find('"') {
        Some(pos) => pos,
        None => return,
    };
    let last_quote: usize = match line.rfind('"') {
        Some(pos) if pos > first_quote => pos,
        _ => return,
    };

    let comment = &line[first_quote + 1..last_quote];

    // Push comment into the signal, remove initial whitespaces in lines
    sig.comment = comment
        .lines()
        .map(|l| l.trim_start())
        .collect::<Vec<_>>()
        .join("\n");
}

pub(crate) fn value_table(db: &mut Database, line: &str) {
    // remove whitespace at end and beginning
    let line: &str = line.trim_start();

    // Example: VAL_ <message_id> <signal_name> <val1> "<descr1>" <val2> "<descr2>" ... ;
    let mut parts = line.split_whitespace();

    // Skip "VAL_"
    parts.next();

    // Message ID come numero decimale
    let message_id: u64 = parts.next().unwrap_or("").parse::<u64>().unwrap_or(0);

    // Signal Name
    let signal_name: String = parts.next().unwrap_or("").to_string();

    // Rest of row contains couples: <value> "<description>"
    let mut remaining: String = parts.collect::<Vec<_>>().join(" ");

    // Remove final ";"
    remaining = remaining.trim_end_matches(';').trim().to_string();

    // Parsing couples: <value> "<description>"
    let mut value_table: HashMap<i32, String> = HashMap::new();
    let mut tokens = remaining.split('"').map(|s| s.trim());

    while let Some(before) = tokens.next() {
        let before: &str = before.trim();
        if before.is_empty() {
            continue;
        }
        if let Some(num_str) = before.split_whitespace().last() {
            if let Ok(val) = num_str.parse::<i32>() {
                if let Some(desc) = tokens.next() {
                    value_table.insert(val, desc.to_string());
                }
            }
        }
    }

    // Add value table to right message and signal
    if let Some(msg) = db.messages.iter_mut().find(|m| m.id == message_id) {
        if let Some(signal) = msg.signals.iter_mut().find(|s| s.name == signal_name) {
            signal.value_table = value_table;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fct() {
        use crate::dbc::messages;
        let mut db: Database = Database::default();

        // Add example message to connect the signal to
        messages::fct(&mut db, "BO_ 960 Key_Status: 4 BCM");

        // There must be only 1 message
        assert_eq!(db.messages.len(), 1);

        // Example Line
        fct(&mut db, r#"SG_ Engine_Speed : 48|8@1+ (1,0) [0|255] "km/h" Infotainment"#);

        // There must be only one signal inside the message
        let msg = &db.messages[0];
        assert_eq!(msg.signals.len(), 1);

        // check on the signal
        let sig = &msg.signals[0];
        assert_eq!(sig.name, "Engine_Speed");
        assert_eq!(sig.bit_start, 48);
        assert_eq!(sig.bit_length, 8);
        assert_eq!(sig.endian, 1);
        assert_eq!(sig.sign, 0);
        assert_eq!(sig.factor, 1.0);
        assert_eq!(sig.offset, 0.0);
        assert_eq!(sig.min, 0.0);
        assert_eq!(sig.max, 255.0);
        assert_eq!(sig.unit_of_measurement, "km/h");

        // Receiver nodes
        assert_eq!(sig.receiver_nodes.len(), 1);
        assert_eq!(sig.receiver_nodes[0].name, "Infotainment");

        // Empty Value Table
        assert!(sig.value_table.is_empty());
    }

    #[test]
    fn test_comments_id_found() {
        // Prepare a db and a message to test
        let mut db: Database = Database::default();
        let msg: Message = Message {
            id: 1635u64,
            id_hex: format!("{:X}", 1635u64),
            name: "Gateway_01".to_string(),
            byte_length: 16,
            msgtype: "CAN FD".to_string(),
            sender_nodes: vec![],
            signals: vec![Signal {
                name: "NetStatus".to_string(),
                ..Default::default()
            }],
            comment: String::new(),
        };
        db.messages.push(msg);

        // Example Line (con rimozione indentazione)
        let line = r#"CM_ SG_ 1635 NetStatus "Example of
                multiline
                comment."; "#;

        let expected_output = r#"Example of
                multiline
                comment."#;

        // Remove initial space of each line
        let expected_output = expected_output
            .lines()
            .map(|l| l.trim_start())
            .collect::<Vec<_>>()
            .join("\n");

        println!("line = {}", line);

        // parse the comment
        comments(&mut db, &line);

        // check that comment has been correctly assigned
        let msg: &mut Message = db.get_message_by_id_mut(1635u64).unwrap();
        let sig: &Signal = msg.get_signal_by_name("NetStatus").unwrap();
        assert_eq!(sig.comment, expected_output);
    }

    #[test]
    fn test_comments_id_not_found() {
        // Empty Database
        let mut db: Database = Database::default();

        // Line with not existing id
        let line: &'static str = r#"CM_ SG_ 9999 SomeSignal "This comment will not be assigned";"#;

        // Parsing
        comments(&mut db, &line);

        // No comment should be added
        assert!(db.messages.is_empty());
    }

    #[test]
    fn test_dbc_parse_value_table() {
        use crate::dbc::messages;
        let mut db: Database = Database::default();

        // Add a message
        messages::fct(&mut db, "BO_ 960 Key_Status: 4 BCM");

        // Add a signal
        fct(&mut db, r#"SG_ Engine_Speed : 48|8@1+ (1,0) [0|255] "km/h" Infotainment"#);

        // Example Line
        value_table(&mut db, r#"VAL_ 960 Engine_Speed 0 "Off" 1 "On" 255 "Error";"#);

        // Check message presence
        assert_eq!(db.messages.len(), 1);

        // Check signal presence
        let msg = &db.messages[0];
        assert_eq!(msg.signals.len(), 1);

        // Check value table
        let sig = &msg.signals[0];
        assert_eq!(sig.value_table.len(), 3);
        assert_eq!(sig.value_table.get(&0), Some(&"Off".to_string()));
        assert_eq!(sig.value_table.get(&1), Some(&"On".to_string()));
        assert_eq!(sig.value_table.get(&255), Some(&"Error".to_string()));
    }
}