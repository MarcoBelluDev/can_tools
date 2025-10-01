/// Attribute value kinds as declared by `BA_DEF_` lines in DBC.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttrType {
    #[default]
    String,
    Int,
    Hex,
    Float,
    Enum,
}

/// Attribute definition (declared by `BA_DEF_`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttributeDef {
    /// Attribute name.
    pub name: String,
    /// Attribute kind.
    pub kind: AttrType,
    // optional fields for numbers
    pub int_min: Option<i64>,
    pub int_max: Option<i64>,
    pub hex_min: Option<u64>,
    pub hex_max: Option<u64>,
    pub float_min: Option<f64>,
    pub float_max: Option<f64>,
    // optional vec<String> for enum entries
    pub enum_values: Vec<String>,
}

/// Concrete attribute value stored on DB/Node/Message/Signal entities.
#[derive(Clone, Debug, PartialEq)]
pub enum AttributeValue {
    Str(String),
    Int(i64),
    Hex(u64), // memorize as a number, proper display later.
    Float(f64),
    Enum(String),
}

/// Attribute specification pairing an optional definition and a default value.
///
/// - `def` comes from `BA_DEF_`
/// - `default` comes from `BA_DEF_DEF_`
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttributeSpec {
    pub def: Option<AttributeDef>,       // from BA_DEF_
    pub default: Option<AttributeValue>, // from BA_DEF_DEF_
}
