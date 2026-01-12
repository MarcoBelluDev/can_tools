use slotmap::Key;
use std::collections::BTreeMap;
use std::fmt::{self, Write as FmtWrite};
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

use crate::types::attributes::AttrObject;
use crate::types::{
    attributes::{AttrValueType, AttributeSpec, AttributeValue},
    database::{CanDatabase, CanSignalKey},
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

const AUTONET_FAKE_NODE: &str = "AutoNet_XXX";
const AUTONET_FAKE_MSG_NAME: &str = "AUTONET__INDEPENDENT_SIG_MSG";
const AUTONET_FAKE_MSG_ID: u32 = 3_221_225_479;

/// Serializes a `CanDatabase` into DBC text and writes it to `path`.
///
/// Ensures the destination has a `.dbc` extension, creates intermediate
/// directories when needed, and reports structured `DbcSaveError` variants
/// for path, I/O, or formatting failures.
pub fn save_to_file(path: &str, database: &CanDatabase) -> Result<(), DbcSaveError> {
    if !path.to_ascii_lowercase().ends_with(".dbc") {
        return Err(DbcSaveError::InvalidExtension {
            path: path.to_string(),
        });
    }

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
    serialize_database(database, &mut writer).map_err(|source| DbcSaveError::Write {
        path: path.to_string(),
        source,
    })?;
    writer.flush().map_err(|source| DbcSaveError::Write {
        path: path.to_string(),
        source,
    })?;
    Ok(())
}

/// Serializes the database into raw DBC text using the provided writer.
fn serialize_database<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
    let version = escape_dbc_string(&db.version);
    write_fmt(out, format_args!("VERSION \"{}\"\n\n", version))?;

    write_fmt(out, format_args!("NS_ :\n"))?;
    for keyword in NS_KEYWORDS {
        write_fmt(out, format_args!("\t{}\n", keyword))?;
    }
    write_fmt(out, format_args!("\n"))?;

    write_fmt(out, format_args!("BS_:\n\n"))?;

    write_fmt(out, format_args!("BU_:"))?;
    for node in db.iter_nodes() {
        write_fmt(out, format_args!(" {}", node.name))?;
    }
    write_fmt(out, format_args!("\n\n"))?;

    let independent: Vec<CanSignalKey> = collect_independent_signals(db);
    write_independent_signals_as_fake_message(db, &independent, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_messages(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_bo_tx_bu(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_comments(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_attribute_definitions(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_relation_attribute_definitions(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_attribute_defaults(db, out)?;
    write_relation_attribute_defaults(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_attribute_assignments(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_relation_attribute_assignments(db, out)?;
    write_fmt(out, format_args!("\n"))?;

    write_sig_valtype(db, out)?;
    write_value_tables(db, out)?;

    Ok(())
}

/// Writes each message and its signals into standard DBC syntax.
fn write_messages<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
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

        write_fmt(out, format_args!("\n"))?;
    }

    Ok(())
}

/// Emits `BO_TX_BU_` entries describing message transmitters.
fn write_bo_tx_bu<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
    for message in db.iter_messages() {
        let mut transmitters: Vec<String> = Vec::new();
        for nk in &message.sender_nodes {
            if let Some(node) = db.get_node_by_key(*nk)
                && !transmitters.iter().any(|name| name == &node.name)
            {
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

/// Outputs attribute definitions for database, node, message, and signal scopes.
fn write_attribute_definitions<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
    // DB
    for (name, spec) in db
        .attr_spec
        .iter()
        .filter(|(_, s)| s.type_of_object == AttrObject::Database)
    {
        let signature: String = format_attribute_spec(spec);
        write_fmt(out, format_args!("BA_DEF_ \"{}\" {};\n", name, signature))?;
    }

    // BU_
    for (name, spec) in db
        .attr_spec
        .iter()
        .filter(|(_, s)| s.type_of_object == AttrObject::Node)
    {
        let signature: String = format_attribute_spec(spec);
        write_fmt(
            out,
            format_args!("BA_DEF_ BU_ \"{}\" {};\n", name, signature),
        )?;
    }

    // BO_
    for (name, spec) in db
        .attr_spec
        .iter()
        .filter(|(_, s)| s.type_of_object == AttrObject::Message)
    {
        let signature: String = format_attribute_spec(spec);
        write_fmt(
            out,
            format_args!("BA_DEF_ BO_ \"{}\" {};\n", name, signature),
        )?;
    }

    // SG_
    for (name, spec) in db
        .attr_spec
        .iter()
        .filter(|(_, s)| s.type_of_object == AttrObject::Signal)
    {
        let signature: String = format_attribute_spec(spec);
        write_fmt(
            out,
            format_args!("BA_DEF_ SG_ \"{}\" {};\n", name, signature),
        )?;
    }

    Ok(())
}

/// Outputs attribute definitions for relation-scoped attributes.
fn write_relation_attribute_definitions<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
    for (name, spec) in &db.rel_attr_spec_bu_sg {
        let signature: String = format_attribute_spec(spec);
        write_fmt(
            out,
            format_args!("BA_DEF_REL_ BU_SG_REL_ \"{}\" {};\n", name, signature),
        )?;
    }

    for (name, spec) in &db.rel_attr_spec_bu_bo {
        let signature: String = format_attribute_spec(spec);
        write_fmt(
            out,
            format_args!("BA_DEF_REL_ BU_BO_REL_ \"{}\" {};\n", name, signature),
        )?;
    }

    Ok(())
}

/// Writes the default values for each scoped attribute.
fn write_attribute_defaults<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
    let mut defaults: BTreeMap<String, AttributeValue> = BTreeMap::new();

    collect_defaults_from_scope(db, AttrObject::Database, &mut defaults);
    collect_defaults_from_scope(db, AttrObject::Node, &mut defaults);
    collect_defaults_from_scope(db, AttrObject::Message, &mut defaults);
    collect_defaults_from_scope(db, AttrObject::Signal, &mut defaults);

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

/// Writes default values for relation-scoped attributes.
fn write_relation_attribute_defaults<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
    let mut defaults: BTreeMap<String, AttributeValue> = BTreeMap::new();

    collect_defaults_from_scope(db, AttrObject::Message, &mut defaults);
    collect_defaults_from_scope(db, AttrObject::Signal, &mut defaults);

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

/// Emits attribute assignments for databases, nodes, messages, and signals.
fn write_attribute_assignments<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
    for (name, value) in &db.attributes {
        let spec = db.attr_spec.get(name);
        let value_str = format_attribute_value(value, spec);
        write_fmt(out, format_args!("BA_ \"{}\" {};\n", name, value_str))?;
    }

    for node in db.iter_nodes() {
        for (name, value) in &node.attributes {
            let spec = db.attr_spec.get(name);
            let value_str = format_attribute_value(value, spec);
            write_fmt(
                out,
                format_args!("BA_ \"{}\" BU_ {} {};\n", name, node.name, value_str),
            )?;
        }
    }

    for message in db.iter_messages() {
        for (name, value) in &message.attributes {
            let spec = db.attr_spec.get(name);
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
                    let spec = db.attr_spec.get(name);
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

/// Emits `BA_REL_` statements for relation-scoped attribute assignments.
fn write_relation_attribute_assignments<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
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

/// Writes `CM_` comment blocks for database items.
fn write_comments<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
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

/// Emits `SIG_VALTYPE_` lines for floating-point signals.
fn write_sig_valtype<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
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

/// Outputs `VAL_` tables for enumerated signal values.
fn write_value_tables<W: Write>(db: &CanDatabase, out: &mut W) -> io::Result<()> {
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
                write_fmt(out, format_args!(" ;\n"))?;
            }
        }
    }

    Ok(())
}

/// Produces the multiplexing tag used in `SG_` lines.
fn format_mux_tag(signal: &crate::types::signal::CanSignal) -> String {
    match signal.mux_role {
        MuxRole::Multiplexor => " M".to_string(),
        MuxRole::Multiplexed => match signal.mux_selector {
            MuxSelector::Value(v) => format!(" m{}", v),
            MuxSelector::Range { min, max } => format!(" m{}-{}", min, max),
        },
        MuxRole::None => String::new(),
    }
}

/// Converts an attribute definition into its signature text.
fn format_attribute_spec(spec: &AttributeSpec) -> String {
    match spec.value_type {
        AttrValueType::String => "STRING".to_string(),
        AttrValueType::Int => format!(
            "INT {} {}",
            spec.int_min.unwrap_or_default(),
            spec.int_max.unwrap_or_default()
        ),
        AttrValueType::Hex => format!(
            "HEX {} {}",
            spec.hex_min.unwrap_or_default(),
            spec.hex_max.unwrap_or_default()
        ),
        AttrValueType::Float => format!(
            "FLOAT {} {}",
            format_f64(spec.float_min.unwrap_or_default()),
            format_f64(spec.float_max.unwrap_or_default())
        ),
        AttrValueType::Enum => {
            let joined = spec
                .enum_values
                .iter()
                .map(|value| format!("\"{}\"", escape_dbc_string(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("ENUM {}", joined)
        }
    }
}

/// Formats an attribute value using optional spec information.
fn format_attribute_value(value: &AttributeValue, spec: Option<&AttributeSpec>) -> String {
    match value {
        AttributeValue::Str(s) => format!("\"{}\"", escape_dbc_string(s)),
        AttributeValue::Int(v) => v.to_string(),
        AttributeValue::Hex(v) => v.to_string(),
        AttributeValue::Float(v) => format_f64(*v),
        AttributeValue::Enum(selected) => {
            if let Some(spec) = spec.filter(|s| matches!(s.value_type, AttrValueType::Enum))
                && let Some(idx) = spec.enum_values.iter().position(|entry| entry == selected)
            {
                return idx.to_string();
            }
            format!("\"{}\"", escape_dbc_string(selected))
        }
    }
}

/// Formats floating-point values while stripping redundant trailing zeros.
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

/// Escapes characters so they are safe inside DBC quoted strings.
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

/// Collects default attribute values across scopes into a single map.
fn collect_defaults_from_scope(
    db: &CanDatabase,
    scope: AttrObject,
    target: &mut BTreeMap<String, AttributeValue>,
) {
    for (name, spec) in db
        .attr_spec
        .iter()
        .filter(|(_, s)| s.type_of_object == scope)
    {
        // first wins (stesso comportamento della tua versione a 4 mappe)
        target
            .entry(name.clone())
            .or_insert_with(|| spec.default.clone());
    }
}
/// Looks up an attribute specification regardless of its scope.
fn lookup_attr_spec<'a>(db: &'a CanDatabase, name: &str) -> Option<&'a AttributeSpec> {
    db.attr_spec.get(name)
}

/// Writes formatted arguments to the writer while preserving `io::Error` details.
struct IoWriteAdapter<'a, W: Write> {
    inner: &'a mut W,
    error: Option<io::Error>,
}

impl<'a, W: Write> FmtWrite for IoWriteAdapter<'a, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Err(err) = self.inner.write_all(s.as_bytes()) {
            self.error = Some(err);
            return Err(fmt::Error);
        }
        Ok(())
    }
}

/// Writes formatted arguments to any writer, propagating the underlying I/O error.
fn write_fmt<W: Write>(out: &mut W, args: fmt::Arguments<'_>) -> io::Result<()> {
    let mut adapter = IoWriteAdapter {
        inner: out,
        error: None,
    };
    match fmt::write(&mut adapter, args) {
        Ok(()) => Ok(()),
        Err(_) => Err(adapter
            .error
            .unwrap_or_else(|| io::Error::other("formatting error"))),
    }
}

/// Filters out signals that are not assigned to a message.
fn collect_independent_signals(db: &CanDatabase) -> Vec<CanSignalKey> {
    db.signals_order
        .iter()
        .filter_map(|&key| db.get_sig_by_key(key).map(|sig| (key, sig)))
        .filter(|(_, sig)| sig.message.is_null())
        .map(|(key, _)| key)
        .collect()
}

/// Synthesizes a fake message containing independent signals for export.
fn write_independent_signals_as_fake_message<W: Write>(
    db: &CanDatabase,
    orphans: &[CanSignalKey],
    out: &mut W,
) -> io::Result<()> {
    if orphans.is_empty() {
        return Ok(());
    }

    write_fmt(
        out,
        format_args!(
            "BO_ {} {}: {} {}\n",
            AUTONET_FAKE_MSG_ID, AUTONET_FAKE_MSG_NAME, 0, AUTONET_FAKE_NODE
        ),
    )?;

    for sig_key in orphans {
        let Some(signal) = db.get_sig_by_key(*sig_key) else {
            continue;
        };
        let mux_tag: String = format_mux_tag(signal);
        let endian: char = if matches!(signal.endian, Endianness::Intel) {
            '1'
        } else {
            '0'
        };
        let sign_char: char = match signal.sign {
            Signess::Signed => '-',
            _ => '+',
        };
        let factor: String = format_f64(signal.factor);
        let offset: String = format_f64(signal.offset);
        let min: String = format_f64(signal.min);
        let max: String = format_f64(signal.max);
        let unit: String = escape_dbc_string(&signal.unit_of_measurement);

        // Receiver: use existing Node receivers, otherwise use AutoNet_XXX
        let receivers: Vec<String> = signal
            .receiver_nodes
            .iter()
            .filter_map(|nk| db.get_node_by_key(*nk).map(|n| n.name.clone()))
            .collect();
        let receivers_field = if receivers.is_empty() {
            AUTONET_FAKE_NODE.to_string()
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

    write_fmt(out, format_args!("\n"))?;
    Ok(())
}
