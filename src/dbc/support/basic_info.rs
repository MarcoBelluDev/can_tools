use crate::types::database::{BusType, Database};

pub(crate) fn decode(db: &mut Database, line: &str) {
    // Expected formats:
    // BA_ "DBName" "TestCAN";
    // BA_ "BusType" "CAN FD";
    // BA_ "Baudrate" 500000;
    // BA_ "BaudrateCANFD" 2000000;

    if line.contains(r#""Baudrate""#) {
        // BA_ "Baudrate" "500000";
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // Baudrate
        if let Some(text) = parts.next() {
            if let Ok(baudrate) = text.parse::<u32>() {
                db.baudrate = baudrate;
            }
        }
    } else if line.contains(r#""BusType""#) {
        // Expected: BA_ "BusType" "CAN FD";
        let s: &str = line.trim_end_matches(';').trim();

        // After split by '"': [unquoted, "BusType", unquoted, "CAN FD", ...]
        let mut quoted = s.split('"').skip(1).step_by(2);
        if let (Some(key), Some(val)) = (quoted.next(), quoted.next()) {
            if key.eq_ignore_ascii_case("BusType") {
                db.bustype = if val == "CAN FD" {
                    BusType::CanFd
                } else {
                    BusType::Can
                }
            }
        }
    } else if line.contains(r#""DBName""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // DBName
        if let Some(text) = parts.next() {
            db.name = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""BaudrateCANFD""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // BaudrateCANFD
        if let Some(text) = parts.next() {
            if let Ok(baudrate_canfd) = text.parse::<u32>() {
                db.baudrate_canfd = baudrate_canfd;
            }
        }
    } else if line.contains(r#""NmhNStart""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhNStart
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u16>() {
                db.nmh_n_start = number;
            }
        }
    } else if line.contains(r#""NmhLongTimer""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhLongTimer
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u16>() {
                db.nmh_long_timer = number;
            }
        }
    } else if line.contains(r#""NmhPrepareBusSleepTimer""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhPrepareBusSleepTimer
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u16>() {
                db.nmh_prepare_bus_sleep_timer = number;
            }
        }
    } else if line.contains(r#""NmhWaitBusSleepTimer""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhWaitBusSleepTimer
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u16>() {
                db.nmh_wait_bus_sleep_timer = number;
            }
        }
    } else if line.contains(r#""NmhTimeoutTimer""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhTimeoutTimer
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u16>() {
                db.nmh_timeout_timer = number;
            }
        }
    } else if line.contains(r#""NBTMax""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NBTMax
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.nmh_nbt_max = number;
            }
        }
    } else if line.contains(r#""NBTMin""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NBTMin
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.nmh_nbt_min = number;
            }
        }
    } else if line.contains(r#""SyncJumpWidthMax""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // SyncJumpWidthMax
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.sync_jump_width_max = number;
            }
        }
    } else if line.contains(r#""SyncJumpWidthMin""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // SyncJumpWidthMin
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.sync_jump_width_min = number;
            }
        }
    } else if line.contains(r#""SamplePointMax""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // SamplePointMax
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.sample_point_max = number;
            }
        }
    } else if line.contains(r#""SamplePointMin""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // SamplePointMin
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.sample_point_min = number;
            }
        }
    } else if line.contains(r#""VersionNumber""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // VersionNumber
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.version_number = number;
            }
        }
    } else if line.contains(r#""VersionYear""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // VersionYear
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.version_year = number;
            }
        }
    } else if line.contains(r#""VersionWeek""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // VersionWeek
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.version_week = number;
            }
        }
    } else if line.contains(r#""VersionMonth""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // VersionMonth
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.version_month = number;
            }
        }
    } else if line.contains(r#""VersionDay""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // VersionDay
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.version_day = number;
            }
        }
    } else if line.contains(r#""VAGTP20_SetupStartAddress""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // VAGTP20_SetupStartAddress
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.vagtp20_setup_start_address = number;
            }
        }
    } else if line.contains(r#""VAGTP20_SetupMessageCount""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // VAGTP20_SetupMessageCount
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.vagtp20_setup_message_count = number;
            }
        }
    } else if line.contains(r#""NmType""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmType
        if let Some(text) = parts.next() {
            db.nm_type = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""NmhMessageCount""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhMessageCount
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u8>() {
                db.nmh_message_count = number;
            }
        }
    } else if line.contains(r#""NmhBaseAddress""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhBaseAddress
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u32>() {
                db.nmh_base_address = number;
            }
        }
    } else if line.contains(r#""Manufacturer""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // Manufacturer
        if let Some(text) = parts.next() {
            db.manufacturer = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMTalkNM""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMTalkNM
        if let Some(text) = parts.next() {
            db.gen_nwm_talk_nm = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMSleepTime""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // NmhBaseAddress
        if let Some(text) = parts.next() {
            if let Ok(number) = text.parse::<u16>() {
                db.gen_nwm_sleep_time = number;
            }
        }
    } else if line.contains(r#""GenNWMGotoMode_BusSleep""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMGotoMode_BusSleep
        if let Some(text) = parts.next() {
            db.gen_nwm_goto_mode_bus_sleep = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMGotoMode_Awake""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMGotoMode_Awake
        if let Some(text) = parts.next() {
            db.gen_nwm_goto_mode_awake = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMApCanWakeUp""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMApCanWakeUp
        if let Some(text) = parts.next() {
            db.gen_nwm_ap_can_wake_up = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMApCanSleep""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMApCanSleep
        if let Some(text) = parts.next() {
            db.gen_nwm_ap_can_sleep = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMApCanOn""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMApCanOn
        if let Some(text) = parts.next() {
            db.gen_nwm_ap_can_on = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMApCanOff""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMApCanOff
        if let Some(text) = parts.next() {
            db.gen_nwm_ap_can_off = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMApCanNormal""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMApCanNormal
        if let Some(text) = parts.next() {
            db.gen_nwm_ap_can_normal = text.trim_matches('"').to_string();
        }
    } else if line.contains(r#""GenNWMApBusSleep""#) {
        let mut parts = line.trim_end_matches(';').split_ascii_whitespace();
        parts.next(); // BA_
        parts.next(); // GenNWMApBusSleep
        if let Some(text) = parts.next() {
            db.gen_nwm_ap_bus_sleep = text.trim_matches('"').to_string();
        }
    }
}

pub(crate) fn comment(db: &mut Database, line: &str) {
    // Expected formats:
    // CM_ "Comment regarding the network";
    let s: &str = line.trim_end_matches(';');
    if let Some((_, rest)) = s.split_once('"') {
        if let Some((inner, _)) = rest.rsplit_once('"') {
            db.comment = inner.to_string(); // quotes removed
        }
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
