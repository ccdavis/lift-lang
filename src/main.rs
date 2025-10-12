mod interpreter;
mod semantic; // Modular semantic analysis
mod symboltable;
mod syntax;
mod compile_types;
mod runtime;
mod cranelift;
mod compiler;

use lalrpop_util::{lalrpop_mod, ParseError};
use std::error;
use std::fs;
use symboltable::SymbolTable;
use syntax::*;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

lalrpop_mod!(pub grammar); // synthesized by LALRPOP

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
    let mut ast = match parser.parse(code) {
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

fn compile_code(code: &str) -> Result<(), Box<dyn error::Error>> {
    use syntax::DataType;
    use semantic::determine_type_with_symbols;

    let parser = grammar::ProgramParser::new();
    let mut ast = match parser.parse(code) {
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

    // Get the expression type before compilation
    let expr_type = determine_type_with_symbols(&ast, &symbols, 0).unwrap_or(DataType::Unsolved);

    // Compile and run
    let mut jit = compiler::JITCompiler::new()?;
    let result = jit.compile_and_run(&ast, &symbols)?;

    // Format output to match interpreter
    match expr_type {
        DataType::Unsolved => println!("Unit"),
        DataType::Int => println!("{}", result),
        DataType::Flt => {
            let float_val = f64::from_bits(result as u64);
            println!("{}", float_val);
        }
        DataType::Bool => {
            println!("{}", if result != 0 { "true" } else { "false" });
        }
        _ => {
            // For complex types (Str, List, Map, Range), the result is a pointer
            // These types should have already been output by lift_output_* functions
            // So we print nothing (they handle their own output)
        }
    }
    Ok(())
}

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    // Check for flags
    let use_compiler = args.iter().any(|arg| arg == "--compile" || arg == "-c");

    if args.len() < 2 || (args.len() == 2 && use_compiler) {
        repl();
    } else {
        // Find the program file (skip flags)
        let program_file = args.iter()
            .skip(1)
            .find(|arg| !arg.starts_with('-'))
            .expect("No program file specified");

        let code = fs::read_to_string(program_file)
            .unwrap_or_else(|_| panic!("File at {} unreadable.", program_file));

        if use_compiler {
            if let Err(e) = compile_code(&code) {
                eprintln!("Compilation error: {}", e);
            }
        } else {
            if let Err(e) = interpret_code(&code) {
                eprintln!("Error: {}", e);
            }
        }
    }
}
