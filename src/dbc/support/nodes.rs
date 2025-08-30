use crate::types::database::{Database, Present};

/// Decode the BU_ line listing node names and register them in the database.
/// Example: `BU_: ECU1 ECU2 ECU3`
pub(crate) fn decode(db: &mut Database, line: &str) {
    // Split tokens, skip the "BU_:"
    let mut parts = line.split_ascii_whitespace();
    let first: Option<&str> = parts.next();
    if first != Some("BU_:") && first != Some("BU_") {
        return;
    }

    for name in parts {
        let name = name.trim();
        if !name.is_empty() {
            // creates if missing, returns existing rif otherwise
            db.add_node_if_absent(name);
        }
    }
}

/// Parse a node-level comment:
/// `CM_ BU_ NodeName "Comment..."`
pub(crate) fn comments(db: &mut Database, text: &str) {
    let mut parts = text.split_ascii_whitespace();
    if parts.next() != Some("CM_") {
        return;
    }
    if parts.next() != Some("BU_") {
        return;
    }
    let node_name = match parts.next() {
        Some(n) => n,
        None => return,
    };

    // Extract the quoted comment as-is (preserving inner spaces/newlines)
    let first_quote = match text.find('\"') {
        Some(p) => p,
        None => return,
    };
    let last_quote = match text.rfind('\"') {
        Some(p) if p > first_quote => p,
        _ => return,
    };
    let comment = text[first_quote + 1..last_quote].to_string();

    // Update single source of truth
    if let Some(node) = db.get_node_by_name_mut(node_name) {
        node.comment = comment;
    }
}

/// `BA_ "Attribute" BO_ <ID> <value>;`
pub(crate) fn add_info(db: &mut Database, line: &str) {
    let mut parts = line.split_ascii_whitespace();
    parts.next(); // BA_
    let attribute: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    match attribute {
        "NodeLayerModules" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.node_layer_modules = value.trim_end_matches(';').trim_matches('"').to_string();
        }
        "GenNodAutoGenDsp" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.gen_nod_auto_gen_dsp = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "GenNodAutoGenSnd" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.gen_nod_auto_gen_snd = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "GenNodSleepTime" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.gen_nod_sleep_time = value.trim_end_matches(';').parse::<u16>().unwrap_or(0);
        }
        "ILUsed" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.int_layer_used = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "NmNode" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.nm_node = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "NmStationAddress" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.nm_station_address = value.trim_end_matches(';').parse::<u32>().unwrap_or(0);
        }
        "ECUVariantDefault" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.ecu_variant_default = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "ECUVariantGroup" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.ecu_variant_group = value.trim_end_matches(';').trim_matches('"').to_string();
        }
        "NmhNode" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.nmh_node = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "SamplePointMin" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sample_point_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SamplePointMax" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sample_point_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SamplePointCANFDMin" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sample_point_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SamplePointCANFDMax" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sample_point_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SSPOffsetCANFDMin" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.ssp_offset_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SSPOffsetCANFDMax" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.ssp_offset_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthMin" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sync_jump_width_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthMax" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sync_jump_width_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthCANFDMin" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sync_jump_width_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthCANFDMax" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.sync_jump_width_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaMin" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.time_quanta_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaMax" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.time_quanta_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaCANFDMin" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.time_quanta_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaCANFDMax" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.time_quanta_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "VAGTP20_TargetAddress" => {
            parts.next(); // BU_
            // fourth and fifth parts exist?
            let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
                return;
            };

            // mutable node exist for that name?
            let Some(node) = db.get_node_by_name_mut(name) else {
                return;
            };

            // assign value
            node.vag_tp20_target_address = value.trim_end_matches(';').parse::<u32>().unwrap_or(0);
        }
        _ => {}
    }
}
