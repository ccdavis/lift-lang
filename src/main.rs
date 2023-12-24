mod interpreter;
mod symboltable;
mod syntax;

use lalrpop_util::lalrpop_mod;
use syntax::*;
lalrpop_mod!(pub grammar); // synthesized by LALRPOP
use grammar::*;

#[test]
fn test_parse_numbers() {
    let src = "3";
    let should_be = LiteralData::Int(3);
    let parser = grammar::LiteralDataParser::new();
    let got = parser.parse(src).unwrap();
    assert_eq!(got, should_be);

    let src = "3.5";
    let should_be = LiteralData::Flt(3.5);
    let got = parser.parse(src).unwrap();
    assert_eq!(got, should_be);

    let src = "09.5";
    let should_be = LiteralData::Flt(9.5);
    let got = parser.parse(src).unwrap();
    assert_eq!(got, should_be);
}

#[test]
fn test_parse_strings() {
    let parser = grammar::LiteralDataParser::new();
    let src = "'abc'";
    let should_be = LiteralData::Str("'abc'".to_string());
    let got = match parser.parse(src) {
        Ok(s) => s,
        Err(e) => {
            println!("Got {:?} for string", e);
            panic!("Parse error");
        }
    };
    assert_eq!(got, should_be);
}

#[test]
fn test_parse_bool() {
    let parser = grammar::LiteralDataParser::new();
    let src = "true";
    let should_be = LiteralData::Bool(true);
    let got = parser.parse(src).unwrap();
    assert_eq!(got, should_be);
    let src = "false";
    let should_be = LiteralData::Bool(false);
    let got = parser.parse(src).unwrap();
    assert_eq!(got, should_be);
}

pub fn make_literal_int(v: i64) -> Box<Expr> {
    let l = LiteralData::from(v);
    Box::new(Expr::Literal(l))
}

#[test]
fn test_binary_expression_parsing() {
    let parser = grammar::ExprParser::new();
    let src = "1 + 2";
    let parse_result = parser.parse(src);
    let one = make_literal_int(1);
    let two = make_literal_int(2);
    let should_be = Expr::BinaryExpr {
        left: one.clone(),
        op: Operator::Add,
        right: two.clone(),
    };

    match parse_result {
        Ok(r) => {
            assert_eq!(r, should_be);
        }
        Err(e) => {
            eprintln!("Error parsing '{}', got {:?}", src, e);
        }
    }

    let src = " 1*2 -2";
    let should_be = Expr::BinaryExpr {
        left: Box::new(Expr::BinaryExpr {
            left: one.clone(),
            op: Operator::Mul,
            right: two.clone(),
        }),
        op: Operator::Sub,
        right: two,
    };

    let parse_result = parser.parse(src);
    assert!(parse_result.is_ok());
    let got = parse_result.unwrap();
    println!("Got {:?}", got);
    assert_eq!(got, should_be);
}

#[test]
fn test_parse_if_expr() {
    let src = "if true  { 8} else{ 5}";
    let parser = grammar::ExprParser::new();
    let parse_result = parser.parse(src);
    if let Err(ref e) = parse_result {
        eprintln!("Error parsing '{}', got {:?}", src, e);
    };
    assert!(parse_result.is_ok());
}

fn main() {
    println!("Hello world!")
}
