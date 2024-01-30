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
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub data_type: DataType,
    pub default: Option<Expr>,
    pub index: (usize, usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum DataType {
    Str,
    Int,
    Flt,
    Bool,
    Any,
    Map {
        key_type: Box<DataType>,
        value_type: Box<DataType>,
    },
    List {
        element_type: Box<DataType>,
    },
    Set(Box<DataType>),
    Struct(Vec<Param>),
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
            LiteralData::Int(i) => write!(f, "{}", i),
            LiteralData::Flt(fl) => write!(f, "{}", fl),
            LiteralData::Bool(b) => write!(f, "{}", b),
            LiteralData::Str(s) => write!(f, "{}", &s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub params: Vec<Param>,
    pub return_type: DataType,
    pub body: Box<Expr>,
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
}
impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Literal(d) => {
                write!(f, "{}", d)
            }
            Expr::ListLiteral { data_type, data } => {
                let printed_items = data
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                write!(f, "[{}", printed_items)
            }
            Expr::MapLiteral {
                key_type,
                value_type,
                data,
            } => {
                write!(f, "{:?}", &data)
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
                    .into_iter()
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
                    .into_iter()
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
            return l == r;
        } else {
            false
        }
    }

    pub fn is_data(&self) -> bool {
        match self {
            Expr::Literal(_) | Expr::MapLiteral { .. } | Expr::ListLiteral { .. } => true,
            _ => false,
        }
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
