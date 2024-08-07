mod interpreter;
mod semantic_analysis;
mod symboltable;
mod syntax;
use interpreter::InterpreterResult;
use lalrpop_util::{lalrpop_mod, ParseError};
use std::error;
use std::error::Error;
use std::fs;
use symboltable::SymbolTable;
use syntax::*;

use rustyline::completion::FilenameCompleter;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use std::borrow::Cow::{self, Borrowed, Owned};

use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Cmd, CompletionType, Config, EditMode, Editor, KeyEvent};
use rustyline::{Completer, Helper, Hinter, Validator};

#[derive(Helper, Completer, Hinter, Validator)]
struct MyHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

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

    #[cfg(feature = "with-file-history")]
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
                    if let Some(continuation_line) = line.trim_right().strip_suffix('\\') {
                        buffer.push_str(continuation_line);
                        prompt = ">>".to_string();
                        continue;
                    } else {
                        buffer.push_str(line);
                        prompt = format!("{count} ==> ");
                    }
                    
                    match parser.parse(&buffer) {
                        Ok(mut ast) => {
                            rl.add_history_entry(buffer.as_str());

                            count += 1;
                            if let Err(errors) = ast.prepare(&mut symbols) {
                                for e in errors {
                                    eprintln!("{}", &e);
                                }
                                println!();
                            }
                            match ast.interpret(&mut symbols, 0) {
                                Err(interpreter_error) => eprintln!("{}", interpreter_error),
                                Ok(res) => println!("=> '{}'", &res),
                            }
                            buffer.clear();
                        }
                        Err(ref parse_error) => match parse_error {
                            ParseError::UnrecognizedEof { location, expected } => {
                                buffer.push('\n');
                                prompt = ">>".to_string();
                            }
                            _ => {
                                eprintln!("ERROR: {}", parse_error);
                                buffer.clear();
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
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            } // match
        } // loop
        if quit {break;}
    } // loop
    #[cfg(feature = "with-file-history")]
    rl.save_history("history.txt");
}

fn interpret_code(code: &str) -> Result<(), Box<dyn error::Error>> {
    let parser = grammar::ProgramPartExprParser::new();
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
    }

    let res = ast.interpret(&mut symbols, 0)?;
    println!("{}", res);
    Ok(())
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        repl();
    } else {
        let program_file = &args[1];
        let code = fs::read_to_string(program_file)
            .expect(&format!("File at {} unreadable.", program_file));

        if let Err(e) = interpret_code(&code) {
            eprintln!("Error: {}", e);
        }
    }
}
