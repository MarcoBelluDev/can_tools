use slotmap::Key;
use std::collections::BTreeMap;
use std::fmt::{self, Write as FmtWrite};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::dbc::types::{
    attributes::{AttrType, AttributeDef, AttributeSpec, AttributeValue},
    database::DatabaseDBC,
    errors::DbcSaveError,
    message::{MuxRole, MuxSelector},
    signal::{Endianness, Signess},
};

const NS_KEYWORDS: &[&str] = &[
    "NS_DESC_",
    "CM_",
    "BA_DEF_",
    "BA_",
    "VAL_",
    "CAT_DEF_",
    "CAT_",
    "FILTER",
    "BA_DEF_DEF_",
    "EV_DATA_",
    "ENVVAR_DATA_",
    "SGTYPE_",
    "SGTYPE_VAL_",
    "BA_DEF_SGTYPE_",
    "BA_SGTYPE_",
    "SIG_TYPE_REF_",
    "VAL_TABLE_",
    "SIG_GROUP_",
    "SIG_VALTYPE_",
    "SIGTYPE_VALTYPE_",
    "BO_TX_BU_",
    "BA_DEF_REL_",
    "BA_REL_",
    "BA_DEF_DEF_REL_",
    "BU_SG_REL_",
    "BU_EV_REL_",
    "BU_BO_REL_",
];

/// Serializes a `DatabaseDBC` into DBC text and writes it to `path`.
///
/// Ensures the destination has a `.dbc` extension, creates intermediate
/// directories when needed, and reports structured `DbcSaveError` variants
/// for path, I/O, or formatting failures.
pub fn save_to_file(path: &str, database: &DatabaseDBC) -> Result<(), DbcSaveError> {
    if !path.to_ascii_lowercase().ends_with(".dbc") {
        return Err(DbcSaveError::InvalidExtension {
            path: path.to_string(),
        });
    }

    let serialized: String = serialize_database(database)?;

    let path_ref: &Path = Path::new(path);
    if let Some(parent) = path_ref.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| DbcSaveError::CreateDirectory {
            path: parent.display().to_string(),
            source,
        })?;
    }

    let file = File::create(path_ref).map_err(|source| DbcSaveError::CreateFile {
        path: path.to_string(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    writer
        .write_all(serialized.as_bytes())
        .map_err(|source| DbcSaveError::Write {
            path: path.to_string(),
            source,
        })?;
    writer.flush().map_err(|source| DbcSaveError::Write {
        path: path.to_string(),
        source,
    })?;
    Ok(())
}

fn serialize_database(db: &DatabaseDBC) -> Result<String, DbcSaveError> {
    let mut out = String::new();

    let version = escape_dbc_string(&db.version);
    write_fmt(&mut out, format_args!("VERSION \"{}\"\n\n", version))?;

    out.push_str("NS_ :\n");
    for keyword in NS_KEYWORDS {
        out.push('\t');
        out.push_str(keyword);
        out.push('\n');
    }
    out.push('\n');

    out.push_str("BS_:\n\n");

    out.push_str("BU_:");
    for node in db.iter_nodes() {
        out.push(' ');
        out.push_str(&node.name);
    }
    out.push('\n');
    out.push('\n');

    write_messages(db, &mut out)?;
    out.push('\n');

    write_bo_tx_bu(db, &mut out)?;
    out.push('\n');

    write_attribute_definitions(db, &mut out)?;
    out.push('\n');

    write_relation_attribute_definitions(db, &mut out)?;
    out.push('\n');

    write_attribute_defaults(db, &mut out)?;
    write_relation_attribute_defaults(db, &mut out)?;
    out.push('\n');

    write_attribute_assignments(db, &mut out)?;
    out.push('\n');

    write_relation_attribute_assignments(db, &mut out)?;
    out.push('\n');

    write_comments(db, &mut out)?;
    out.push('\n');

    write_sig_valtype(db, &mut out)?;
    write_value_tables(db, &mut out)?;

    Ok(out)
}

fn write_messages(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    for message in db.iter_messages() {
        let transmitter = message
            .sender_nodes
            .iter()
            .find_map(|nk| db.get_node_by_key(*nk).map(|node| node.name.as_str()))
            .unwrap_or("Vector__XXX");

        write_fmt(
            out,
            format_args!(
                "BO_ {} {}: {} {}\n",
                message.id, message.name, message.byte_length, transmitter
            ),
        )?;

        for sig_key in &message.signals {
            if let Some(signal) = db.get_sig_by_key(*sig_key) {
                let mux_tag = format_mux_tag(signal);
                let endian = if matches!(signal.endian, Endianness::Intel) {
                    '1'
                } else {
                    '0'
                };
                let sign_char = match signal.sign {
                    Signess::Signed => '-',
                    _ => '+',
                };
                let factor = format_f64(signal.factor);
                let offset = format_f64(signal.offset);
                let min = format_f64(signal.min);
                let max = format_f64(signal.max);
                let unit = escape_dbc_string(&signal.unit_of_measurement);
                let receivers: Vec<String> = signal
                    .receiver_nodes
                    .iter()
                    .filter_map(|nk| db.get_node_by_key(*nk).map(|node| node.name.clone()))
                    .collect();
                let receivers_field = if receivers.is_empty() {
                    "Vector__XXX".to_string()
                } else {
                    receivers.join(",")
                };

                write_fmt(
                    out,
                    format_args!(
                        "\tSG_ {}{} : {}|{}@{}{} ({},{}) [{}|{}] \"{}\"  {}\n",
                        signal.name,
                        mux_tag,
                        signal.bit_start,
                        signal.bit_length,
                        endian,
                        sign_char,
                        factor,
                        offset,
                        min,
                        max,
                        unit,
                        receivers_field
                    ),
                )?;
            }
        }

        out.push('\n');
    }

    Ok(())
}

fn write_bo_tx_bu(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    for message in db.iter_messages() {
        let mut transmitters: Vec<String> = Vec::new();
        for nk in &message.sender_nodes {
            if let Some(node) = db.get_node_by_key(*nk)
                && !transmitters.iter().any(|name| name == &node.name) {
                    transmitters.push(node.name.clone());
                }
        }

        if transmitters.is_empty() {
            continue;
        }

        write_fmt(
            out,
            format_args!("BO_TX_BU_ {} :{};\n", message.id, transmitters.join(",")),
        )?;
    }

    Ok(())
}

fn write_attribute_definitions(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    for (name, spec) in &db.db_attr_spec {
        if let Some(def) = spec.def.as_ref() {
            let signature = format_attribute_def(def);
            write_fmt(out, format_args!("BA_DEF_ \"{}\" {};\n", name, signature))?;
        }
    }

    for (name, spec) in &db.node_attr_spec {
        if let Some(def) = spec.def.as_ref() {
            let signature = format_attribute_def(def);
            write_fmt(
                out,
                format_args!("BA_DEF_ BU_ \"{}\" {};\n", name, signature),
            )?;
        }
    }

    for (name, spec) in &db.msg_attr_spec {
        if let Some(def) = spec.def.as_ref() {
            let signature = format_attribute_def(def);
            write_fmt(
                out,
                format_args!("BA_DEF_ BO_ \"{}\" {};\n", name, signature),
            )?;
        }
    }

    for (name, spec) in &db.sig_attr_spec {
        if let Some(def) = spec.def.as_ref() {
            let signature = format_attribute_def(def);
            write_fmt(
                out,
                format_args!("BA_DEF_ SG_ \"{}\" {};\n", name, signature),
            )?;
        }
    }

    Ok(())
}

fn write_relation_attribute_definitions(
    db: &DatabaseDBC,
    out: &mut String,
) -> Result<(), DbcSaveError> {
    for (name, spec) in &db.rel_attr_spec_bu_sg {
        if let Some(def) = spec.def.as_ref() {
            let signature = format_attribute_def(def);
            write_fmt(
                out,
                format_args!("BA_DEF_REL_ BU_SG_REL_ \"{}\" {};\n", name, signature),
            )?;
        }
    }

    for (name, spec) in &db.rel_attr_spec_bu_bo {
        if let Some(def) = spec.def.as_ref() {
            let signature = format_attribute_def(def);
            write_fmt(
                out,
                format_args!("BA_DEF_REL_ BU_BO_REL_ \"{}\" {};\n", name, signature),
            )?;
        }
    }

    Ok(())
}

fn write_attribute_defaults(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    let mut defaults: BTreeMap<String, AttributeValue> = BTreeMap::new();

    collect_defaults(&db.db_attr_spec, &mut defaults);
    collect_defaults(&db.node_attr_spec, &mut defaults);
    collect_defaults(&db.msg_attr_spec, &mut defaults);
    collect_defaults(&db.sig_attr_spec, &mut defaults);

    for (name, value) in defaults {
        let spec = lookup_attr_spec(db, &name);
        let value_str = format_attribute_value(&value, spec);
        write_fmt(
            out,
            format_args!("BA_DEF_DEF_ \"{}\" {};\n", name, value_str),
        )?;
    }

    Ok(())
}

fn write_relation_attribute_defaults(
    db: &DatabaseDBC,
    out: &mut String,
) -> Result<(), DbcSaveError> {
    let mut defaults: BTreeMap<String, AttributeValue> = BTreeMap::new();

    collect_defaults(&db.rel_attr_spec_bu_sg, &mut defaults);
    collect_defaults(&db.rel_attr_spec_bu_bo, &mut defaults);

    for (name, value) in defaults {
        let spec = db
            .rel_attr_spec_bu_sg
            .get(&name)
            .or_else(|| db.rel_attr_spec_bu_bo.get(&name));
        let value_str = format_attribute_value(&value, spec);
        write_fmt(
            out,
            format_args!("BA_DEF_DEF_REL_ \"{}\" {};\n", name, value_str),
        )?;
    }

    Ok(())
}

fn write_attribute_assignments(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    for (name, value) in &db.attributes {
        let spec = db.db_attr_spec.get(name);
        let value_str = format_attribute_value(value, spec);
        write_fmt(out, format_args!("BA_ \"{}\" {};\n", name, value_str))?;
    }

    for node in db.iter_nodes() {
        for (name, value) in &node.attributes {
            let spec = db.node_attr_spec.get(name);
            let value_str = format_attribute_value(value, spec);
            write_fmt(
                out,
                format_args!("BA_ \"{}\" BU_ {} {};\n", name, node.name, value_str),
            )?;
        }
    }

    for message in db.iter_messages() {
        for (name, value) in &message.attributes {
            let spec = db.msg_attr_spec.get(name);
            let value_str = format_attribute_value(value, spec);
            write_fmt(
                out,
                format_args!("BA_ \"{}\" BO_ {} {};\n", name, message.id, value_str),
            )?;
        }
    }

    for message in db.iter_messages() {
        for sig_key in &message.signals {
            if let Some(signal) = db.get_sig_by_key(*sig_key) {
                for (name, value) in &signal.attributes {
                    let spec = db.sig_attr_spec.get(name);
                    let value_str = format_attribute_value(value, spec);
                    write_fmt(
                        out,
                        format_args!(
                            "BA_ \"{}\" SG_ {} {} {};\n",
                            name, message.id, signal.name, value_str
                        ),
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn write_relation_attribute_assignments(
    db: &DatabaseDBC,
    out: &mut String,
) -> Result<(), DbcSaveError> {
    let mut bu_sg_entries: Vec<(String, u32, String, &BTreeMap<String, AttributeValue>)> =
        Vec::new();
    for ((node_key, sig_key), attrs) in &db.bu_sg_rel_attributes {
        let (Some(node), Some(signal)) =
            (db.get_node_by_key(*node_key), db.get_sig_by_key(*sig_key))
        else {
            continue;
        };
        if signal.message.is_null() {
            continue;
        }
        let Some(message) = db.get_message_by_key(signal.message) else {
            continue;
        };
        bu_sg_entries.push((node.name.clone(), message.id, signal.name.clone(), attrs));
    }
    bu_sg_entries.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    for (node_name, msg_id, signal_name, attrs) in bu_sg_entries {
        for (attr_name, value) in attrs {
            let spec = db.rel_attr_spec_bu_sg.get(attr_name);
            let value_str = format_attribute_value(value, spec);
            write_fmt(
                out,
                format_args!(
                    "BA_REL_ \"{}\" BU_SG_REL_ {} SG_ {} {} {};\n",
                    attr_name, node_name, msg_id, signal_name, value_str
                ),
            )?;
        }
    }

    let mut bu_bo_entries: Vec<(String, u32, &BTreeMap<String, AttributeValue>)> = Vec::new();
    for ((node_key, msg_key), attrs) in &db.bu_bo_rel_attributes {
        let (Some(node), Some(message)) = (
            db.get_node_by_key(*node_key),
            db.get_message_by_key(*msg_key),
        ) else {
            continue;
        };
        bu_bo_entries.push((node.name.clone(), message.id, attrs));
    }
    bu_bo_entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    for (node_name, msg_id, attrs) in bu_bo_entries {
        for (attr_name, value) in attrs {
            let spec = db.rel_attr_spec_bu_bo.get(attr_name);
            let value_str = format_attribute_value(value, spec);
            write_fmt(
                out,
                format_args!(
                    "BA_REL_ \"{}\" BU_BO_REL_ {} BO_ {} {};\n",
                    attr_name, node_name, msg_id, value_str
                ),
            )?;
        }
    }

    Ok(())
}

fn write_comments(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    if !db.comment.is_empty() {
        let comment = escape_dbc_string(&db.comment);
        write_fmt(out, format_args!("CM_ \"{}\";\n", comment))?;
    }

    for node in db.iter_nodes() {
        if node.comment.is_empty() {
            continue;
        }
        let comment = escape_dbc_string(&node.comment);
        write_fmt(
            out,
            format_args!("CM_ BU_ {} \"{}\";\n", node.name, comment),
        )?;
    }

    for message in db.iter_messages() {
        if message.comment.is_empty() {
            continue;
        }
        let comment = escape_dbc_string(&message.comment);
        write_fmt(
            out,
            format_args!("CM_ BO_ {} \"{}\";\n", message.id, comment),
        )?;
    }

    for message in db.iter_messages() {
        for sig_key in &message.signals {
            if let Some(signal) = db.get_sig_by_key(*sig_key)
                && !signal.comment.is_empty()
            {
                let comment = escape_dbc_string(&signal.comment);
                write_fmt(
                    out,
                    format_args!("CM_ SG_ {} {} \"{}\";\n", message.id, signal.name, comment),
                )?;
            }
        }
    }

    Ok(())
}

fn write_sig_valtype(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    for message in db.iter_messages() {
        for sig_key in &message.signals {
            if let Some(signal) = db.get_sig_by_key(*sig_key) {
                let value = match signal.sign {
                    Signess::IeeeFloat => Some(1),
                    Signess::IeeeDouble => Some(2),
                    _ => None,
                };
                if let Some(code) = value {
                    write_fmt(
                        out,
                        format_args!("SIG_VALTYPE_ {} {} : {};\n", message.id, signal.name, code),
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn write_value_tables(db: &DatabaseDBC, out: &mut String) -> Result<(), DbcSaveError> {
    for message in db.iter_messages() {
        for sig_key in &message.signals {
            if let Some(signal) = db.get_sig_by_key(*sig_key)
                && !signal.value_table.is_empty()
            {
                write_fmt(out, format_args!("VAL_ {} {}", message.id, signal.name))?;
                for (value, description) in &signal.value_table {
                    let desc = escape_dbc_string(description);
                    write_fmt(out, format_args!(" {} \"{}\"", value, desc))?;
                }
                out.push_str(" ;\n");
            }
        }
    }

    Ok(())
}

fn format_mux_tag(signal: &crate::dbc::types::signal::SignalDBC) -> String {
    match signal.mux_role {
        MuxRole::Multiplexor => " M".to_string(),
        MuxRole::Multiplexed => match signal.mux_selector {
            MuxSelector::Value(v) => format!(" m{}", v),
            MuxSelector::Range { min, max } => format!(" m{}-{}", min, max),
        },
        MuxRole::None => String::new(),
    }
}

fn format_attribute_def(def: &AttributeDef) -> String {
    match def.kind {
        AttrType::String => "STRING".to_string(),
        AttrType::Int => format!(
            "INT {} {}",
            def.int_min.unwrap_or_default(),
            def.int_max.unwrap_or_default()
        ),
        AttrType::Hex => format!(
            "HEX {} {}",
            def.hex_min.unwrap_or_default(),
            def.hex_max.unwrap_or_default()
        ),
        AttrType::Float => format!(
            "FLOAT {} {}",
            format_f64(def.float_min.unwrap_or_default()),
            format_f64(def.float_max.unwrap_or_default())
        ),
        AttrType::Enum => {
            let joined = def
                .enum_values
                .iter()
                .map(|value| format!("\"{}\"", escape_dbc_string(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("ENUM {}", joined)
        }
    }
}

fn format_attribute_value(value: &AttributeValue, spec: Option<&AttributeSpec>) -> String {
    match value {
        AttributeValue::Str(s) => format!("\"{}\"", escape_dbc_string(s)),
        AttributeValue::Int(v) => v.to_string(),
        AttributeValue::Hex(v) => v.to_string(),
        AttributeValue::Float(v) => format_f64(*v),
        AttributeValue::Enum(selected) => {
            if let Some(spec) = spec
                .and_then(|s| s.def.as_ref())
                .filter(|def| matches!(def.kind, AttrType::Enum))
                && let Some(idx) = spec.enum_values.iter().position(|entry| entry == selected) {
                    return idx.to_string();
                }
            format!("\"{}\"", escape_dbc_string(selected))
        }
    }
}

fn format_f64(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{:.0}", value)
    } else {
        let mut s = format!("{:.12}", value);
        while s.contains('.') && s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.push('0');
        }
        s
    }
}

fn escape_dbc_string(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn collect_defaults(
    source: &BTreeMap<String, AttributeSpec>,
    target: &mut BTreeMap<String, AttributeValue>,
) {
    for (name, spec) in source {
        if let Some(default) = spec.default.clone() {
            target.entry(name.clone()).or_insert(default);
        }
    }
}

fn lookup_attr_spec<'a>(db: &'a DatabaseDBC, name: &str) -> Option<&'a AttributeSpec> {
    db.db_attr_spec
        .get(name)
        .or_else(|| db.node_attr_spec.get(name))
        .or_else(|| db.msg_attr_spec.get(name))
        .or_else(|| db.sig_attr_spec.get(name))
}

fn write_fmt(out: &mut String, args: fmt::Arguments<'_>) -> Result<(), DbcSaveError> {
    out.write_fmt(args).map_err(|_| DbcSaveError::Format)
}
