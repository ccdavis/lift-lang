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
    Str(String),
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
    Str(String),
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
        LiteralData::Str(data.to_string())
    }
}

impl From<String> for LiteralData {
    fn from(data: String) -> LiteralData {
        LiteralData::Str(data.clone())
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

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    params: Vec<Param>,
    return_type: DataType,
    body: Box<Expr>,
}

// The AssignableData is what we can place in the  Environment for
// blocks and functions for variables and functions after they are
// evaluated.
#[derive(Clone, Debug, PartialEq)]
pub enum AssignableData {
    Lambda(Function, usize),
    Literal(LiteralData),
    ListLiteral(Vec<AssignableData>),
    MapLiteral(Vec<(LiteralData, AssignableData)>),
    Tbd(Box<Expr>), // To be determined later
                    // TODO if we allow something like Macro(Expr) we can
                    // store arbitrary code and interpret it later
}

impl From<Expr> for AssignableData {
    fn from(data: Expr) -> AssignableData {
        match data {
            Expr::Literal(value) => AssignableData::Literal(value),
            Expr::Lambda { value, environment } => AssignableData::Lambda(value, environment),
            Expr::List(l) => {
                let assignable_items = l
                    .into_iter()
                    .map(|item| AssignableData::from(item))
                    .collect::<Vec<AssignableData>>();
                AssignableData::ListLiteral(assignable_items)
            }
            Expr::Map(m) => {
                let assignable_values = m
                    .into_iter()
                    .map(|item| (LiteralData::from(item.0), AssignableData::from(item.1)))
                    .collect::<Vec<(LiteralData, AssignableData)>>();
                AssignableData::MapLiteral(assignable_values)
            }
            _ => AssignableData::Tbd(Box::new(data)),
        }
    }
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
    Literal(LiteralData),
    List(Vec<Expr>),
    Map(HashMap<KeyData, Expr>),
    BinaryExpr {
        left: Box<Expr>,
        op: Operator,
        right: Box<Expr>,
    },
    UnaryExpr {
        op: Operator,
        expr: Box<Expr>,
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
        value: Function,
        environment: usize,
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

impl From<AssignableData> for Expr {
    fn from(data: AssignableData) -> Expr {
        match data {
            AssignableData::Literal(value) => Expr::Literal(value),
            AssignableData::Lambda(value, environment) => Expr::Lambda {value,environment},
            AssignableData::ListLiteral(l) => {
                let assignable_items = l.into_iter()
                    .map(|item| Expr::from(item))
                    .collect::<Vec<Expr>>();
                Expr::List(assignable_items)
            }
            AssignableData::MapLiteral(m) => {
                let assignable_values = m.into_iter()
                    .map(|item| (KeyData::from(item.0),Expr::from(item.1)))
                    .collect::<HashMap<KeyData,Expr>>();
                Expr::Map(assignable_values)
            }                    
            _ => panic!("Error converting compiled data into runtime representation:\n -->  '{:?}' \nProbably this is an accidentally unsupported data structure -- a compiler bug.", &data),
        }
    }
}

impl Expr {
    pub fn has_value(&self, value: &LiteralData) -> bool {
        if let (Expr::Literal(l), r) = (self, value) {
            return l == r;
        } else {
            false
        }
    }

    pub fn is_data(&self) -> bool {
        match self {
            Expr::Literal(_) | Expr::Map(_) | Expr::List(_) => true,
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
