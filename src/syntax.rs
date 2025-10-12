/*

expr := <term>
      | "(" <expr>")"
      | <function-call>
      | "{" <expr> (";" <expr>)*"}"
term := <factor>
      | <term> "+" <term>
      | <term> "-" <term>
factor := Int
        | Flt
        | IDENTIFIER
        | <factor> "*" <factor>
        | <factor> "/" <factor>
function-call := IDENTIFIER "(" <arg-list> ")"
arg-list := EPSILON
          | <expr> ("," <expr
*/
#![allow(unused_variables)]

use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Operator {
    Div,
    Mul,
    Add,
    Sub,
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
    And,
    Or,
    Not,
    Range,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub data_type: DataType,
    pub default: Option<Expr>,
    pub index: (usize, usize),
    pub copy: bool, // true = pass by value (mutable inside function), false = pass by reference (immutable)
}

#[derive(Clone, Debug, PartialEq)]
pub enum DataType {
    Unsolved,
    Optional(Box<DataType>),
    Range(Box<Expr>),
    Str,
    Int,
    Flt,
    Bool,
    Map {
        key_type: Box<DataType>,
        value_type: Box<DataType>,
    },
    List {
        element_type: Box<DataType>,
    },
    Set(Box<DataType>),
    Enum(Vec<String>),
    Struct(Vec<Param>),
    TypeRef(String), // Reference to a user-defined type
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeywordArg {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LiteralData {
    Int(i64),
    Flt(f64),
    Str(Rc<str>),
    Bool(bool),
}

impl From<KeyData> for LiteralData {
    fn from(data: KeyData) -> LiteralData {
        match data {
            KeyData::Str(s) => LiteralData::Str(s),
            KeyData::Bool(b) => LiteralData::Bool(b),
            KeyData::Int(i) => LiteralData::Int(i),
        }
    }
}

// This is for using as keys and pattern matching: Floats
// aren't able to act as HashMap keys and we wouldn't really
// want them to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum KeyData {
    Int(i64),
    Bool(bool),
    Str(Rc<str>),
}

impl From<LiteralData> for KeyData {
    fn from(data: LiteralData) -> KeyData {
        match data {
            LiteralData::Str(s) => KeyData::Str(s),
            LiteralData::Int(i) => KeyData::Int(i),
            LiteralData::Bool(b) => KeyData::Bool(b),
            _ => panic!("Error converting LiteralData to KeyData: \n'{:?}'. \nThis is likely a compiler bug and the conversion needs to be implemented or a type-check is missing.",&data),

        }
    }
}

impl From<&str> for LiteralData {
    fn from(data: &str) -> LiteralData {
        LiteralData::Str(data.into())
    }
}

impl From<String> for LiteralData {
    fn from(data: String) -> LiteralData {
        LiteralData::Str(data.into())
    }
}

impl From<i64> for LiteralData {
    fn from(data: i64) -> LiteralData {
        LiteralData::Int(data)
    }
}

impl From<f64> for LiteralData {
    fn from(data: f64) -> LiteralData {
        LiteralData::Flt(data)
    }
}

impl From<bool> for LiteralData {
    fn from(data: bool) -> LiteralData {
        LiteralData::Bool(data)
    }
}

impl From<Expr> for LiteralData {
    fn from(data: Expr) -> LiteralData {
        match data {
            Expr::Literal(l) => l,
            _ => panic!("Can only extract LiteralData from Expr::LiteralData."),
        }
    }
}

impl std::fmt::Display for LiteralData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralData::Int(i) => write!(f, "{i}"),
            LiteralData::Flt(fl) => write!(f, "{fl}"),
            LiteralData::Bool(b) => write!(f, "{b}"),
            LiteralData::Str(s) => write!(f, "{}", &s),
        }
    }
}

impl std::fmt::Display for KeyData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyData::Int(i) => write!(f, "{i}"),
            KeyData::Bool(b) => write!(f, "{b}"),
            KeyData::Str(s) => write!(f, "{}", &s),
        }
    }
}

/// Built-in methods for String, List, and Map types
#[derive(Clone, Debug, PartialEq)]
pub enum BuiltinMethod {
    // String methods (10)
    StrUpper,
    StrLower,
    StrSubstring,
    StrContains,
    StrTrim,
    StrSplit,
    StrReplace,
    StrStartsWith,
    StrEndsWith,
    StrIsEmpty,

    // List methods (7)
    ListFirst,
    ListLast,
    ListContains,
    ListSlice,
    ListReverse,
    ListJoin,
    ListIsEmpty,

    // Map methods (4)
    MapKeys,
    MapValues,
    MapContainsKey,
    MapIsEmpty,
}

impl BuiltinMethod {
    /// Create a BuiltinMethod from type and method names
    pub fn from_name(type_name: &str, method_name: &str) -> Option<Self> {
        match (type_name, method_name) {
            // String methods
            ("Str", "upper") => Some(BuiltinMethod::StrUpper),
            ("Str", "lower") => Some(BuiltinMethod::StrLower),
            ("Str", "substring") => Some(BuiltinMethod::StrSubstring),
            ("Str", "contains") => Some(BuiltinMethod::StrContains),
            ("Str", "trim") => Some(BuiltinMethod::StrTrim),
            ("Str", "split") => Some(BuiltinMethod::StrSplit),
            ("Str", "replace") => Some(BuiltinMethod::StrReplace),
            ("Str", "starts_with") => Some(BuiltinMethod::StrStartsWith),
            ("Str", "ends_with") => Some(BuiltinMethod::StrEndsWith),
            ("Str", "is_empty") => Some(BuiltinMethod::StrIsEmpty),

            // List methods
            ("List", "first") => Some(BuiltinMethod::ListFirst),
            ("List", "last") => Some(BuiltinMethod::ListLast),
            ("List", "contains") => Some(BuiltinMethod::ListContains),
            ("List", "slice") => Some(BuiltinMethod::ListSlice),
            ("List", "reverse") => Some(BuiltinMethod::ListReverse),
            ("List", "join") => Some(BuiltinMethod::ListJoin),
            ("List", "is_empty") => Some(BuiltinMethod::ListIsEmpty),

            // Map methods
            ("Map", "keys") => Some(BuiltinMethod::MapKeys),
            ("Map", "values") => Some(BuiltinMethod::MapValues),
            ("Map", "contains_key") => Some(BuiltinMethod::MapContainsKey),
            ("Map", "is_empty") => Some(BuiltinMethod::MapIsEmpty),

            _ => None,
        }
    }

    /// Execute this built-in method with the given receiver and arguments
    pub fn execute(&self, receiver: Expr, args: Vec<Expr>) -> Result<Expr, Box<dyn Error>> {
        match self {
            // === String Methods ===
            BuiltinMethod::StrUpper => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    let upper = s.trim_matches('\'').to_uppercase();
                    Ok(Expr::RuntimeData(LiteralData::Str(
                        format!("'{}'", upper).into(),
                    )))
                }
                _ => Err("upper() can only be called on strings".into()),
            },

            BuiltinMethod::StrLower => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    let lower = s.trim_matches('\'').to_lowercase();
                    Ok(Expr::RuntimeData(LiteralData::Str(
                        format!("'{}'", lower).into(),
                    )))
                }
                _ => Err("lower() can only be called on strings".into()),
            },

            BuiltinMethod::StrSubstring => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    if args.len() != 2 {
                        return Err("substring() requires exactly 2 arguments".into());
                    }

                    let str_content = s.trim_matches('\'');
                    let (start_idx, end_idx) = match (&args[0], &args[1]) {
                        (
                            Expr::Literal(LiteralData::Int(start))
                            | Expr::RuntimeData(LiteralData::Int(start)),
                            Expr::Literal(LiteralData::Int(end))
                            | Expr::RuntimeData(LiteralData::Int(end)),
                        ) => (*start, *end),
                        _ => return Err("substring() requires integer arguments".into()),
                    };

                    let start_usize = start_idx.max(0) as usize;
                    let end_usize = end_idx.max(0) as usize;

                    if start_usize <= end_usize && end_usize <= str_content.len() {
                        let substring = &str_content[start_usize..end_usize];
                        Ok(Expr::RuntimeData(LiteralData::Str(
                            format!("'{}'", substring).into(),
                        )))
                    } else {
                        Err(format!(
                            "substring indices out of bounds: start={}, end={}, length={}",
                            start_usize,
                            end_usize,
                            str_content.len()
                        )
                        .into())
                    }
                }
                _ => Err("substring() can only be called on strings".into()),
            },

            BuiltinMethod::StrContains => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    if args.len() != 1 {
                        return Err("contains() requires exactly 1 argument".into());
                    }

                    let str_content = s.trim_matches('\'');
                    match &args[0] {
                        Expr::Literal(LiteralData::Str(ref needle))
                        | Expr::RuntimeData(LiteralData::Str(ref needle)) => {
                            let needle_content = needle.trim_matches('\'');
                            let contains = str_content.contains(needle_content);
                            Ok(Expr::RuntimeData(LiteralData::Bool(contains)))
                        }
                        _ => Err("contains() requires a string argument".into()),
                    }
                }
                _ => Err("contains() can only be called on strings".into()),
            },

            BuiltinMethod::StrTrim => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    let trimmed = s.trim_matches('\'').trim();
                    Ok(Expr::RuntimeData(LiteralData::Str(
                        format!("'{}'", trimmed).into(),
                    )))
                }
                _ => Err("trim() can only be called on strings".into()),
            },

            BuiltinMethod::StrSplit => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    if args.len() != 1 {
                        return Err("split() requires exactly 1 argument".into());
                    }

                    let str_content = s.trim_matches('\'');
                    match &args[0] {
                        Expr::Literal(LiteralData::Str(ref delim))
                        | Expr::RuntimeData(LiteralData::Str(ref delim)) => {
                            let delim_content = delim.trim_matches('\'');
                            let parts: Vec<Expr> = str_content
                                .split(delim_content)
                                .map(|part| {
                                    Expr::RuntimeData(LiteralData::Str(
                                        format!("'{}'", part).into(),
                                    ))
                                })
                                .collect();
                            Ok(Expr::RuntimeList {
                                data_type: DataType::Str,
                                data: parts,
                            })
                        }
                        _ => Err("split() requires a string delimiter".into()),
                    }
                }
                _ => Err("split() can only be called on strings".into()),
            },

            BuiltinMethod::StrReplace => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    if args.len() != 2 {
                        return Err("replace() requires exactly 2 arguments".into());
                    }

                    let str_content = s.trim_matches('\'');
                    match (&args[0], &args[1]) {
                        (
                            Expr::Literal(LiteralData::Str(ref old))
                            | Expr::RuntimeData(LiteralData::Str(ref old)),
                            Expr::Literal(LiteralData::Str(ref new))
                            | Expr::RuntimeData(LiteralData::Str(ref new)),
                        ) => {
                            let old_content = old.trim_matches('\'');
                            let new_content = new.trim_matches('\'');
                            let replaced = str_content.replace(old_content, new_content);
                            Ok(Expr::RuntimeData(LiteralData::Str(
                                format!("'{}'", replaced).into(),
                            )))
                        }
                        _ => Err("replace() requires two string arguments".into()),
                    }
                }
                _ => Err("replace() can only be called on strings".into()),
            },

            BuiltinMethod::StrStartsWith => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    if args.len() != 1 {
                        return Err("starts_with() requires exactly 1 argument".into());
                    }

                    let str_content = s.trim_matches('\'');
                    match &args[0] {
                        Expr::Literal(LiteralData::Str(ref prefix))
                        | Expr::RuntimeData(LiteralData::Str(ref prefix)) => {
                            let prefix_content = prefix.trim_matches('\'');
                            let starts = str_content.starts_with(prefix_content);
                            Ok(Expr::RuntimeData(LiteralData::Bool(starts)))
                        }
                        _ => Err("starts_with() requires a string argument".into()),
                    }
                }
                _ => Err("starts_with() can only be called on strings".into()),
            },

            BuiltinMethod::StrEndsWith => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    if args.len() != 1 {
                        return Err("ends_with() requires exactly 1 argument".into());
                    }

                    let str_content = s.trim_matches('\'');
                    match &args[0] {
                        Expr::Literal(LiteralData::Str(ref suffix))
                        | Expr::RuntimeData(LiteralData::Str(ref suffix)) => {
                            let suffix_content = suffix.trim_matches('\'');
                            let ends = str_content.ends_with(suffix_content);
                            Ok(Expr::RuntimeData(LiteralData::Bool(ends)))
                        }
                        _ => Err("ends_with() requires a string argument".into()),
                    }
                }
                _ => Err("ends_with() can only be called on strings".into()),
            },

            BuiltinMethod::StrIsEmpty => match receiver {
                Expr::Literal(LiteralData::Str(ref s))
                | Expr::RuntimeData(LiteralData::Str(ref s)) => {
                    let str_content = s.trim_matches('\'');
                    Ok(Expr::RuntimeData(LiteralData::Bool(str_content.is_empty())))
                }
                _ => Err("is_empty() can only be called on strings".into()),
            },

            // === List Methods ===
            BuiltinMethod::ListFirst => match receiver {
                Expr::RuntimeList { ref data, .. } => {
                    if let Some(first_item) = data.first() {
                        Ok(first_item.clone())
                    } else {
                        Err("first() called on empty list".into())
                    }
                }
                _ => Err("first() can only be called on lists".into()),
            },

            BuiltinMethod::ListLast => match receiver {
                Expr::RuntimeList { ref data, .. } => {
                    if let Some(last_item) = data.last() {
                        Ok(last_item.clone())
                    } else {
                        Err("last() called on empty list".into())
                    }
                }
                _ => Err("last() can only be called on lists".into()),
            },

            BuiltinMethod::ListContains => match receiver {
                Expr::RuntimeList { ref data, .. } => {
                    if args.len() != 1 {
                        return Err("contains() requires exactly 1 argument".into());
                    }

                    let item_to_find = &args[0];
                    let found = data.iter().any(|elem| match (elem, item_to_find) {
                        (
                            Expr::Literal(a) | Expr::RuntimeData(a),
                            Expr::Literal(b) | Expr::RuntimeData(b),
                        ) => a == b,
                        _ => false,
                    });
                    Ok(Expr::RuntimeData(LiteralData::Bool(found)))
                }
                _ => Err("contains() can only be called on lists".into()),
            },

            BuiltinMethod::ListSlice => match receiver {
                Expr::RuntimeList {
                    ref data,
                    ref data_type,
                } => {
                    if args.len() != 2 {
                        return Err("slice() requires exactly 2 arguments".into());
                    }

                    let (start_idx, end_idx) = match (&args[0], &args[1]) {
                        (
                            Expr::Literal(LiteralData::Int(start))
                            | Expr::RuntimeData(LiteralData::Int(start)),
                            Expr::Literal(LiteralData::Int(end))
                            | Expr::RuntimeData(LiteralData::Int(end)),
                        ) => (*start, *end),
                        _ => return Err("slice() requires integer arguments".into()),
                    };

                    let start_usize = start_idx.max(0) as usize;
                    let end_usize = end_idx.max(0) as usize;

                    if start_usize <= end_usize && end_usize <= data.len() {
                        let sliced = data[start_usize..end_usize].to_vec();
                        Ok(Expr::RuntimeList {
                            data_type: data_type.clone(),
                            data: sliced,
                        })
                    } else {
                        Err(format!(
                            "slice indices out of bounds: start={}, end={}, length={}",
                            start_usize,
                            end_usize,
                            data.len()
                        )
                        .into())
                    }
                }
                _ => Err("slice() can only be called on lists".into()),
            },

            BuiltinMethod::ListReverse => match receiver {
                Expr::RuntimeList {
                    ref data,
                    ref data_type,
                } => {
                    let mut reversed = data.clone();
                    reversed.reverse();
                    Ok(Expr::RuntimeList {
                        data_type: data_type.clone(),
                        data: reversed,
                    })
                }
                _ => Err("reverse() can only be called on lists".into()),
            },

            BuiltinMethod::ListJoin => match receiver {
                Expr::RuntimeList { ref data, .. } => {
                    if args.len() != 1 {
                        return Err("join() requires exactly 1 argument".into());
                    }

                    match &args[0] {
                        Expr::Literal(LiteralData::Str(ref sep))
                        | Expr::RuntimeData(LiteralData::Str(ref sep)) => {
                            let sep_content = sep.trim_matches('\'');

                            let strings: Result<Vec<String>, Box<dyn Error>> = data
                                .iter()
                                .map(|elem| match elem {
                                    Expr::Literal(LiteralData::Str(s))
                                    | Expr::RuntimeData(LiteralData::Str(s)) => {
                                        Ok(s.trim_matches('\'').to_string())
                                    }
                                    _ => {
                                        Err("join() can only be called on lists of strings".into())
                                    }
                                })
                                .collect();

                            match strings {
                                Ok(str_vec) => {
                                    let joined = str_vec.join(sep_content);
                                    Ok(Expr::RuntimeData(LiteralData::Str(
                                        format!("'{}'", joined).into(),
                                    )))
                                }
                                Err(e) => Err(e),
                            }
                        }
                        _ => Err("join() requires a string separator".into()),
                    }
                }
                _ => Err("join() can only be called on lists".into()),
            },

            BuiltinMethod::ListIsEmpty => match receiver {
                Expr::RuntimeList { ref data, .. } => {
                    Ok(Expr::RuntimeData(LiteralData::Bool(data.is_empty())))
                }
                _ => Err("is_empty() can only be called on lists".into()),
            },

            // === Map Methods ===
            BuiltinMethod::MapKeys => {
                match receiver {
                    Expr::RuntimeMap {
                        ref data,
                        ref key_type,
                        ..
                    } => {
                        let mut keys_vec: Vec<Expr> = data
                            .keys()
                            .map(|key| match key {
                                KeyData::Int(i) => Expr::RuntimeData(LiteralData::Int(*i)),
                                KeyData::Str(s) => Expr::RuntimeData(LiteralData::Str(s.clone())),
                                KeyData::Bool(b) => Expr::RuntimeData(LiteralData::Bool(*b)),
                            })
                            .collect();

                        // Sort keys for consistent output
                        keys_vec.sort_by(|a, b| match (a, b) {
                            (
                                Expr::RuntimeData(LiteralData::Int(x)),
                                Expr::RuntimeData(LiteralData::Int(y)),
                            ) => x.cmp(y),
                            (
                                Expr::RuntimeData(LiteralData::Str(x)),
                                Expr::RuntimeData(LiteralData::Str(y)),
                            ) => x.cmp(y),
                            (
                                Expr::RuntimeData(LiteralData::Bool(x)),
                                Expr::RuntimeData(LiteralData::Bool(y)),
                            ) => x.cmp(y),
                            _ => std::cmp::Ordering::Equal,
                        });

                        Ok(Expr::RuntimeList {
                            data_type: key_type.clone(),
                            data: keys_vec,
                        })
                    }
                    _ => Err("keys() can only be called on maps".into()),
                }
            }

            BuiltinMethod::MapValues => {
                match receiver {
                    Expr::RuntimeMap {
                        ref data,
                        ref value_type,
                        ..
                    } => {
                        // Extract values - order matches sorted keys
                        let mut key_value_pairs: Vec<_> = data.iter().collect();
                        key_value_pairs.sort_by(|(a, _), (b, _)| match (a, b) {
                            (KeyData::Int(x), KeyData::Int(y)) => x.cmp(y),
                            (KeyData::Str(x), KeyData::Str(y)) => x.cmp(y),
                            (KeyData::Bool(x), KeyData::Bool(y)) => x.cmp(y),
                            _ => std::cmp::Ordering::Equal,
                        });

                        let values_vec: Vec<Expr> =
                            key_value_pairs.iter().map(|(_, v)| (*v).clone()).collect();
                        Ok(Expr::RuntimeList {
                            data_type: value_type.clone(),
                            data: values_vec,
                        })
                    }
                    _ => Err("values() can only be called on maps".into()),
                }
            }

            BuiltinMethod::MapContainsKey => match receiver {
                Expr::RuntimeMap { ref data, .. } => {
                    if args.len() != 1 {
                        return Err("contains_key() requires exactly 1 argument".into());
                    }

                    let key_data = match &args[0] {
                        Expr::Literal(LiteralData::Int(i))
                        | Expr::RuntimeData(LiteralData::Int(i)) => KeyData::Int(*i),
                        Expr::Literal(LiteralData::Str(s))
                        | Expr::RuntimeData(LiteralData::Str(s)) => KeyData::Str(s.clone()),
                        Expr::Literal(LiteralData::Bool(b))
                        | Expr::RuntimeData(LiteralData::Bool(b)) => KeyData::Bool(*b),
                        _ => return Err("contains_key() key must be Int, Str, or Bool".into()),
                    };

                    let contains = data.contains_key(&key_data);
                    Ok(Expr::RuntimeData(LiteralData::Bool(contains)))
                }
                _ => Err("contains_key() can only be called on maps".into()),
            },

            BuiltinMethod::MapIsEmpty => match receiver {
                Expr::RuntimeMap { ref data, .. } => {
                    Ok(Expr::RuntimeData(LiteralData::Bool(data.is_empty())))
                }
                _ => Err("is_empty() can only be called on maps".into()),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub params: Vec<Param>,
    pub return_type: DataType,
    pub body: Box<Expr>,
    pub receiver_type: Option<String>, // Some("TypeName") for methods, None for regular functions
    pub builtin: Option<BuiltinMethod>, // Some(...) for built-in methods, None for user-defined
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Program {
        body: Vec<Expr>,
        environment: usize,
    },
    Block {
        body: Vec<Expr>,
        environment: usize,
    },
    Output {
        data: Vec<Expr>,
    },

    // Parsed out from the source file; the structure will resemble the source code
    // and is easily scanned and type checked.
    Literal(LiteralData),
    MapLiteral {
        key_type: DataType,
        value_type: DataType,
        data: Vec<(KeyData, Expr)>,
    },
    ListLiteral {
        data_type: DataType,
        data: Vec<Expr>,
    },
    Range(LiteralData, LiteralData),

    // Special case for values accessed and changed during runtime in the interpreter; we
    // may wish to change the hashtable for Map or expand how data is physically represented
    // in memory during execution.
    RuntimeData(LiteralData),
    RuntimeList {
        data_type: DataType,
        data: Vec<Expr>,
    },
    RuntimeMap {
        key_type: DataType,
        value_type: DataType,
        data: HashMap<KeyData, Expr>,
    },
    StructLiteral {
        type_name: String,
        fields: Vec<(String, Expr)>, // field_name -> value expression
    },
    RuntimeStruct {
        type_name: String,
        fields: HashMap<String, Expr>, // field_name -> runtime value
    },
    FieldAccess {
        expr: Box<Expr>,
        field_name: String,
    },
    FieldAssign {
        expr: Box<Expr>,
        field_name: String,
        value: Box<Expr>,
        index: (usize, usize),
    },

    BinaryExpr {
        left: Box<Expr>,
        op: Operator,
        right: Box<Expr>,
    },
    UnaryExpr {
        op: Operator,
        expr: Box<Expr>,
    },
    Assign {
        name: String,
        value: Box<Expr>,
        index: (usize, usize),
    },
    Variable {
        name: String,
        index: (usize, usize),
    },
    Call {
        fn_name: String,
        index: (usize, usize),
        args: Vec<KeywordArg>,
    },
    MethodCall {
        receiver: Box<Expr>,      // The object (e.g., mystring)
        method_name: String,      // The method name (e.g., upper)
        fn_index: (usize, usize), // Index of the function in symbol table
        args: Vec<KeywordArg>,    // Arguments (not including self)
    },
    DefineFunction {
        fn_name: String,
        index: (usize, usize),
        value: Box<Expr>, // Probably an Expr::Lambda
    },
    Lambda {
        value: Function,
        environment: usize,
    },
    Let {
        var_name: String,
        index: (usize, usize),
        data_type: DataType,
        value: Box<Expr>,
        mutable: bool,
    },
    DefineType {
        type_name: String,
        definition: DataType,
        index: (usize, usize),
    },
    If {
        cond: Box<Expr>,
        then: Box<Expr>,
        final_else: Box<Expr>,
    },
    Match {
        cond: Box<Expr>,
        against: Vec<(Expr, Expr)>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    Return(Box<Expr>),
    Unit,
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
    },
    Len {
        expr: Box<Expr>,
    },
}
impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Literal(d) => {
                write!(f, "{}", d)
            }
            Expr::RuntimeData(d) => {
                write!(f, "{}", d)
            }
            Expr::ListLiteral { data_type, data } => {
                let printed_items = data
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "[{}]", printed_items)
            }
            Expr::RuntimeList { data_type, data } => {
                let printed_items = data
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "[{}]", printed_items)
            }
            Expr::MapLiteral {
                key_type,
                value_type,
                data,
            } => {
                let printed_pairs = data
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<String>>()
                    .join(",");
                write!(f, "{{{}}}", printed_pairs)
            }
            Expr::RuntimeMap {
                key_type,
                value_type,
                data,
            } => {
                let mut pairs: Vec<String> =
                    data.iter().map(|(k, v)| format!("{}:{}", k, v)).collect();
                pairs.sort(); // For consistent output
                write!(f, "{{{}}}", pairs.join(","))
            }
            Expr::StructLiteral { type_name, fields } => {
                write!(f, "{} {{ ", type_name)?;
                for (i, (name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, value)?;
                }
                write!(f, " }}")
            }
            Expr::RuntimeStruct { type_name, fields } => {
                write!(f, "{} {{ ", type_name)?;
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(name, _)| *name);
                for (i, (name, value)) in sorted_fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, value)?;
                }
                write!(f, " }}")
            }
            Expr::FieldAccess { expr, field_name } => {
                write!(f, "{}.{}", expr, field_name)
            }
            Expr::FieldAssign {
                expr,
                field_name,
                value,
                ..
            } => {
                write!(f, "{}.{} := {}", expr, field_name, value)
            }
            Expr::Range(start, end) => {
                write!(f, "{}..{}", start, end)
            }
            Expr::Index { expr, index } => {
                write!(f, "{}[{}]", expr, index)
            }
            Expr::Len { expr } => {
                write!(f, "len({})", expr)
            }
            Expr::MethodCall {
                receiver,
                method_name,
                args,
                ..
            } => {
                write!(f, "{}.{}(", receiver, method_name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", arg.name, arg.value)?;
                }
                write!(f, ")")
            }
            _ => write!(f, "{:?}", &self),
        }
    }
}

impl Expr {
    // Makes copies of the initial data emitted by the parser for use at runtime.
    // Only happens once when starting the interpreter, so maximum performance isn't too
    // important.
    pub fn copy_to_runtime_data(&self) -> Expr {
        match self {
            Expr::Literal(value) => Expr::RuntimeData(value.clone()),
            Expr::ListLiteral {
                ref data_type,
                ref data,
            } => {
                let upgraded_items = data
                    .iter()
                    .map(|i| i.copy_to_runtime_data())
                    .collect::<Vec<Expr>>();
                Expr::RuntimeList {
                    data_type: data_type.clone(),
                    data: upgraded_items.clone(),
                }
            }
            Expr::MapLiteral {
                key_type,
                value_type,
                data,
            } => {
                let upgraded_values = data
                    .iter()
                    .map(|item| (item.0.clone(), item.1.copy_to_runtime_data()))
                    .collect::<HashMap<KeyData, Expr>>();
                Expr::RuntimeMap {
                    key_type: key_type.clone(),
                    value_type: value_type.clone(),
                    data: upgraded_values,
                }
            }
            _ => Expr::Unit,
            //_ => panic!("Error converting compiled data into runtime representation:\n -->  '{:?}' \nProbably this is an accidentally unsupported data structure -- a compiler bug.", &self),
        }
    }

    pub fn has_value(&self, value: &LiteralData) -> bool {
        if let (Expr::Literal(l), r) = (self, value) {
            l == r
        } else {
            false
        }
    }

    pub fn is_data(&self) -> bool {
        matches!(
            self,
            Expr::Literal(_) | Expr::MapLiteral { .. } | Expr::ListLiteral { .. }
        )
    }

    pub fn equal(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr {
            left: Box::new(l),
            right: Box::new(r),
            op: Operator::Eq,
        }
    }
    pub fn add(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr {
            left: Box::new(l),
            right: Box::new(r),
            op: Operator::Add,
        }
    }

    pub fn sub(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr {
            left: Box::new(l),
            right: Box::new(r),
            op: Operator::Sub,
        }
    }
    pub fn mul(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr {
            left: Box::new(l),
            right: Box::new(r),
            op: Operator::Mul,
        }
    }
    pub fn div(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr {
            left: Box::new(l),
            right: Box::new(r),
            op: Operator::Div,
        }
    }
}
