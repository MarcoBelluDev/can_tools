use std::fmt;

/// Attribute specification pairing an optional definition and a default value.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttributeSpec {
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
    pub default: Option<AttributeValue>, // from BA_DEF_DEF_
    pub type_of_object: AttrObject,
}
impl AttributeSpec {
    pub fn minimum_to_string(&self) -> String {
        match self.kind {
            AttrType::String | AttrType::Enum => String::new(),

            AttrType::Int => match self.int_min {
                Some(v) => v.to_string(),
                None => String::new(),
            },

            AttrType::Hex => match self.hex_min {
                Some(v) => format!("0x{:X}", v),
                None => String::new(),
            },

            AttrType::Float => match self.float_min {
                Some(v) => {
                    // stampa compatta tipo la tua Display
                    let mut s = v.to_string();
                    if s.contains('.') {
                        while s.ends_with('0') {
                            s.pop();
                        }
                        if s.ends_with('.') {
                            s.pop();
                        }
                    }
                    s
                }
                None => String::new(),
            },
        }
    }
    pub fn maximum_to_string(&self) -> String {
        match self.kind {
            AttrType::String | AttrType::Enum => String::new(),

            AttrType::Int => match self.int_max {
                Some(v) => v.to_string(),
                None => String::new(),
            },

            AttrType::Hex => match self.hex_max {
                Some(v) => format!("0x{:X}", v),
                None => String::new(),
            },

            AttrType::Float => match self.float_max {
                Some(v) => {
                    // stampa compatta tipo la tua Display
                    let mut s: String = v.to_string();
                    if s.contains('.') {
                        while s.ends_with('0') {
                            s.pop();
                        }
                        if s.ends_with('.') {
                            s.pop();
                        }
                    }
                    s
                }
                None => String::new(),
            },
        }
    }
    pub fn default_to_string(&self) -> String {
        match &self.default {
            Some(attr_value) => {
                match attr_value {
                    AttributeValue::Str(s) => s.to_string(),
                    AttributeValue::Int(v) => v.to_string(),
                    AttributeValue::Hex(v) => {
                        let s: String = format!("0x{:X}", v);
                        s
                    }
                    AttributeValue::Float(v) => {
                        // stampa compatta tipo la tua Display
                        let mut s: String = v.to_string();
                        if s.contains('.') {
                            while s.ends_with('0') {
                                s.pop();
                            }
                            if s.ends_with('.') {
                                s.pop();
                            }
                        }
                        s
                    }
                    AttributeValue::Enum(s) => s.to_string(),
                }
            }
            None => "".to_string(),
        }
    }
}

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
impl fmt::Display for AttrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AttrType::String => "String",
            AttrType::Int => "Int",
            AttrType::Hex => "Hex",
            AttrType::Float => "Float",
            AttrType::Enum => "Enum",
        })
    }
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
impl fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttributeValue::Str(s) => write!(f, "{}", s),
            AttributeValue::Int(i) => write!(f, "{}", i),
            AttributeValue::Hex(h) => write!(f, "0x{:X}", h),
            AttributeValue::Float(x) => {
                // stampa compatta, senza zeri finali superflui
                let mut s = format!("{}", x);
                if s.contains('.') {
                    while s.ends_with('0') {
                        s.pop();
                    }
                    if s.ends_with('.') {
                        s.pop();
                    }
                }
                write!(f, "{}", s)
            }
            AttributeValue::Enum(s) => write!(f, "{}", s),
        }
    }
}

/// The type of Object of the Attribute
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttrObject {
    #[default]
    Database,
    Node,
    Message,
    Signal,
}

impl fmt::Display for AttrObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AttrObject::Database => "Database",
            AttrObject::Node => "Node",
            AttrObject::Message => "Message",
            AttrObject::Signal => "Signal",
        })
    }
}
