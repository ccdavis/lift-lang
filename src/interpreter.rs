use crate::semantic_analysis::*;
use crate::symboltable::SymbolTable;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::Function;
use crate::syntax::KeywordArg;
use crate::syntax::LiteralData;
use crate::syntax::Operator;
use std::error;
use std::error::Error;

// TODO this should eventually  store line numbers, columns in source and function names
#[derive(Debug, Clone)]
pub struct RuntimeError {
    stack: Option<Vec<String>>, // should be able to unwind the stack
    location: Option<(usize, usize)>,
    pub msg: String,
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stack_trace = if let Some(ref trace) = self.stack {
            trace.join("\n")
        } else {
            "\n".to_string()
        };

        if let Some((line, column)) = self.location {
            write!(f, "{}{}, {}: {}", &stack_trace, line, column, self.msg)
        } else {
            write!(f, "{}{}", &stack_trace, self.msg)
        }
    }
}
impl RuntimeError {
    pub fn new(msg: &str, location: Option<(usize, usize)>, stack: Option<Vec<String>>) -> Self {
        Self {
            msg: msg.to_string(),
            location,
            stack,
        }
    }
}

impl Error for RuntimeError {
    fn description(&self) -> &str {
        &self.msg
    }
}

pub type InterpreterResult = Result<Expr, Box<dyn error::Error>>;

impl Expr {
    pub fn prepare(&mut self, symbols: &mut SymbolTable) -> Result<(), Vec<CompileError>> {
        let mut errors = Vec::new();

        // Analyze  parse tree to index symbols across scopes.
        let result = add_symbols(self, symbols, 0);
        if let Err(ref msg) = result {
            eprintln!("Error indexing variable and function names: {}", msg);
            errors.push(msg.clone());
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
            Expr::Literal(_) => Ok(self.clone()),
            Expr::RuntimeData(_) => Ok(self.clone()),
            Expr::Program {
                ref body,
                ref environment,
            } => interpret_program(symbols, body, *environment),
            Expr::Block {
                ref body,
                ref environment,
            } => interpret_block(symbols, body, *environment),
            Expr::Let {
                ref var_name,
                ref value,
                ref index,
                ref data_type,
            } => interpret_let(symbols, var_name, data_type, value, index),
            Expr::BinaryExpr {
                ref left,
                op,
                ref right,
            } => interpret_binary(symbols, left, op, right, current_scope),
            Expr::Variable {
                ref name,
                ref index,
            } => interpret_var(symbols, name, index),
            Expr::If {
                ref cond,
                ref then,
                ref final_else,
            } => interpret_if(symbols, cond, then, final_else, current_scope),
            Expr::While { ref cond, ref body } => {
                interpret_while(symbols, current_scope, cond, body)
            }
            Expr::Call {
                ref fn_name,
                ref index,
                ref args,
            } => interpret_call(symbols, current_scope, fn_name, *index, args),
            Expr::Lambda {
                ref value,
                environment,
            } => interpret_lambda(symbols, value, *environment),
            Expr::DefineFunction { .. } => Ok(Expr::Unit), // The function got assigned in an earlier compiler pass
            _ => panic!(
                "Interpreter error: interpret() not implemented for '{:?}'",
                self
            ),
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
    let mut tmp_expr_result: InterpreterResult = Ok(Expr::Unit);
    for exp in body {
        tmp_expr_result = exp.interpret(symbols, env);
        if let Err(ref err) = tmp_expr_result {
            eprintln!("Runtime error: {}", err);
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
    symbols.update_runtime_value(result, index);
    Ok(Expr::Unit)
}

fn interpret_call(
    symbols: &mut SymbolTable,
    current_scope: usize,
    fn_name: &str,
    index: (usize, usize),
    args: &[KeywordArg],
) -> InterpreterResult {
    // Get the lambda for this function
    let maybe_lambda = symbols.get_compiletime_value(&index);
    if maybe_lambda.is_none() {
        symbols.print_debug();
        panic!(
            "Compiler Error: Can't find function definition for '{}' at index '{},{}', in scope {}",
            &fn_name, &index.0, &index.1, current_scope
        );
    }

    let lm = maybe_lambda.unwrap();

    // If the call has any arguments we have to  evaluate them in the current scope before passing to the
    // lambda  (by updating the lambda's  environment with their values.)
    // If the call has no arguments, the expression bound to this "function" doesn't need to be a lambda;
    // we just evaluate it in the function's captured scope (the index).
    match lm {
        Expr::Lambda { value, environment } => {
            if args.len() != value.params.len() {
                // TODO this should be in the compile pass
                panic!(
                    "Interpreter error: Function {} called with wrong number of arguments.",
                    fn_name
                );
            }

            for a in args {
                let arg_value = a.value.interpret(symbols, current_scope)?;

                // TODO this part should be done in a compiler pass, it's sort of slow this way.
                if let Some(assign_to_index) = symbols.get_index_in_scope(&a.name, environment) {
                    symbols.update_runtime_value(arg_value, &(environment, assign_to_index));
                } else {
                    panic!("Interpreter error: Keyword arg names must match the function definition parameters.");
                }
            }

            interpret_lambda(symbols, &value, environment)
        }
        _ => {
            if args.len() > 0 {
                // TODO this should really be in the compile pass
                panic!("Interpreter error: function {} called with {} args but it is a simple expression not a lambda. The type checking pass should have caught this.",fn_name, args.len());
            }
            lm.interpret(symbols, current_scope)
        }
    }
}

fn interpret_lambda(
    symbols: &mut SymbolTable,
    value: &Function,
    environment: usize,
) -> InterpreterResult {
    value.body.interpret(symbols, environment)
}

fn interpret_var(
    symbols: &mut SymbolTable,
    name: &str,
    index: &(usize, usize),
) -> InterpreterResult {
    let stored_value: Expr = match symbols.get_runtime_value(index) {
        Some(value) => value,
        None => {
            return Err(Box::new(RuntimeError::new(
                &format!("Symbol '{}' not found at runtime", name),
                None,
                None,
            )))
        }
    };
    if let Expr::RuntimeData(d) = stored_value {
        Ok(Expr::Literal(d))
    } else {
        Ok(stored_value)
    }
}

// Given scopes in 'symbols', evaluate 'cond' within scope 'current_scope' as true or false
fn interprets_as_true(
    symbols: &mut SymbolTable,
    current_scope: usize,
    cond: &Expr,
) -> Result<bool, Box<dyn Error>> {
    if let Expr::Literal(LiteralData::Bool(b)) = cond.interpret(symbols, current_scope)? {
        Ok(b)
    } else {
        panic!("Can't use expression '{:?}' as boolean. This is an interpreter bug. The type checker should have caught this.",cond);
    }
}

fn interpret_if(
    symbols: &mut SymbolTable,
    cond: &Expr,
    then: &Expr,
    final_else: &Expr,
    current_scope: usize,
) -> InterpreterResult {
    if interprets_as_true(symbols, current_scope, cond)? {
        then.interpret(symbols, current_scope)
    } else {
        final_else.interpret(symbols, current_scope)
    }
}

fn interpret_while(
    symbols: &mut SymbolTable,
    current_scope: usize,
    cond: &Expr,
    body: &Expr,
) -> InterpreterResult {
    while interprets_as_true(symbols, current_scope, cond)? {
        body.interpret(symbols, current_scope)?;
    }
    Ok(Expr::Unit)
}

impl LiteralData {
    fn apply_binary_operator(&self, rhs: &LiteralData, op: &Operator) -> InterpreterResult {
        use LiteralData::*;
        use Operator::*;

        let result = match (op, self, rhs) {
            (Add, Int(l), Int(r)) => Int(l + r),
            (Add, Flt(l), Flt(r)) => Flt(l + r),
            (Add, Str(l), Str(r)) => LiteralData::Str((l.to_string() + &r).into()),
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
            (Eq, Str(ref l), Str(ref r)) => Bool(l == r),

            (Neq, Int(l), Int(r)) => Bool(l != r),
            (Neq, Flt(l), Flt(r)) => Bool(l != r),
            (Neq, Bool(l), Bool(r)) => Bool(l != r),
            (Neq, Str(l), Str(r)) => Bool(l != r),
            _ => {
                // The type checker and parser should have prevented us from
                // reaching this point.
                let msg = format!("{:?} not allowed on {:?},{:?}", op, self, rhs);
                return Err(RuntimeError::new(&msg, None, None).into());
            }
        };
        Ok(Expr::Literal(result))
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
    let mut result: InterpreterResult = Ok(Expr::Unit);

    // This is repetaative because we are optimizing for the case where the expressions
    // are literal values (primary expressions) and don't need to be interpreted.
    // This saves a clone().
    match (left, right) {
        (Expr::Literal(l_value), Expr::Literal(r_value)) => {
            result = l_value.apply_binary_operator(r_value, op)
        }
        (_, Expr::Literal(r_value)) => {
            if let Expr::Literal(ref l_value) = left.interpret(symbols, current_scope)? {
                result = l_value.apply_binary_operator(r_value, op);
            } else {
                let msg = format!(
                    "Result of {:?} isn't a simple primary expression. Cannot apply {:?} to it.",
                    left, op
                );
                error = Some(RuntimeError::new(&msg, None, None).into());
            }
        }
        (Expr::Literal(l_value), _) => {
            if let Expr::Literal(ref r_value) = right.interpret(symbols, current_scope)? {
                result = l_value.apply_binary_operator(r_value, op);
            } else {
                let msg = format!(
                    "Result of {:?} isn't a simple primary expression. Cannot apply {:?} to it.",
                    right, op
                );
                error = Some(RuntimeError::new(&msg, None, None).into());
            }
        }
        (_, _) => {
            let l_value = left.interpret(symbols, current_scope)?;
            let r_value = right.interpret(symbols, current_scope)?;
            if let (Expr::Literal(ref l_data), Expr::Literal(ref r_data)) = (l_value, r_value) {
                result = l_data.apply_binary_operator(r_data, op);
            } else {
                let msg = format!(
                    "Expressions don't evaluate to anything applicable to a binary operator: {:?}, {:?}",
                    &left, &right
                );
                error = Some(RuntimeError::new(&msg, None, None));
            }
        }
    }
    if let Some(e) = error {
        Err(e.into())
    } else {
        result
    }
}
