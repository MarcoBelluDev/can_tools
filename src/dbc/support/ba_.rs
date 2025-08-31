use crate::types::database::{BusType, Database};

pub(crate) fn decode(db: &mut Database, line: &str) {
    // Expected formats (global BA_ attributes):
    // BA_ "DBName" "TestCAN";
    // BA_ "BusType" "CAN FD";
    // BA_ "Baudrate" 500000;
    // BA_ "BaudrateCANFD" 2000000;
    // ...plus other attributes listed below.

    // Trim ending ';' and split by ASCII whitespace.
    let mut parts = line.trim().trim_end_matches(';').split_ascii_whitespace();

    // 1) "BA_"
    match parts.next() {
        Some("BA_") => {}
        _ => return,
    }

    // 2) Attribute token (e.g., "\"DBName\"")
    let attr_tok: &str = match parts.next() {
        Some(a) => a,
        None => return,
    };
    let attribute: &str = attr_tok.trim_matches('"');

    // 3) Rebuild the remaining tail to preserve spaces inside quoted values
    let rest_joined: String = parts.collect::<Vec<_>>().join(" ");
    let rest: &str = rest_joined.trim();

    // 4) Extract the value:
    //    - if it starts with a quote => take content up to the next quote
    //    - otherwise treat the remainder as the numeric value (already ';'-stripped)
    let value: &str = if let Some(inner) = rest.strip_prefix('"') {
        match inner.find('"') {
            Some(end) => &inner[..end],
            None => return, // unmatched quotes
        }
    } else {
        rest
    };

    match attribute {
        // ---- u32 fields ----
        "Baudrate" => {
            if let Ok(num) = value.parse::<u32>() {
                db.baudrate = num;
            }
        }
        "BaudrateCANFD" => {
            if let Ok(num) = value.parse::<u32>() {
                db.baudrate_canfd = num;
            }
        }
        "NmhBaseAddress" => {
            if let Ok(num) = value.parse::<u32>() {
                db.nmh_base_address = num;
            }
        }

        // ---- u16 fields ----
        "NmhNStart" => {
            if let Ok(num) = value.parse::<u16>() {
                db.nmh_n_start = num;
            }
        }
        "NmhLongTimer" => {
            if let Ok(num) = value.parse::<u16>() {
                db.nmh_long_timer = num;
            }
        }
        "NmhPrepareBusSleepTimer" => {
            if let Ok(num) = value.parse::<u16>() {
                db.nmh_prepare_bus_sleep_timer = num;
            }
        }
        "NmhWaitBusSleepTimer" => {
            if let Ok(num) = value.parse::<u16>() {
                db.nmh_wait_bus_sleep_timer = num;
            }
        }
        "NmhTimeoutTimer" => {
            if let Ok(num) = value.parse::<u16>() {
                db.nmh_timeout_timer = num;
            }
        }
        "GenNWMSleepTime" => {
            if let Ok(num) = value.parse::<u16>() {
                db.gen_nwm_sleep_time = num;
            }
        }

        // ---- u8 fields ----
        "NBTMax" => {
            if let Ok(num) = value.parse::<u8>() {
                db.nmh_nbt_max = num;
            }
        }
        "NBTMin" => {
            if let Ok(num) = value.parse::<u8>() {
                db.nmh_nbt_min = num;
            }
        }
        "SyncJumpWidthMax" => {
            if let Ok(num) = value.parse::<u8>() {
                db.sync_jump_width_max = num;
            }
        }
        "SyncJumpWidthMin" => {
            if let Ok(num) = value.parse::<u8>() {
                db.sync_jump_width_min = num;
            }
        }
        "SamplePointMax" => {
            if let Ok(num) = value.parse::<u8>() {
                db.sample_point_max = num;
            }
        }
        "SamplePointMin" => {
            if let Ok(num) = value.parse::<u8>() {
                db.sample_point_min = num;
            }
        }
        "VersionNumber" => {
            if let Ok(num) = value.parse::<u8>() {
                db.version_number = num;
            }
        }
        "VersionYear" => {
            if let Ok(num) = value.parse::<u8>() {
                db.version_year = num;
            }
        }
        "VersionWeek" => {
            if let Ok(num) = value.parse::<u8>() {
                db.version_week = num;
            }
        }
        "VersionMonth" => {
            if let Ok(num) = value.parse::<u8>() {
                db.version_month = num;
            }
        }
        "VersionDay" => {
            if let Ok(num) = value.parse::<u8>() {
                db.version_day = num;
            }
        }
        "VAGTP20_SetupStartAddress" => {
            if let Ok(num) = value.parse::<u8>() {
                db.vagtp20_setup_start_address = num;
            }
        }
        "VAGTP20_SetupMessageCount" => {
            if let Ok(num) = value.parse::<u8>() {
                db.vagtp20_setup_message_count = num;
            }
        }
        "NmhMessageCount" => {
            if let Ok(num) = value.parse::<u8>() {
                db.nmh_message_count = num;
            }
        }

        // ---- string fields ----
        "DBName" => {
            db.name = value.to_string();
        }
        "BusType" => {
            db.bustype = if value.eq_ignore_ascii_case("CAN FD") {
                BusType::CanFd
            } else {
                BusType::Can
            };
        }
        "NmType" => {
            db.nm_type = value.to_string();
        }
        "Manufacturer" => {
            db.manufacturer = value.to_string();
        }
        "GenNWMTalkNM" => {
            db.gen_nwm_talk_nm = value.to_string();
        }
        "GenNWMGotoMode_BusSleep" => {
            db.gen_nwm_goto_mode_bus_sleep = value.to_string();
        }
        "GenNWMGotoMode_Awake" => {
            db.gen_nwm_goto_mode_awake = value.to_string();
        }
        "GenNWMApCanWakeUp" => {
            db.gen_nwm_ap_can_wake_up = value.to_string();
        }
        "GenNWMApCanSleep" => {
            db.gen_nwm_ap_can_sleep = value.to_string();
        }
        "GenNWMApCanOn" => {
            db.gen_nwm_ap_can_on = value.to_string();
        }
        "GenNWMApCanOff" => {
            db.gen_nwm_ap_can_off = value.to_string();
        }
        "GenNWMApCanNormal" => {
            db.gen_nwm_ap_can_normal = value.to_string();
        }
        "GenNWMApBusSleep" => {
            db.gen_nwm_ap_bus_sleep = value.to_string();
        }

        // Unhandled BA_ attributes are ignored here.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode() {
        let mut db: Database = Database::default();

        // check that invalid lines are not accepted
        decode(&mut db, r#"BA_ "UnknownAttr" "SomeValue";"#);
        decode(&mut db, r#"This is not a valid line"#);
        decode(&mut db, r#"BA_ "Baudrate";"#); // Missing value

        // Nothing should be set
        assert_eq!(db.baudrate, 0);
        assert_eq!(db.bustype, BusType::Can);
        assert_eq!(db.name, "");
        assert_eq!(db.baudrate_canfd, 0);

        // check valid lines are accepted
        decode(&mut db, r#"BA_ "Baudrate" 500000;"#);
        decode(&mut db, r#"BA_ "BusType" "CAN FD";"#);
        decode(&mut db, r#"BA_ "DBName" "TestCAN";"#);
        decode(&mut db, r#"BA_ "BaudrateCANFD" 2000000;"#);

        assert_eq!(db.baudrate, 500000);
        assert_eq!(db.bustype, BusType::CanFd);
        assert_eq!(db.name, "TestCAN");
        assert_eq!(db.baudrate_canfd, 2000000);
    }
}
