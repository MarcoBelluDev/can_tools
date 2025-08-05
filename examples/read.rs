use candbc_parser::parser::file;

fn main() {
    let db_path = "examples/MIBCAN.dbc";

    match file::parse(db_path) {
        Ok(db) => {
            println!("Version: {}", db.version);
            println!(
                "Nodes: {:?}",
                db.nodes.iter().map(|n| &n.name).collect::<Vec<_>>()
            );
            println!("Messages: {}", db.messages.len());
            for msg in &db.messages {
                println!(
                    "Message Name: {}\nid = {}\nbyte_length = {}\nsender_node = {}\n",
                    msg.name, msg.id, msg.byte_length, msg.sender_node
                );
                for sig in &msg.signals {
                    println!(
                        "\tSignal Name: {}\n\tbit_start = {}\n\tbit_length = {}\n\tendian = {}\n\tsign = {}\n\tfactor = {}\n\toffset = {}\n\tmin = {}\n\tmax = {}\n\tunit_of_measurement = {}\n",
                        sig.name, sig.bit_start, sig.bit_length, sig.endian, sig.sign, sig.factor, sig.offset, sig.min, sig.max, sig.unit_of_measurement,
                    );

                    // Print Value Table if Present
                    if !sig.value_table.is_empty() {
                        println!("\tValue Table:");
                        let mut keys: Vec<_> = sig.value_table.keys().cloned().collect();
                        keys.sort(); // ordina per valore numerico
                        for key in keys {
                            if let Some(desc) = sig.value_table.get(&key) {
                                println!("\t  {} => {}", key, desc);
                            }
                        }
                    }
                }
                println!("\n");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
