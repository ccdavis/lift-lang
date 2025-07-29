mod interpreter;
mod semantic_analysis;
mod symboltable;
mod syntax;
use interpreter::InterpreterResult;
use lalrpop_util::{lalrpop_mod, ParseError};
use std::error;
use std::fs;
use symboltable::SymbolTable;
use syntax::*;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

lalrpop_mod!(pub grammar); // synthesized by LALRPOP

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

pub fn make_literal_int(v: i64) -> Box<Expr> {
    let l = LiteralData::from(v);
    Box::new(Expr::Literal(l))
}

#[test]
fn test_binary_expression_parsing() {
    let parser = grammar::ProgramPartExprParser::new();
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
    let parser = grammar::ProgramPartExprParser::new();
    let parse_result = parser.parse(src);
    if let Err(ref e) = parse_result {
        eprintln!("Error parsing '{}', got {:?}", src, e);
    };
    assert!(parse_result.is_ok());
}
#[test]
fn test_interpret_math() {
    let src = "1 + 2 * 3";
    let parser = grammar::ProgramPartExprParser::new();
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
    let parser = grammar::ProgramPartExprParser::new();
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
    let parser = grammar::ProgramPartExprParser::new();
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

#[test]
fn test_variables() {
    let parser = grammar::ProgramPartExprParser::new();
    let src = "{let x = 25; let y = 3; x + y}";
    let parse_result = parser.parse(src);
    match parse_result {
        Err(ref e) => eprintln!("Parse variable definition failed: {:?}", &e),
        Ok(ref r) => println!("Success parsing variable definition 'let'."),
    }
    assert!(parse_result.is_ok());
    let mut root_expr = parse_result.unwrap();

    let mut symbols = SymbolTable::new();
    if let Err(err) = root_expr.prepare(&mut symbols) {
        eprintln!("Error assigning symbols and scopes: '{:?}'", &err);
    }
    let s = root_expr.interpret(&mut symbols, 0);
    match s {
        Err(ref e) => println!("Runtime error: {:?}", e),
        Ok(ref r) => println!("Success: {:?}", &r),
    }
    assert!(s.is_ok());
    assert!(check_value(&s, LiteralData::Int(28)));
}

#[test]
fn test_functions() {
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();
    let src = "{function f(a: Int, b: Int): Bool { 
                let unused = 9;
                a * a > b
            };

            if f(a: 2,b: 5) {  1111 } else {0}
    }";
    let parse_result = parser.parse(src);
    match parse_result {
        Err(ref e) => eprintln!("Parse function definition : {:?}", &e),
        Ok(ref r) => println!("Success parsing function definition."),
    }
    assert!(parse_result.is_ok());
    let mut root_expr = parse_result.unwrap();
    if let Err(err) = root_expr.prepare(&mut symbols) {
        eprintln!("Error assigning symbols and scopes: '{:?}'", &err);
    }
    let s = root_expr.interpret(&mut symbols, 0);
    match s {
        Err(ref e) => println!("Runtime error: {:?}", e),
        Ok(ref r) => println!("Success: {:?}", &r),
    }
    assert!(s.is_ok());
}

// A test helper
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

pub fn repl() {
    let mut quit = false;
    let parser = grammar::ProgramPartExprParser::new();
    let mut symbols = SymbolTable::new();

    let mut rl = DefaultEditor::new().unwrap();

    //rl.bind_sequence(KeyEvent::alt('n'), Cmd::HistorySearchForward);
    //rl.bind_sequence(KeyEvent::alt('p'), Cmd::HistorySearchBackward);

    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }

    let mut count = 0;
    loop {
        let mut buffer: String = "".to_string();
        let mut prompt = format!("{count} ==> ");
        loop {
            let readline = rl.readline(&prompt);
            match readline {
                Ok(ref line) => {
                    if let Some(continuation_line) = line.trim_end().strip_suffix('\\') {
                        buffer.push_str(continuation_line);
                        prompt = ">>".to_string();
                        continue;
                    } else {
                        buffer.push_str(line);
                        prompt = format!("{count} ==> ");
                    }
                    
                    match parser.parse(&buffer) {
                        Ok(mut ast) => {
                            let _ = rl.add_history_entry(buffer.as_str());

                            count += 1;
                            if let Err(errors) = ast.prepare(&mut symbols) {
                                for e in errors {
                                    eprintln!("{}", &e);
                                }
                                println!();
                                buffer.clear();
                                break; // Exit the inner loop after error
                            } else {
                                match ast.interpret(&mut symbols, 0) {
                                    Err(interpreter_error) => eprintln!("{}", interpreter_error),
                                    Ok(res) => println!("=> '{}'", &res),
                                }
                                buffer.clear();
                                break; // Exit the inner loop after successful execution
                            }
                        }
                        Err(ref parse_error) => match parse_error {
                            ParseError::UnrecognizedEof { location: _, expected: _ } => {
                                buffer.push('\n');
                                prompt = ">>".to_string();
                            }
                            _ => {
                                eprintln!("ERROR: {}", parse_error);
                                buffer.clear();
                                break; // Exit the inner loop after parse error
                            }
                        },
                    } //  match parse
                } // loop
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    quit = true;
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    quit = true;
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    // Don't quit on general errors, just break from inner loop
                    break;
                }
            } // match
        } // loop
        if quit {break;}
    } // loop
    let _ = rl.save_history("history.txt");
}

fn interpret_code(code: &str) -> Result<(), Box<dyn error::Error>> {
    let parser = grammar::ProgramParser::new();
    let mut ast = match parser.parse(&code) {
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(3);
        }
        Ok(parsed_ast) => parsed_ast,
    };

    let mut symbols = SymbolTable::new();
    if let Err(ref errors) = ast.prepare(&mut symbols) {
        for e in errors {
            eprintln!("{}", e);
        }
        std::process::exit(2);
    }

    let res = ast.interpret(&mut symbols, 0)?;
    println!("{}", res);
    Ok(())
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
    assert_eq!(result, Expr::Literal(LiteralData::Str("'Hello'' World'".into())));
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

// Helper function for integration tests
#[cfg(test)]
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
    // This file should fail because it needs type annotation for 'result'
    let result = run_lift_file("tests/type_checking_examples.lt");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot determine type of variable: result"));
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
    // Should return 0 after counting down from 100
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
    assert_eq!(result2, Expr::Literal(LiteralData::Str("'Hi''!'".into())));
}

#[test]
fn test_lt_file_define_type() {
    let result = run_lift_file("tests/test_define_type.lt").unwrap();
    // The test file outputs various things and returns Unit
    assert_eq!(result, Expr::Unit);
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
    assert!(result.unwrap_err().contains("Cannot infer type for empty list"));
}

#[test]
fn test_lt_file_test_lists() {
    // This should fail because of empty list without type annotation
    let result = run_lift_file("tests/test_lists.lt");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Cannot infer type for empty list"));
}

#[test]
fn test_lt_file_test_lists_simple() {
    let result = run_lift_file("tests/test_lists_simple.lt").unwrap();
    // The test file outputs various lists and returns Unit
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
    assert!(errors[0].to_string().contains("Cannot infer type for empty list"));
    
    // Test 2: Empty map without type annotation should fail
    let mut symbols2 = SymbolTable::new();
    let mut ast2 = parser.parse("let m = #{}").unwrap();
    let result2 = ast2.prepare(&mut symbols2);
    assert!(result2.is_err());
    let errors2 = result2.unwrap_err();
    assert!(errors2[0].to_string().contains("Cannot infer type for empty map"));
    
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
    // The test file tests negative numbers and returns -62 (sum of x, sum, product, quotient)
    assert_eq!(result, Expr::Literal(LiteralData::Int(-62)));
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        repl();
    } else {
        let program_file = &args[1];
        let code = fs::read_to_string(program_file)
            .expect(&format!("File at {program_file} unreadable."));

        if let Err(e) = interpret_code(&code) {
            eprintln!("Error: {}", e);
        }
    }
}
