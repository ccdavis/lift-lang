mod interpreter;
mod semantic_analysis;
mod symboltable;
mod syntax;

use interpreter::InterpreterResult;
use lalrpop_util::lalrpop_mod;
use symboltable::SymbolTable;
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
#[test]
fn test_interpret_math() {
    let src = "1 + 2 * 3";
    let parser = grammar::ExprParser::new();
    let parse_result = parser.parse(src);
    assert!(parse_result.is_ok());

    let mut symbols = SymbolTable::new();
    let s = parse_result.unwrap().interpret(&mut symbols, 0);
    match s {
        Err(ref e) => println!("Runtime error: {:?}", e),
        Ok(ref r) => println!("Success: {:?}", &r),
    }
    assert!(s.is_ok());
}

#[test]
fn test_boolean_expressions() {
    let parser = grammar::ExprParser::new();
    let src = "3 = 3";
    let parse_result = parser.parse(src);
    assert!(parse_result.is_ok());

    let mut symbols = SymbolTable::new();
    let s = parse_result.unwrap().interpret(&mut symbols, 0);
    match s {
        Err(ref e) => println!("Runtime error: {:?}", e),
        Ok(ref r) => println!("Success: {:?}", &r),
    }
    assert!(s.is_ok());
    assert_eq!(LiteralData::Bool(true), extract_value(s));

    let src = "3 = 4";
    let parse_result = parser.parse(src);
    assert!(parse_result.is_ok());

    let mut symbols = SymbolTable::new();
    let s = parse_result.unwrap().interpret(&mut symbols, 0);
    match s {
        Err(ref e) => println!("Runtime error: {:?}", e),
        Ok(ref r) => println!("Success: {:?}", &r),
    }
    assert!(s.is_ok());
    assert_eq!(LiteralData::Bool(false), extract_value(s));

    let src = "3+9 =  1 + 11";
    let parse_result = parser.parse(src);
    assert!(parse_result.is_ok());
    println!("Parse result for complex equality: {:?}", &parse_result);

    let mut symbols = SymbolTable::new();
    let s = parse_result.unwrap().interpret(&mut symbols, 0);
    match s {
        Err(ref e) => println!("Runtime error: {:?}", e),
        Ok(ref r) => println!("Success: {:?}", &r),
    }
    assert!(s.is_ok());
    assert_eq!(LiteralData::Bool(true), extract_value(s));
}

#[test]
fn test_interpret_conditionals() {
    let parser = grammar::ExprParser::new();
    let src = "if true { 25*5} else { 1-3}";
    let parse_result = parser.parse(src);
    match parse_result {
        Err(ref e) => eprintln!("Parse conditional failed: {:?}", &e),
        Ok(ref r) => println!("Success parsing conditional."),
    }
    assert!(parse_result.is_ok());

    let mut symbols = SymbolTable::new();
    let s = parse_result.unwrap().interpret(&mut symbols, 0);
    match s {
        Err(ref e) => println!("Runtime error: {:?}", e),
        Ok(ref r) => println!("Success: {:?}", &r),
    }
    assert!(s.is_ok());
    assert!(check_value(&s, LiteralData::Int(125)));

    let src = "if false { 25*5} else { 1-3}";
    let parse_result = parser.parse(src);
    match parse_result {
        Err(ref e) => eprintln!("Parse conditional failed: {:?}", &e),
        Ok(ref r) => println!("Success parsing conditional."),
    }
    assert!(parse_result.is_ok());
    let mut symbols = SymbolTable::new();
    let s = parse_result.unwrap().interpret(&mut symbols, 0);
    assert_eq!(LiteralData::Int(-2), extract_value(s));
}

// A test helper
fn check_value(s: &InterpreterResult, value: LiteralData) -> bool {
    if let Ok(Some(ref e)) = s {
        return e.has_value(&value);
    }
    false
}

fn extract_value(r: InterpreterResult) -> LiteralData {
    if let Ok(Some(Expr::Literal(l))) = r {
        return l;
    }
    panic!("Must pass an interpreter result that holds a literal data value.");
}

fn main() {
    println!("Hello world!")
}
