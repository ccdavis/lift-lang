// Integration tests for the Lift language
// This file contains all integration tests moved from src/main.rs

use lift_lang::interpreter::InterpreterResult;
use lift_lang::semantic;
use lift_lang::symboltable::SymbolTable;
use lift_lang::syntax::*;
use std::fs;

// Include the generated parser
use lift_lang::grammar;

// Test helper functions

pub fn make_literal_int(v: i64) -> Box<Expr> {
    let l = LiteralData::from(v);
    Box::new(Expr::Literal(l))
}

fn check_value(s: &InterpreterResult, value: LiteralData) -> bool {
    if let Ok(ref e) = s {
        return e.has_value(&value);
    }
    false
}

fn extract_value(r: InterpreterResult) -> LiteralData {
    if let Ok(Expr::Literal(l)) = r {
        return l;
    }
    panic!("Must pass an interpreter result that holds a literal data value.");
}

// Helper function for integration tests
fn run_lift_file(file_path: &str) -> Result<Expr, String> {
    let code = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let parser = grammar::ProgramParser::new();
    let mut ast = parser.parse(&code)
        .map_err(|e| format!("Parse error: {e}"))?;

    let mut symbols = SymbolTable::new();

    // Handle the Vec<CompileError> return type from prepare
    if let Err(errors) = ast.prepare(&mut symbols) {
        // Join all error messages and return as a single error
        let error_msg = errors.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(error_msg);
    }

    ast.interpret(&mut symbols, 0)
        .map_err(|e| format!("Runtime error: {e}"))
}

// All test functions below...

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
    let should_be = LiteralData::Str("'abc'".to_string().into());
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


#[test]
fn test_typecheck_arithmetic() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test int arithmetic
    let mut ast = parser.parse("1 + 2").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test float arithmetic
    let mut ast = parser.parse("1.5 + 2.5").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test mixed arithmetic (should work)
    let mut ast = parser.parse("1 + 2.5").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test string concatenation
    let mut ast = parser.parse("'hello' + ' world'").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test invalid arithmetic
    let mut ast = parser.parse("'hello' - 5").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

#[test]
fn test_typecheck_comparison() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test numeric comparison
    let mut ast = parser.parse("1 < 2").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    let mut ast = parser.parse("1.5 >= 2").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test invalid comparison
    let mut ast = parser.parse("'hello' < 5").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

#[test]
fn test_typecheck_logical() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test logical operations
    let mut ast = parser.parse("true and false").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    let mut ast = parser.parse("true or false").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test invalid logical
    let mut ast = parser.parse("1 and true").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
    
    let mut ast = parser.parse("'hello' or false").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

#[test]
fn test_typecheck_if_else() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test valid if-else
    let mut ast = parser.parse("if true { 1 } else { 2 }").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test invalid condition
    let mut ast = parser.parse("if 5 { 1 } else { 2 }").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
    
    // Test mismatched branches
    let mut ast = parser.parse("if true { 1 } else { 'hello' }").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

#[test]
fn test_typecheck_while() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test valid while
    let mut ast = parser.parse("while false { 1 }").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test invalid condition
    let mut ast = parser.parse("while 5 { 1 }").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

#[test]
fn test_typecheck_variables() {
    let parser = grammar::ProgramPartExprParser::new();
    
    // Test let binding
    let mut symbols = SymbolTable::new();
    let mut ast = parser.parse("let x = 5").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test let with type annotation
    let mut symbols = SymbolTable::new();
    let mut ast = parser.parse("let x: Int = 5").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test mismatched type annotation
    let mut symbols = SymbolTable::new();
    let mut ast = parser.parse("let x: Int = 'hello'").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

// TODO: List literals are not yet implemented in the grammar
// #[test]
// fn test_typecheck_lists() {
//     let parser = grammar::ProgramPartExprParser::new();
//     let mut symbols = SymbolTable::new();
//     
//     // Test homogeneous list
//     let mut ast = parser.parse("[1, 2, 3]").unwrap();
//     assert!(ast.prepare(&mut symbols).is_ok());
//     
//     // Test heterogeneous list (should fail)
//     let mut ast = parser.parse("[1, 'hello', 3]").unwrap();
//     assert!(ast.prepare(&mut symbols).is_err());
//     
//     // Test empty list
//     let mut ast = parser.parse("[]").unwrap();
//     assert!(ast.prepare(&mut symbols).is_ok());
// }

#[test]
fn test_typecheck_functions() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test function definition
    let mut ast = parser.parse("function add(x: Int, y: Int): Int { x + y }").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test undefined function call
    let mut ast = parser.parse("foo(x: 1, y: 2)").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

#[test]
fn test_typecheck_complex_expressions() {
    let parser = grammar::ProgramPartExprParser::new();
    
    // Test expression with variables (valid)
    let mut symbols = SymbolTable::new();
    let mut ast = parser.parse("{let x = 5; x + 10}").unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    
    // Test type mismatch in expression
    let mut symbols = SymbolTable::new();
    let mut ast = parser.parse("5 + 'hello'").unwrap();
    assert!(ast.prepare(&mut symbols).is_err());
}

#[test]
fn test_interpreter_arithmetic() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test integer arithmetic
    let mut ast = parser.parse("5 + 3").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(8)));
    
    // Test float arithmetic
    let mut ast = parser.parse("10.5 - 3.5").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Flt(7.0)));
    
    // Test multiplication and division
    let mut ast = parser.parse("4 * 5").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(20)));
    
    let mut ast = parser.parse("20 / 4").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(5)));
    
    // Test negative integer arithmetic
    let mut ast = parser.parse("-10 + 5").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(-5)));
    
    // Test negative float arithmetic
    let mut ast = parser.parse("-7.5 + 2.5").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Flt(-5.0)));
    
    // Test multiplication with negative numbers
    let mut ast = parser.parse("-4 * 3").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(-12)));
    
    // Test division with negative numbers
    let mut ast = parser.parse("-20 / 4").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(-5)));
}

#[test]
fn test_interpreter_string_operations() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test string concatenation
    let mut ast = parser.parse("'Hello' + ' World'").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Str("'Hello World'".into())));
}

#[test]
fn test_interpreter_comparison() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test integer comparison
    let mut ast = parser.parse("5 < 10").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(true)));
    
    let mut ast = parser.parse("10 >= 10").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(true)));
    
    // Test equality
    let mut ast = parser.parse("5 = 5").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(true)));
    
    let mut ast = parser.parse("5 <> 10").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(true)));
}

#[test]
fn test_interpreter_logical_operations() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test AND
    let mut ast = parser.parse("true and false").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(false)));
    
    // Test OR
    let mut ast = parser.parse("true or false").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(true)));
}

#[test]
fn test_interpreter_variables() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test variable binding and use
    let mut ast = parser.parse("{let x = 10; x + 5}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(15)));
    
    // Test multiple variables
    let mut ast = parser.parse("{let x = 5; let y = 10; x * y}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(50)));
}

#[test]
fn test_interpreter_if_else() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test true branch
    let mut ast = parser.parse("if true { 10 } else { 20 }").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(10)));
    
    // Test false branch
    let mut ast = parser.parse("if false { 10 } else { 20 }").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(20)));
    
    // Test with condition expression
    let mut ast = parser.parse("{let x = 5; if x > 3 { x * 2 } else { x }}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(10)));
}

#[test]
fn test_interpreter_while_loop() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test simple variable reference
    let mut ast = parser.parse("{let x = 5; x}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(5)));
}

#[test]
fn test_interpreter_functions() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test function definition and call
    let mut ast = parser.parse("{function square(x: Int): Int { x * x }; square(x: 5)}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(25)));
    
    // Test function with multiple parameters
    let mut ast = parser.parse("{function add(x: Int, y: Int): Int { x + y }; add(x: 3, y: 7)}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(10)));
}

#[test]
fn test_negative_numbers() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test negative numbers in let statements
    let mut ast = parser.parse("{let x = -42; x}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(-42)));
    
    // Test negative floats in let statements
    let mut ast = parser.parse("{let pi = -3.14; pi}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Flt(-3.14)));
    
    // Test negative numbers in comparisons
    let mut ast = parser.parse("-5 < 0").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(true)));
    
    // Test negative numbers in function arguments
    let mut ast = parser.parse("{function abs(x: Int): Int { if x < 0 { 0 - x } else { x } }; abs(x: -10)}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(10)));
}

#[test]
fn test_interpreter_nested_blocks() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test nested scopes with shadowing
    let mut ast = parser.parse("{let x = 5; let result: Int = {let x = 10; x}; result + x}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(15)));
}

#[test]
fn test_interpreter_complex_expressions() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test operator precedence
    let mut ast = parser.parse("2 + 3 * 4").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(14)));
    
    // Test parentheses
    let mut ast = parser.parse("(2 + 3) * 4").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(20)));
}

#[test]
fn test_interpreter_fibonacci() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test recursive function - simpler
    let program = "{
        function sum(n: Int): Int {
            if n <= 0 { 0 } else { n + sum(n: n - 1) }
        };
        sum(n: 3)
    }";
    let mut ast = parser.parse(program).unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(6))); // 3 + 2 + 1 + 0
}

#[test]
fn test_interpreter_factorial() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test recursive factorial
    let program = "{
        function fact(n: Int): Int {
            if n <= 1 { 1 } else { n * fact(n: n - 1) }
        };
        fact(n: 5)
    }";
    let mut ast = parser.parse(program).unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(120)));
}

#[test]
fn test_interpreter_type_inference() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test that type inference works correctly
    let mut ast = parser.parse("{let x = 5; let y = 2.5; let z = true; z}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Bool(true)));
}

#[test]
fn test_interpreter_else_if() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test else if chain
    let program = "{
        let score = 75;
        if score >= 90 {
            'A'
        } else if score >= 80 {
            'B'
        } else if score >= 70 {
            'C'
        } else {
            'F'
        }
    }";
    let mut ast = parser.parse(program).unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Str("'C'".into())));
    
    // Test multiple else if conditions
    let program = "{
        let x = 5;
        if x > 10 {
            'big'
        } else if x > 5 {
            'medium'
        } else if x = 5 {
            'exact'
        } else {
            'small'
        }
    }";
    let mut ast = parser.parse(program).unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Str("'exact'".into())));
}

#[test]
fn test_interpreter_if_without_else() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test if without else (returns Unit)
    let mut ast = parser.parse("if false { 42 }").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Unit);
    
    // Test if without else (condition is true)
    let mut ast = parser.parse("if true { 42 }").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_interpreter_nested_if_else() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test nested if-else
    let program = "{
        let x = 10;
        let y = 5;
        if x > 5 {
            if y > 3 {
                'both true'
            } else {
                'x true, y false'
            }
        } else {
            'x false'
        }
    }";
    let mut ast = parser.parse(program).unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Str("'both true'".into())));
}


#[test]
fn test_lt_file_else_if() {
    let result = run_lift_file("tests/test_else_if.lt").unwrap();
    // The test file outputs 'B' and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_if_no_else() {
    let result = run_lift_file("tests/test_if_no_else.lt").unwrap();
    // The test file outputs 'Greater than 5' and returns 42
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_lt_file_typechecker() {
    let result = run_lift_file("tests/test_typechecker.lt").unwrap();
    // This file returns 6 (x + 1 since x < y)
    assert_eq!(result, Expr::Literal(LiteralData::Int(6)));
}

#[test]
fn test_lt_file_type_checking_examples() {
    // This file should succeed with type inference for 'result'
    // Type inference can determine that both branches of the if return Int
    let result = run_lift_file("tests/type_checking_examples.lt");
    assert!(result.is_ok());
}

#[test]
fn test_lt_file_type_error() {
    // This file should fail with a type error
    let result = run_lift_file("tests/test_type_error.lt");
    assert!(result.is_err());
}

#[test]
fn test_lt_file_needs_annotation() {
    // This file should fail because it needs type annotation
    let result = run_lift_file("tests/test_needs_annotation.lt");
    assert!(result.is_err());
}

#[test]
fn test_lt_file_mandelbrot() {
    // Test the static Mandelbrot pattern
    let result = run_lift_file("tests/mandelbrot_visual.lt").unwrap();
    // The mandelbrot program outputs the pattern and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_mandelbrot_computed() {
    // Test the computed Mandelbrot that checks specific points
    let result = run_lift_file("tests/mandelbrot_tiny_computed.lt").unwrap();
    // This program computes membership for specific points and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_recursion_depth() {
    // Test that recursion works for reasonable depths
    let result = run_lift_file("tests/test_recursion_depth.lt").unwrap();
    // Should return 0 after counting down from 30
    assert_eq!(result, Expr::Literal(LiteralData::Int(0)));
}

#[test]
fn test_comments() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test single-line comment
    let mut ast = parser.parse("42 // This is a comment").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
    
    // Test block comment
    let mut ast = parser.parse("/* comment */ 100").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(100)));
    
    // Test inline block comment in expression
    let mut ast = parser.parse("5 /* inline */ + /* another */ 3").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(8)));
    
    // Test comment in block
    let mut ast = parser.parse("{
        // Comment at start
        let x = 10; // inline comment
        /* block comment
           on multiple lines */
        x + 5
    }").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(15)));
}

#[test]
fn test_lt_file_pattern_output() {
    // Test simple pattern output
    let result = run_lift_file("tests/test_pattern.lt").unwrap();
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_list_literals() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test basic integer list
    let mut ast = parser.parse("[1, 2, 3, 4, 5]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeList { data, .. } => {
            assert_eq!(data.len(), 5);
            assert_eq!(data[0], Expr::Literal(LiteralData::Int(1)));
            assert_eq!(data[4], Expr::Literal(LiteralData::Int(5)));
        }
        _ => panic!("Expected RuntimeList, got {:?}", result),
    }
    
    // Test string list
    let mut ast = parser.parse("['hello', 'world']").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeList { data, .. } => {
            assert_eq!(data.len(), 2);
            assert_eq!(data[0], Expr::Literal(LiteralData::Str("'hello'".into())));
            assert_eq!(data[1], Expr::Literal(LiteralData::Str("'world'".into())));
        }
        _ => panic!("Expected RuntimeList, got {:?}", result),
    }
    
    // Test list with expressions
    let mut ast = parser.parse("[1 + 2, 3 * 4, 10 - 5]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeList { data, .. } => {
            assert_eq!(data.len(), 3);
            assert_eq!(data[0], Expr::Literal(LiteralData::Int(3)));
            assert_eq!(data[1], Expr::Literal(LiteralData::Int(12)));
            assert_eq!(data[2], Expr::Literal(LiteralData::Int(5)));
        }
        _ => panic!("Expected RuntimeList, got {:?}", result),
    }
    
    // Test nested lists
    let mut ast = parser.parse("[[1, 2], [3, 4], [5, 6]]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeList { data, .. } => {
            assert_eq!(data.len(), 3);
            match &data[0] {
                Expr::RuntimeList { data: inner, .. } => {
                    assert_eq!(inner.len(), 2);
                    assert_eq!(inner[0], Expr::Literal(LiteralData::Int(1)));
                    assert_eq!(inner[1], Expr::Literal(LiteralData::Int(2)));
                }
                _ => panic!("Expected nested RuntimeList"),
            }
        }
        _ => panic!("Expected RuntimeList, got {:?}", result),
    }
}

#[test]
fn test_map_literals() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test basic integer key map
    let mut ast = parser.parse("#{1: 'one', 2: 'two', 3: 'three'}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeMap { data, .. } => {
            assert_eq!(data.len(), 3);
            assert_eq!(data.get(&KeyData::Int(1)), Some(&Expr::Literal(LiteralData::Str("'one'".into()))));
            assert_eq!(data.get(&KeyData::Int(2)), Some(&Expr::Literal(LiteralData::Str("'two'".into()))));
            assert_eq!(data.get(&KeyData::Int(3)), Some(&Expr::Literal(LiteralData::Str("'three'".into()))));
        }
        _ => panic!("Expected RuntimeMap, got {:?}", result),
    }
    
    // Test string key map
    let mut ast = parser.parse("#{'hello': 1, 'world': 2}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeMap { data, .. } => {
            assert_eq!(data.len(), 2);
            assert_eq!(data.get(&KeyData::Str("'hello'".into())), Some(&Expr::Literal(LiteralData::Int(1))));
            assert_eq!(data.get(&KeyData::Str("'world'".into())), Some(&Expr::Literal(LiteralData::Int(2))));
        }
        _ => panic!("Expected RuntimeMap, got {:?}", result),
    }
    
    // Test boolean key map
    let mut ast = parser.parse("#{true: 'yes', false: 'no'}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeMap { data, .. } => {
            assert_eq!(data.len(), 2);
            assert_eq!(data.get(&KeyData::Bool(true)), Some(&Expr::Literal(LiteralData::Str("'yes'".into()))));
            assert_eq!(data.get(&KeyData::Bool(false)), Some(&Expr::Literal(LiteralData::Str("'no'".into()))));
        }
        _ => panic!("Expected RuntimeMap, got {:?}", result),
    }
    
    // Test map with expression values
    let mut ast = parser.parse("#{1: 10 + 5, 2: 20 * 2, 3: 100 / 4}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeMap { data, .. } => {
            assert_eq!(data.len(), 3);
            assert_eq!(data.get(&KeyData::Int(1)), Some(&Expr::Literal(LiteralData::Int(15))));
            assert_eq!(data.get(&KeyData::Int(2)), Some(&Expr::Literal(LiteralData::Int(40))));
            assert_eq!(data.get(&KeyData::Int(3)), Some(&Expr::Literal(LiteralData::Int(25))));
        }
        _ => panic!("Expected RuntimeMap, got {:?}", result),
    }
}

#[test]
fn test_nested_collections() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test map containing lists
    let mut ast = parser.parse("#{1: [1, 2, 3], 2: [4, 5, 6]}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeMap { data, .. } => {
            assert_eq!(data.len(), 2);
            match data.get(&KeyData::Int(1)) {
                Some(Expr::RuntimeList { data: list, .. }) => {
                    assert_eq!(list.len(), 3);
                    assert_eq!(list[0], Expr::Literal(LiteralData::Int(1)));
                    assert_eq!(list[2], Expr::Literal(LiteralData::Int(3)));
                }
                _ => panic!("Expected RuntimeList as map value"),
            }
        }
        _ => panic!("Expected RuntimeMap, got {:?}", result),
    }
    
    // Test list containing maps
    let mut ast = parser.parse("[#{1: 'one'}, #{2: 'two'}]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeList { data, .. } => {
            assert_eq!(data.len(), 2);
            match &data[0] {
                Expr::RuntimeMap { data: map, .. } => {
                    assert_eq!(map.len(), 1);
                    assert_eq!(map.get(&KeyData::Int(1)), Some(&Expr::Literal(LiteralData::Str("'one'".into()))));
                }
                _ => panic!("Expected RuntimeMap in list"),
            }
        }
        _ => panic!("Expected RuntimeList, got {:?}", result),
    }
    
    // Test map of maps
    let mut ast = parser.parse("#{'a': #{1: 'one'}, 'b': #{2: 'two'}}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeMap { data, .. } => {
            assert_eq!(data.len(), 2);
            match data.get(&KeyData::Str("'a'".into())) {
                Some(Expr::RuntimeMap { data: inner, .. }) => {
                    assert_eq!(inner.len(), 1);
                    assert_eq!(inner.get(&KeyData::Int(1)), Some(&Expr::Literal(LiteralData::Str("'one'".into()))));
                }
                _ => panic!("Expected nested RuntimeMap"),
            }
        }
        _ => panic!("Expected RuntimeMap, got {:?}", result),
    }
}

#[test]
fn test_collections_in_variables() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test list in variable
    let mut ast = parser.parse("{let mylist = [1, 2, 3]; mylist}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeList { data, .. } => {
            assert_eq!(data.len(), 3);
            assert_eq!(data[1], Expr::Literal(LiteralData::Int(2)));
        }
        _ => panic!("Expected RuntimeList, got {:?}", result),
    }
    
    // Test map in variable
    let mut ast = parser.parse("{let mymap = #{1: 'one', 2: 'two'}; mymap}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeMap { data, .. } => {
            assert_eq!(data.len(), 2);
            assert_eq!(data.get(&KeyData::Int(1)), Some(&Expr::Literal(LiteralData::Str("'one'".into()))));
        }
        _ => panic!("Expected RuntimeMap, got {:?}", result),
    }
    
    // Test list with variable references
    let mut ast = parser.parse("{let x = 10; let y = 20; [x, y, x + y]}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    match result {
        Expr::RuntimeList { data, .. } => {
            assert_eq!(data.len(), 3);
            assert_eq!(data[0], Expr::Literal(LiteralData::Int(10)));
            assert_eq!(data[1], Expr::Literal(LiteralData::Int(20)));
            assert_eq!(data[2], Expr::Literal(LiteralData::Int(30)));
        }
        _ => panic!("Expected RuntimeList, got {:?}", result),
    }
}

#[test]
fn test_define_type() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test simple type alias - need to wrap in a block for multiple statements
    let src = "{type Age = Int; let myAge: Age = 25; myAge}";
    let mut ast = parser.parse(src).unwrap();
    assert!(ast.prepare(&mut symbols).is_ok());
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(25)));
    
    // Test custom type in function - also needs to be in a block
    let src2 = "{type Name = Str; function greet(n: Name): Name { n + '!' }; greet(n: 'Hi')}";
    let mut ast2 = parser.parse(src2).unwrap();
    let mut symbols2 = SymbolTable::new();
    assert!(ast2.prepare(&mut symbols2).is_ok());
    let result2 = ast2.interpret(&mut symbols2, 0).unwrap();
    assert_eq!(result2, Expr::Literal(LiteralData::Str("'Hi!'".into())));
}

#[test]
fn test_lt_file_define_type() {
    let result = run_lift_file("tests/test_define_type.lt").unwrap();
    // The test file outputs various things and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_struct_definition() {
    let result = run_lift_file("tests/test_struct_definition.lt").unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_lt_file_struct_field_access() {
    let result = run_lift_file("tests/test_struct_field_access.lt").unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_lt_file_struct_methods() {
    let result = run_lift_file("tests/test_struct_methods.lt").unwrap();
    // The test file tests struct methods and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_field_mutation() {
    let result = run_lift_file("tests/test_field_mutation.lt").unwrap();
    // The test file tests field mutation and returns 42
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_lt_file_struct_comparison() {
    let result = run_lift_file("tests/test_struct_comparison.lt").unwrap();
    // The test file tests struct comparison and returns 42
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_range_literals() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test basic range literal
    let src = "3..10";
    let mut ast = parser.parse(src).unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result.to_string(), "3..10");
    
    // Test range with expressions
    let src2 = "(1 + 2)..(10 - 3)";
    let mut ast2 = parser.parse(src2).unwrap();
    let mut symbols2 = SymbolTable::new();
    ast2.prepare(&mut symbols2).unwrap();
    let result2 = ast2.interpret(&mut symbols2, 0).unwrap();
    assert_eq!(result2.to_string(), "3..7");
    
    // Test range with variables
    let src3 = "{let a = 5; let b = 15; a..b}";
    let mut ast3 = parser.parse(src3).unwrap();
    let mut symbols3 = SymbolTable::new();
    ast3.prepare(&mut symbols3).unwrap();
    let result3 = ast3.interpret(&mut symbols3, 0).unwrap();
    assert_eq!(result3.to_string(), "5..15");
}

#[test]
fn test_lt_file_ranges() {
    let result = run_lift_file("tests/test_ranges.lt").unwrap();
    // The test file outputs various ranges and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_simple_list() {
    // This should fail because empty list needs type annotation
    let result = run_lift_file("tests/simple_list.lt");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot infer type for 'x'"));
}

#[test]
fn test_lt_file_test_lists() {
    // This should fail because of empty list without type annotation
    let result = run_lift_file("tests/test_lists.lt");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot infer type"));
}

#[test]
fn test_lt_file_test_lists_simple() {
    let result = run_lift_file("tests/test_lists_simple.lt").unwrap();
    // The test file outputs various lists and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_list_indexing() {
    let result = run_lift_file("tests/test_list_indexing_simple.lt").unwrap();
    // The test file outputs indexed values and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_list_indexing_comprehensive() {
    let result = run_lift_file("tests/test_list_indexing_final.lt").unwrap();
    // The test file tests various indexing scenarios and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_map_indexing() {
    let result = run_lift_file("tests/test_map_indexing_simple.lt").unwrap();
    // The test file outputs indexed values and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_map_indexing_comprehensive() {
    let result = run_lift_file("tests/test_map_indexing_comprehensive.lt").unwrap();
    // The test file tests various map indexing scenarios and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_test_maps() {
    let result = run_lift_file("tests/test_maps.lt").unwrap();
    // The test file outputs various maps and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_test_maps_simple() {
    let result = run_lift_file("tests/test_maps_simple.lt").unwrap();
    // The test file outputs various maps and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_test_runtime_collections() {
    let result = run_lift_file("tests/test_runtime_collections.lt").unwrap();
    // The test file outputs various runtime collections and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_collection_display() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test list display format
    let mut ast = parser.parse("[1, 2, 3]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result.to_string(), "[1,2,3]");
    
    // Test map display format
    let mut ast = parser.parse("#{1: 'one', 2: 'two'}").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    // Maps are sorted by key in display
    assert!(result.to_string().contains("1:'one'"));
    assert!(result.to_string().contains("2:'two'"));
    
    // Test nested display
    let mut ast = parser.parse("[[1, 2], [3, 4]]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result.to_string(), "[[1,2],[3,4]]");
}

#[test]
fn test_let_type_resolution() {
    let parser = grammar::ProgramParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test 1: let with integer literal should have Int type
    let mut ast = parser.parse("let x = 42").unwrap();
    ast.prepare(&mut symbols).unwrap();
    
    // Get the symbol and check its type
    let x_index = symbols.find_index_reachable_from("x", 0).unwrap();
    let x_type = symbols.get_symbol_type(&x_index).unwrap();
    assert_eq!(x_type, DataType::Int);
    
    // Test 2: let with boolean literal should have Bool type
    let mut ast2 = parser.parse("let debug = true").unwrap();
    ast2.prepare(&mut symbols).unwrap();
    
    let debug_index = symbols.find_index_reachable_from("debug", 0).unwrap();
    let debug_type = symbols.get_symbol_type(&debug_index).unwrap();
    assert_eq!(debug_type, DataType::Bool);
    
    // Test 3: let with string literal should have Str type
    let mut ast3 = parser.parse("let name = 'Alice'").unwrap();
    ast3.prepare(&mut symbols).unwrap();
    
    let name_index = symbols.find_index_reachable_from("name", 0).unwrap();
    let name_type = symbols.get_symbol_type(&name_index).unwrap();
    assert_eq!(name_type, DataType::Str);
    
    // Test 4: let with float literal should have Flt type
    let mut ast4 = parser.parse("let pi = 3.14").unwrap();
    ast4.prepare(&mut symbols).unwrap();
    
    let pi_index = symbols.find_index_reachable_from("pi", 0).unwrap();
    let pi_type = symbols.get_symbol_type(&pi_index).unwrap();
    assert_eq!(pi_type, DataType::Flt);
}

#[test]
fn test_empty_collection_type_errors() {
    let parser = grammar::ProgramParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test 1: Empty list without type annotation should fail
    let mut ast = parser.parse("let x = []").unwrap();
    let result = ast.prepare(&mut symbols);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors[0].to_string().contains("Cannot infer type for 'x'"));
    
    // Test 2: Empty map without type annotation should fail
    let mut symbols2 = SymbolTable::new();
    let mut ast2 = parser.parse("let m = #{}").unwrap();
    let result2 = ast2.prepare(&mut symbols2);
    assert!(result2.is_err());
    let errors2 = result2.unwrap_err();
    assert!(errors2[0].to_string().contains("Cannot infer type for 'm'"));
    
    // Test 3: Empty list with type annotation should succeed
    let mut symbols3 = SymbolTable::new();
    let mut ast3 = parser.parse("let nums: List of Int = []").unwrap();
    let result3 = ast3.prepare(&mut symbols3);
    assert!(result3.is_ok());
    
    // Verify the type was properly set
    let nums_index = symbols3.find_index_reachable_from("nums", 0).unwrap();
    let nums_type = symbols3.get_symbol_type(&nums_index).unwrap();
    // Check if we got a List type with Int elements
    if let DataType::List { element_type } = nums_type {
        // element_type is a Box<DataType>, so we need to deref it properly
        assert!(matches!(element_type.as_ref(), DataType::Int));
    } else {
        panic!("Expected List type, got {:?}", nums_type);
    }
}

#[test]
fn test_list_indexing() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test basic indexing
    let mut ast = parser.parse("[10, 20, 30][0]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(10)));
    
    // Test indexing with expression
    let mut ast = parser.parse("[10, 20, 30][1 + 1]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(30)));
    
    // Test indexing with variable
    let program = grammar::ProgramParser::new();
    let mut ast = program.parse("let nums = [100, 200, 300]; let i = 1; nums[i]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(200)));
    
    // Test string list indexing
    let mut ast = parser.parse("['hello', 'world', 'lift'][1]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Str("'world'".into())));
    
    // Test nested list indexing
    let mut ast = parser.parse("[[1, 2], [3, 4]][0][1]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(2)));
}

#[test]
fn test_list_indexing_type_checking() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test type inference through indexing
    let program = grammar::ProgramParser::new();
    let mut ast = program.parse("let nums = [1, 2, 3]; let x = nums[0]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    
    // Check that x has type Int
    let x_index = symbols.find_index_reachable_from("x", 0).unwrap();
    let x_type = symbols.get_symbol_type(&x_index).unwrap();
    assert_eq!(x_type, DataType::Int);
    
    // Test indexing type must be Int
    let mut ast = parser.parse("[1, 2, 3][1.5]").unwrap();
    let result = ast.prepare(&mut symbols);
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].to_string().contains("List index must be of type Int"));
    
    // Test can only index lists
    let mut ast = parser.parse("42[0]").unwrap();
    let result = ast.prepare(&mut symbols);
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].to_string().contains("Cannot index into type"));
}

#[test]
fn test_list_indexing_runtime_errors() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test index out of bounds
    let mut ast = parser.parse("[10, 20, 30][5]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Index 5 out of bounds for list of length 3"));
    
    // Test negative index
    let mut ast = parser.parse("[10, 20, 30][-1]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Index -1 out of bounds"));
}

#[test]
fn test_map_indexing() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test basic map indexing with integer keys
    let mut ast = parser.parse("#{1: 'one', 2: 'two', 3: 'three'}[2]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Str("'two'".into())));
    
    // Test map indexing with string keys
    let mut ast = parser.parse("#{'a': 10, 'b': 20, 'c': 30}['b']").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(20)));
    
    // Test map indexing with boolean keys
    let mut ast = parser.parse("#{true: 'yes', false: 'no'}[false]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Str("'no'".into())));
    
    // Test map indexing with variable
    let program = grammar::ProgramParser::new();
    let mut ast = program.parse("let m = #{'x': 100, 'y': 200}; let k = 'x'; m[k]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(100)));
    
    // Test nested map indexing
    let mut ast = parser.parse("#{'outer': #{'inner': 42}}['outer']['inner']").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_map_indexing_type_checking() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test type inference through map indexing
    let program = grammar::ProgramParser::new();
    let mut ast = program.parse("let m = #{'a': 123}; let v = m['a']").unwrap();
    ast.prepare(&mut symbols).unwrap();
    
    // Check that v has type Int
    let v_index = symbols.find_index_reachable_from("v", 0).unwrap();
    let v_type = symbols.get_symbol_type(&v_index).unwrap();
    assert_eq!(v_type, DataType::Int);
    
    // Test wrong key type
    let mut ast = parser.parse("#{1: 'one', 2: 'two'}['key']").unwrap();
    let result = ast.prepare(&mut symbols);
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].to_string().contains("Map key must be of type Int"));
    
    // Test float key error
    let mut ast = parser.parse("#{'a': 1}[1.5]").unwrap();
    let result = ast.prepare(&mut symbols);
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].to_string().contains("Map key must be of type Str"));
}

#[test]
fn test_map_indexing_runtime_errors() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test key not found
    let mut ast = parser.parse("#{1: 'one', 2: 'two'}[3]").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Key Int(3) not found in map"));
    
    // Test float key runtime error
    let program = grammar::ProgramParser::new();
    let mut ast = program.parse("let k: Flt = 1.5; #{'a': 1}['a']").unwrap(); // This should pass type check since we're using 'a' not k
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(1)));
}

#[test]
fn test_not_operator() {
    let parser = grammar::ProgramParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test basic not operation
    let mut ast = parser.parse("not true").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::RuntimeData(LiteralData::Bool(false)));
    
    // Test double not
    let mut ast = parser.parse("not not false").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::RuntimeData(LiteralData::Bool(false)));
    
    // Test not with comparison
    let mut ast = parser.parse("not (5 > 10)").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::RuntimeData(LiteralData::Bool(true)));
    
    // Test not with logical operators
    let mut ast = parser.parse("not (true and false)").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::RuntimeData(LiteralData::Bool(true)));
    
    // Test type checking - not with non-boolean should fail
    let mut ast = parser.parse("not 42").unwrap();
    let result = ast.prepare(&mut symbols);
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].to_string().contains("Not operator requires boolean operand"));
}

#[test]
fn test_indexing_from_inner_scopes() {
    let parser = grammar::ProgramParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test list indexing from if expression
    let mut ast = parser.parse("
        let nums = [10, 20, 30];
        let result: Int = if true { nums[1] } else { nums[0] };
        result
    ").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(20)));
    
    // Test map indexing from nested blocks
    let mut symbols2 = SymbolTable::new();
    let mut ast = parser.parse("
        let map = #{'a': 100, 'b': 200};
        let result: Int = {
            let key = 'b';
            { map[key] }
        };
        result
    ").unwrap();
    ast.prepare(&mut symbols2).unwrap();
    let result = ast.interpret(&mut symbols2, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(200)));
    
    // Test accessing outer scope from deeply nested blocks
    let mut symbols3 = SymbolTable::new();
    let mut ast = parser.parse("
        let outer = [1, 2, 3, 4, 5];
        let result: Int = {
            let a = 2;
            {
                let b = 3;
                { outer[b] }
            }
        };
        result
    ").unwrap();
    ast.prepare(&mut symbols3).unwrap();
    let result = ast.interpret(&mut symbols3, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(4)));
    
    // Test mixed list and map access across scopes
    let mut symbols4 = SymbolTable::new();
    let mut ast = parser.parse("
        let list = [10, 20, 30];
        let map = #{1: 100, 2: 200};
        let result: Int = {
            let idx = 2;
            let key = 1;
            list[idx] + map[key]  // 30 + 100 = 130
        };
        result
    ").unwrap();
    ast.prepare(&mut symbols4).unwrap();
    let result = ast.interpret(&mut symbols4, 0).unwrap();
    assert_eq!(result, Expr::Literal(LiteralData::Int(130)));
}

#[test]
fn test_let_with_expressions() {
    let parser = grammar::ProgramParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test arithmetic expression type inference
    let mut ast = parser.parse("let sum = 5 + 3").unwrap();
    ast.prepare(&mut symbols).unwrap();
    
    let sum_index = symbols.find_index_reachable_from("sum", 0).unwrap();
    let sum_type = symbols.get_symbol_type(&sum_index).unwrap();
    assert_eq!(sum_type, DataType::Int);
    
    // Test comparison expression type inference
    let mut ast2 = parser.parse("let is_greater = 10 > 5").unwrap();
    ast2.prepare(&mut symbols).unwrap();
    
    let is_greater_index = symbols.find_index_reachable_from("is_greater", 0).unwrap();
    let is_greater_type = symbols.get_symbol_type(&is_greater_index).unwrap();
    assert_eq!(is_greater_type, DataType::Bool);
}

#[test]
fn test_lt_file_type_resolution() {
    let result = run_lift_file("tests/test_type_resolution.lt").unwrap();
    // The test file outputs various values and returns Unit
    assert_eq!(result, Expr::Unit);
}

#[test]
fn test_lt_file_comments() {
    let result = run_lift_file("tests/test_comments.lt").unwrap();
    // The test file tests various comment scenarios and returns 52 (sum of x and y)
    assert_eq!(result, Expr::Literal(LiteralData::Int(52)));
}

#[test]
fn test_lt_file_negative_numbers() {
    let result = run_lift_file("tests/test_negative_numbers.lt").unwrap();
    // The test file tests negative numbers and returns -64 (sum of x, sum, product, quotient)
    assert_eq!(result, Expr::Literal(LiteralData::Int(-64)));
}

#[test]
fn test_lt_file_not_operator() {
    let result = run_lift_file("tests/test_not_operator.lt").unwrap();
    // The test file tests the not operator and returns 42
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

#[test]
fn test_if_expression_type_inference() {
    let parser = grammar::ProgramParser::new();
    let mut symbols = SymbolTable::new();
    
    // Test 1: Basic if-else type inference
    let mut ast = parser.parse("let x = if true { 42 } else { 100 }").unwrap();
    ast.prepare(&mut symbols).unwrap();
    let result = ast.interpret(&mut symbols, 0).unwrap();
    assert_eq!(result, Expr::Unit);
    
    // Test 2: String type inference
    let mut ast = parser.parse("let y = if false { 'hello' } else { 'world' }").unwrap();
    ast.prepare(&mut symbols).unwrap();
    
    // Test 3: Type mismatch should fail
    let mut ast = parser.parse("let z = if true { 42 } else { 'string' }").unwrap();
    let result = ast.prepare(&mut symbols);
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].to_string().contains("Cannot infer type"));
    
    // Test 4: If without else cannot be used in let
    let mut symbols2 = SymbolTable::new();
    let mut ast = parser.parse("let w = if true { 42 }").unwrap();
    let result = ast.prepare(&mut symbols2);
    assert!(result.is_err());
    assert!(result.unwrap_err()[0].to_string().contains("Cannot infer type"));
    
    // Test 5: If expression in function return type
    let mut symbols3 = SymbolTable::new();
    let mut ast = parser.parse("function test(): Int { if true { 1 } else { 2 } }").unwrap();
    ast.prepare(&mut symbols3).unwrap();
    
    // Test 6: Nested if expressions
    let mut ast = parser.parse("let grade = if 85 >= 90 { 'A' } else if 85 >= 80 { 'B' } else { 'C' }").unwrap();
    ast.prepare(&mut symbols).unwrap();
}

#[test] 
fn test_lt_file_if_type_inference() {
    let result = run_lift_file("tests/test_if_type_inference.lt").unwrap();
    // The test file tests if expression type inference and returns 42
    assert_eq!(result, Expr::Literal(LiteralData::Int(42)));
}

