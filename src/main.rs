mod syntax;

use lalrpop_util::lalrpop_mod;
use syntax::*;
lalrpop_mod!(pub grammar); // synthesized by LALRPOP
use grammar::*;

#[test]
fn test_parse_numbers() {
    let src = "3";
    let should_be =  LiteralData::Int(3);
    let parser  = grammar::LiteralDataParser::new();
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

fn main() {
    println!("Hello world!")
}
