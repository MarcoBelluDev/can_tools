use crate::types::database::{Database, Present};

/// `BA_ "Attribute" BU_ <Name> <value>;`
pub(crate) fn decode(db: &mut Database, line: &str) {
    let mut parts = line.split_ascii_whitespace();
    parts.next(); // BA_

    // Attribute
    let attribute: &str = match parts.next() {
        Some(a) => a.trim_matches('"'),
        None => return,
    };

    parts.next(); // BU_

    // Node name and value
    let (Some(name), Some(value)) = (parts.next(), parts.next()) else {
        return;
    };

    // mutable node exist for that name?
    let Some(node) = db.get_node_by_name_mut(name) else {
        return;
    };

    match attribute {
        "NodeLayerModules" => {
            node.node_layer_modules = value.trim_end_matches(';').trim_matches('"').to_string();
        }
        "GenNodAutoGenDsp" => {
            node.gen_nod_auto_gen_dsp = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "GenNodAutoGenSnd" => {
            node.gen_nod_auto_gen_snd = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "GenNodSleepTime" => {
            node.gen_nod_sleep_time = value.trim_end_matches(';').parse::<u16>().unwrap_or(0);
        }
        "ILUsed" => {
            node.int_layer_used = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "NmNode" => {
            node.nm_node = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "NmStationAddress" => {
            node.nm_station_address = value.trim_end_matches(';').parse::<u32>().unwrap_or(0);
        }
        "ECUVariantDefault" => {
            node.ecu_variant_default = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "ECUVariantGroup" => {
            node.ecu_variant_group = value.trim_end_matches(';').trim_matches('"').to_string();
        }
        "NmhNode" => {
            node.nmh_node = if value.trim_end_matches(';') == "1" {
                Present::Yes
            } else {
                Present::No
            }
        }
        "SamplePointMin" => {
            node.sample_point_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SamplePointMax" => {
            node.sample_point_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SamplePointCANFDMin" => {
            node.sample_point_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SamplePointCANFDMax" => {
            node.sample_point_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SSPOffsetCANFDMin" => {
            node.ssp_offset_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SSPOffsetCANFDMax" => {
            node.ssp_offset_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthMin" => {
            node.sync_jump_width_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthMax" => {
            node.sync_jump_width_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthCANFDMin" => {
            node.sync_jump_width_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "SyncJumpWidthCANFDMax" => {
            node.sync_jump_width_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaMin" => {
            node.time_quanta_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaMax" => {
            node.time_quanta_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaCANFDMin" => {
            node.time_quanta_canfd_min = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "TimeQuantaCANFDMax" => {
            node.time_quanta_canfd_max = value.trim_end_matches(';').parse::<u8>().unwrap_or(0);
        }
        "VAGTP20_TargetAddress" => {
            node.vag_tp20_target_address = value.trim_end_matches(';').parse::<u32>().unwrap_or(0);
        }
        _ => {}
    }
}
