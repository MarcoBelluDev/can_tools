use chrono::{DateTime, Datelike, IsoWeek, Local, NaiveDate};

use crate::dbc::types::attributes::{AttrObject, AttrType, AttributeSpec, AttributeValue};
use crate::dbc::types::database::{BusType, DatabaseDBC};
use crate::dbc::types::errors::DbcCreateError;

/// Builds an empty `DatabaseDBC` populated with canonical metadata defaults.
///
/// Validates that `name` and `version` are non-empty, configures the appropriate bus
/// attributes (including CAN FD baud rate support), and seeds the default date/version
/// attribute specs so the caller can start adding nodes/messages from a clean slate.
/// Returns `DbcCreateError::EmptyDatabaseName` or `DbcCreateError::EmptyDatabaseVersion`
/// when the provided identifiers are blank.
pub fn new_database(
    name: &str,
    bustype: BusType,
    version: &str,
) -> Result<DatabaseDBC, DbcCreateError> {
    if name.trim().is_empty() {
        return Err(DbcCreateError::EmptyDatabaseName);
    }
    if version.trim().is_empty() {
        return Err(DbcCreateError::EmptyDatabaseVersion);
    }

    // initialize the Database
    let mut db: DatabaseDBC = DatabaseDBC {
        name: name.to_string(),
        bustype: bustype.clone(),
        version: version.to_string(),
        ..Default::default()
    };

    // Fill in default DBName
    let dbname_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        name: "DBName".to_string(),
        kind: AttrType::String,
        default: Some(AttributeValue::Str("".to_string())),
        ..Default::default()
    };
    db.attr_spec.insert("DBName".to_string(), dbname_spec);

    db.attributes
        .insert("DBName".to_string(), AttributeValue::Str(db.name.clone()));

    // Fill in default BusType
    let bustype_label: String = bustype.to_str();
    let bustype_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Str("".to_string())),
        name: "BusType".to_string(),
        kind: AttrType::String,
        ..Default::default()
    };
    db.attr_spec.insert("BusType".to_string(), bustype_spec);

    db.attributes
        .insert("BusType".to_string(), AttributeValue::Str(bustype_label));

    // Fill in default Baudrate for Standard CAN and its definition
    let baudrate_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Int(500_000)),
        name: "Baudrate".to_string(),
        kind: AttrType::Int,
        int_min: Some(1),
        int_max: Some(1_000_000),
        ..Default::default()
    };
    db.attr_spec.insert("Baudrate".to_string(), baudrate_spec);

    db.attributes
        .insert("Baudrate".to_string(), AttributeValue::Int(500_000));

    // Fill in default Baudrate for CANFD and its definition (only if BusType==CanFd)
    if bustype == BusType::CanFd {
        let baudrate_canfd_spec: AttributeSpec = AttributeSpec {
            type_of_object: AttrObject::Database,
            default: Some(AttributeValue::Int(500_000)),
            name: "BaudrateCANFD".to_string(),
            kind: AttrType::Int,
            int_min: Some(1),
            int_max: Some(16_000_000),
            ..Default::default()
        };
        db.attr_spec
            .insert("BaudrateCANFD".to_string(), baudrate_canfd_spec);

        db.attributes
            .insert("BaudrateCANFD".to_string(), AttributeValue::Int(2_000_000));
    }

    // Take current time values
    let now: DateTime<Local> = Local::now();
    let date: NaiveDate = now.date_naive();

    let day_of_month: u32 = date.day();
    let iso: IsoWeek = date.iso_week();
    let week_of_year_iso: u32 = iso.week();
    let month: u32 = date.month();
    let year: i32 = date.year();
    let year_last_2digit: i32 = year % 100;

    // Fill in default VersionDay
    let versionday_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Int(30)),
        name: "VersionDay".to_string(),
        kind: AttrType::Int,
        int_min: Some(1),
        int_max: Some(31),
        ..Default::default()
    };
    db.attr_spec
        .insert("VersionDay".to_string(), versionday_spec);

    db.attributes.insert(
        "VersionDay".to_string(),
        AttributeValue::Int(day_of_month as i64),
    );

    // Fill in default VersionMonth
    let version_month_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Int(4)),
        name: "VersionMonth".to_string(),
        kind: AttrType::Int,
        int_min: Some(1),
        int_max: Some(12),
        ..Default::default()
    };
    db.attr_spec
        .insert("VersionMonth".to_string(), version_month_spec);

    db.attributes.insert(
        "VersionMonth".to_string(),
        AttributeValue::Int(month as i64),
    );

    // Fill in default VersionWeek
    let version_week_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Int(18)),
        name: "VersionWeek".to_string(),
        kind: AttrType::Int,
        int_min: Some(1),
        int_max: Some(52),
        ..Default::default()
    };
    db.attr_spec
        .insert("VersionWeek".to_string(), version_week_spec);

    db.attributes.insert(
        "VersionWeek".to_string(),
        AttributeValue::Int(week_of_year_iso as i64),
    );

    // Fill in default VersionYear
    let version_year_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Int(25)),
        name: "VersionYear".to_string(),
        kind: AttrType::Int,
        int_min: Some(1),
        int_max: Some(99),
        ..Default::default()
    };
    db.attr_spec
        .insert("VersionYear".to_string(), version_year_spec);

    db.attributes.insert(
        "VersionYear".to_string(),
        AttributeValue::Int(year_last_2digit as i64),
    );

    // Fill in default VersionNumber
    let version_number_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Int(1)),
        name: "VersionNumber".to_string(),
        kind: AttrType::Int,
        int_min: Some(1),
        int_max: Some(65535),
        ..Default::default()
    };
    db.attr_spec
        .insert("VersionNumber".to_string(), version_number_spec);

    db.attributes
        .insert("VersionNumber".to_string(), AttributeValue::Int(1));

    // Fill in default Manufacturer
    let manufacturer_spec: AttributeSpec = AttributeSpec {
        type_of_object: AttrObject::Database,
        default: Some(AttributeValue::Str("".to_string())),
        name: "Manufacturer".to_string(),
        kind: AttrType::String,
        ..Default::default()
    };
    db.attr_spec
        .insert("Manufacturer".to_string(), manufacturer_spec);

    db.attributes.insert(
        "Manufacturer".to_string(),
        AttributeValue::Str("".to_string()),
    );

    Ok(db)
}
