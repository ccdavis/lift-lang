use crate::semantic_analysis::ParseError;
use crate::semantic_analysis::*;
use crate::symboltable::SymbolTable;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::LiteralData;
use crate::syntax::Operator;

// TODO this should eventually  store line numbers, columns in source and function names
#[derive(Debug)]
pub struct RuntimeError {
    stack: Vec<String>,
    pub msg: String,
}

pub type InterpreterResult = Result<Option<Expr>, RuntimeError>;

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
            Expr::Literal(_) => Ok(Some(self.clone())),
            Expr::Program { body, environment } => interpret_program(symbols, body, *environment),
            Expr::Block { body, environment } => interpret_block(symbols, body, *environment),
            Expr::Let {
                var_name,
                value,
                index,
                data_type,
            } => interpret_let(symbols, var_name, data_type, value, index),
            Expr::BinaryExpr { left, op, right } => {
                interpret_binary(symbols, left, op, right, current_scope)
            }
            Expr::Variable { name, index } => interpret_var(symbols, name, index),
            Expr::If {
                cond,
                then,
                final_else,
            } => interpret_if(symbols, cond, then, final_else, current_scope),

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

fn interpret_let(
    symbols: &mut SymbolTable,
    var_name: &str,
    data_type: &DataType,
    value: &Expr,
    index: &(usize, usize),
) -> InterpreterResult {
    // The analysis phase has already placed the variable
    // in the current scope; here we only need to
    // evaluate the right-hand side.
    let current_scope = index.0;
    let result = value.interpret(symbols, current_scope)?;
    if let Some(expr) = result {
        symbols.update_value(expr.into(), index);
        Ok(Some(Expr::Unit))
    } else {
        let msg = format!(
            "Didn't make any assignment to '{}': {:?}",
            var_name, data_type
        );
        let error = RuntimeError {
            msg,
            stack: Vec::new(),
        };
        Err(error)
    }
}

fn interpret_var(
    symbols: &mut SymbolTable,
    name: &str,
    index: &(usize, usize),
) -> InterpreterResult {
    let var_value = symbols.get_value(index).into();
    Ok(Some(var_value))
}

fn interpret_if(
    symbols: &mut SymbolTable,
    cond: &Expr,
    then: &Expr,
    final_else: &Expr,
    current_scope: usize,
) -> InterpreterResult {
    if let Some(Expr::Literal(LiteralData::Bool(true))) = cond.interpret(symbols, current_scope)? {
        then.interpret(symbols, current_scope)
    } else {
        final_else.interpret(symbols, current_scope)
    }
}

impl LiteralData {
    fn apply_binary_operator(&self, rhs: &LiteralData, op: &Operator) -> InterpreterResult {
        use LiteralData::*;
        use Operator::*;
        let mut error: Option<RuntimeError> = None;

        let result = match (op, self, rhs) {
            (Add, Int(l), Int(r)) => Int(l + r),
            (Add, Flt(l), Flt(r)) => Flt(l + r),
            (Add, Str(l), Str(r)) => LiteralData::Str(l.to_string() + r),
            (Sub, Int(l), Int(r)) => Int(l - r),
            (Sub, Flt(l), Flt(r)) => Flt(l - r),
            (Mul, Int(l), Int(r)) => Int(l * r),
            (Mul, Flt(l), Flt(r)) => Flt(l * r),
            (Div, Int(l), Int(r)) => Int(l / r),
            (Div, Flt(l), Flt(r)) => Flt(l / r),

            (Gt, Int(l), Int(r)) => Bool(l > r),
            (Gt, Flt(l), Flt(r)) => Bool(l > r),

            (Lt, Int(l), Int(r)) => Bool(l < r),
            (Lt, Flt(l), Flt(r)) => Bool(l < r),

            (Gte, Int(l), Int(r)) => Bool(l >= r),
            (Gte, Flt(l), Flt(r)) => Bool(l >= r),

            (Lte, Int(l), Int(r)) => Bool(l <= r),
            (Lte, Flt(l), Flt(r)) => Bool(l <= r),

            (Eq, Int(l), Int(r)) => Bool(l == r),
            (Eq, Flt(l), Flt(r)) => Bool(l == r),
            (Eq, Bool(l), Bool(r)) => Bool(l == r),
            (Eq, Str(l), Str(r)) => Bool(l == r),

            (Neq, Int(l), Int(r)) => Bool(l != r),
            (Neq, Flt(l), Flt(r)) => Bool(l != r),
            (Neq, Bool(l), Bool(r)) => Bool(l != r),
            (Neq, Str(l), Str(r)) => Bool(l != r),
            _ => {
                // The type checker and parser should have prevented us from
                // reaching this point.
                let msg = format!("{:?} not allowed on {:?},{:?}", op, self, rhs);
                error = Some(RuntimeError {
                    msg,
                    stack: Vec::new(),
                });
                self.clone()
            }
        };
        if error.is_none() {
            Ok(Some(Expr::Literal(result)))
        } else {
            Err(error.unwrap())
        }
    }
}

fn interpret_binary(
    symbols: &mut SymbolTable,
    left: &Expr,
    op: &Operator,
    right: &Expr,
    current_scope: usize,
) -> InterpreterResult {
    let mut error: Option<RuntimeError> = None;
    let mut result: InterpreterResult = Ok(None);

    // This is repeative because we are optimizing for the case where the expressions
    // are literal values (primary expressions) and don't need to be interpreted.
    // This saves a clone().
    match (left, right) {
        (Expr::Literal(l_value), Expr::Literal(r_value)) => {
            result = l_value.apply_binary_operator(r_value, op)
        }
        (_, Expr::Literal(r_value)) => {
            if let Some(Expr::Literal(ref l_value)) = left.interpret(symbols, current_scope)? {
                result = l_value.apply_binary_operator(r_value, op);
            } else {
                let msg = format!(
                    "Result of {:?} isn't a simple primary expression. Cannot apply {:?} to it.",
                    left, op
                );
                error = Some(RuntimeError {
                    msg,
                    stack: Vec::new(),
                });
            }
        }
        (Expr::Literal(l_value), _) => {
            if let Some(Expr::Literal(ref r_value)) = right.interpret(symbols, current_scope)? {
                result = l_value.apply_binary_operator(r_value, op);
            } else {
                let msg = format!(
                    "Result of {:?} isn't a simple primary expression. Cannot apply {:?} to it.",
                    right, op
                );
                error = Some(RuntimeError {
                    msg,
                    stack: Vec::new(),
                });
            }
        }
        (_, _) => {
            let l_value = left.interpret(symbols, current_scope)?;
            let r_value = right.interpret(symbols, current_scope)?;
            if let (Some(Expr::Literal(ref l_data)), Some(Expr::Literal(ref r_data))) =
                (l_value, r_value)
            {
                result = l_data.apply_binary_operator(r_data, op);
            } else {
                let msg = format!(
                    "Expressions don't evaluate to anything applicable to a binary operator: {:?}, {:?}",
                    &left, &right
                );
                error = Some(RuntimeError {
                    msg,
                    stack: Vec::new(),
                });
            }
        }
    }
    if error.is_none() {
        result
    } else {
        Err(error.unwrap())
    }
}
