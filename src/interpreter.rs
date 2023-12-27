use crate::semantic_analysis::ParseError;
use crate::semantic_analysis::*;
use crate::symboltable::SymbolTable;
use crate::syntax::Expr;
use crate::syntax::LiteralData;
use crate::syntax::Operator;

// TODO this should eventually  store line numbers, columns in source and function names
pub struct RuntimeError {
    stack: Vec<String>,
    pub msg: String,
}

type InterpreterResult = Result<Option<Expr>, RuntimeError>;

impl Expr {
    pub fn prepare(&mut self, symbols: &mut SymbolTable) -> Result<(), Vec<ParseError>> {
        let mut errors = Vec::new();

        // Analyze  parse tree to index symbols across scopes.
        let result = add_symbols(self, symbols, 0);
        if let Err(ref msg) = result {
            eprintln!("Error indexing variable and function names: {}", msg);
            errors.push(msg.to_string());
        }
        // Collect other errors...

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // Receives a "prepared" parse tree and symbol table.
    pub fn interpret(&self, symbols: &mut SymbolTable, current_scope: usize) -> InterpreterResult {
        match self {
            Expr::Program { body, environment } => interpret_program(symbols, body, *environment),
            Expr::Block { body, environment } => interpret_block(symbols, body, *environment),
            Expr::BinaryExpr { left, op, right } => {
                interpret_binary(symbols, left, op.clone(), right, current_scope)
            }

            _ => Ok(None),
        }
    }
} // impl

fn interpret_program(symbols: &mut SymbolTable, body: &Vec<Expr>, env: usize) -> InterpreterResult {
    interpret_body_or_block(symbols, body, env)
}

fn interpret_block(symbols: &mut SymbolTable, body: &Vec<Expr>, env: usize) -> InterpreterResult {
    interpret_body_or_block(symbols, body, env)
}

fn interpret_body_or_block(
    symbols: &mut SymbolTable,
    body: &Vec<Expr>,
    env: usize,
) -> InterpreterResult {
    let mut tmp_expr_result: InterpreterResult = Ok(None);
    for exp in body {
        tmp_expr_result = exp.interpret(symbols, env);
        if let Err(ref err) = tmp_expr_result {
            eprintln!("Runtime error: {}", err.msg);
            return tmp_expr_result;
        }
    }
    tmp_expr_result
}

fn interpret_binary(
    symbols: &mut SymbolTable,
    left: &Expr,
    op: Operator,
    right: &Expr,
    current_scope: usize,
) -> InterpreterResult {
    let l_value = left.interpret(symbols, current_scope)?;
    let r_value = right.interpret(symbols, current_scope)?;
    if let (Some(Expr::Literal(ref l_data)), Some(Expr::Literal(ref r_data))) = (l_value, r_value) {
        match op {
            Operator::Add => match (l_data, r_data) {
                (LiteralData::Int(l), LiteralData::Int(r)) => {
                    Ok(Some(Expr::Literal(LiteralData::Int(l + r))))
                }
                (LiteralData::Flt(l), LiteralData::Flt(r)) => {
                    Ok(Some(Expr::Literal(LiteralData::Flt(l + r))))
                }
                _ => {
                    let msg = format!(
                        "Can't apply '+' operator to {:?} and {:?}",
                        &l_data, &r_data
                    );
                    let err = RuntimeError {
                        msg,
                        stack: Vec::new(),
                    };
                    Err(err)
                }
            },
            Operator::Sub => Ok(None),
            Operator::Mul => Ok(None),
            Operator::Div => Ok(None),
            _ => panic!("Not implemented: {:?}", op),
        }
    } else {
        let msg = format!(
            "Expressions don't evaluate to anything applicable to a binary operator: {:?}, {:?}",
            &left, &right
        );
        let err = RuntimeError {
            msg,
            stack: Vec::new(),
        };
        Err(err)
    }
}
