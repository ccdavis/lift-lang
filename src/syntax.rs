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
#[derive(Clone,Debug,PartialEq)]
pub enum Operator {
    Div, Mul, Add, Sub, Gt, Lt, Gte, Lte, Eq, Neq, And, Or, Not
}

#[derive(Clone,Debug,PartialEq)]
pub struct Param {
    pub name: String,
    pub data_type: DataType,
}

#[derive(Clone,Debug,PartialEq)]
pub enum DataType {
    Str, Int, Flt, Bool,  Any, None,
    Map { key_type: Box<DataType>, value_type: Box<DataType>}, 
    List { element_type: Box<DataType> },
    Set(Box<DataType>),
    Struct(Vec<Param>),    
}

#[derive(Clone,Debug,PartialEq)]
pub struct KeywordArg {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone,Debug,PartialEq)]
pub enum LiteralData {
    Int(i64),
    Flt(f64),
    Str(String),
    Bool(bool),    
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



#[derive(Clone,Debug,PartialEq)]
pub enum Expr {
    Literal(LiteralData),    
    ListLiteral(Vec<Expr>),    
    MapLiteral(Vec<(Expr,Expr)>),    
    Block(Vec<Expr>),
    BinaryExpr { left: Box<Expr>, op: Operator, right: Box<Expr> },
    UnaryExpr {op: Operator, expr: Box<Expr> },
    Variable(String),
    Call { fn_name: String, args: Vec<KeywordArg>},
     DefineFunction { fn_name: String, params: Vec<Param>, return_type: DataType, body: Box<Expr>},
    Lambda { args: Vec<KeywordArg>, return_type: DataType, body: Box<Expr> },
    Let { var_name: String, data_type: DataType, value: Box<Expr>},
    DefineType{ type_name: String, definition: DataType },
    If { cond: Box<Expr>, then: Box<Expr>, elsif: Vec<Box<Expr>>, final_else: Box<Expr>},
    Match { cond: Box<Expr>, against: Vec<(Expr,Box<Expr>)>},
    While { cond: Box<Expr>, body: Box<Expr>},
    Return(Box<Expr>),
}

impl Expr {
    pub fn add(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr{left: Box::new(l), right: Box::new(r), op: Operator::Add}
    }
    pub fn sub(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr{left: Box::new(l), right: Box::new(r), op: Operator::Sub}
    }
    pub fn mul(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr{left: Box::new(l), right: Box::new(r), op: Operator::Mul}
    }
    pub fn div(l: Expr, r: Expr) -> Expr {
        Expr::BinaryExpr{left: Box::new(l), right: Box::new(r), op: Operator::Div}
    }





}