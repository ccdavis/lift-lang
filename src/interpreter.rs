use crate::semantic_analysis::*;
use crate::symboltable::SymbolTable;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::Function;
use crate::syntax::KeywordArg;
use crate::syntax::LiteralData;
use crate::syntax::Operator;
use std::collections::HashMap;
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

        // Add built-in methods to the symbol table
        if let Err(err) = symbols.add_builtins() {
            errors.push(err);
        }

        // Analyze  parse tree to index symbols across scopes.
        let result = add_symbols(self, symbols, 0);
        if let Err(ref msg) = result {
            errors.push(msg.clone());
        }
        
        // Type check the expression tree
        let type_result = typecheck(self, symbols, 0);
        if let Err(ref msg) = type_result {
            errors.push(msg.clone());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // Receives a "prepared" parse tree and symbol table.
    pub fn interpret(&self, symbols: &mut SymbolTable, current_scope: usize) -> InterpreterResult {
        match self {
            Expr::Output { data } => interpret_output(symbols, data, current_scope),
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
                mutable: _,
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
            } => {
                // Generic call handling - works for both regular functions and UFCS method calls
                interpret_call(symbols, current_scope, fn_name, *index, args)
            },
            Expr::MethodCall {
                ref receiver,
                ref method_name,
                ref fn_index,
                ref args,
            } => {
                // Evaluate the receiver
                let receiver_value = receiver.interpret(symbols, current_scope)?;

                // Look up the method from the symbol table and extract builtin info
                let builtin_method = if let Some(fn_expr) = symbols.get_symbol_value(fn_index) {
                    match fn_expr {
                        Expr::DefineFunction { value, .. } => {
                            match value.as_ref() {
                                Expr::Lambda { value: func, .. } => {
                                    func.builtin.clone()
                                }
                                _ => None
                            }
                        }
                        _ => None
                    }
                } else {
                    return Err(Box::new(RuntimeError::new(
                        &format!("Method '{}' not found in symbol table", method_name),
                        None, None
                    )));
                };

                // Now handle the method call based on whether it's built-in or user-defined
                if let Some(builtin) = builtin_method {
                    // Evaluate all arguments
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        evaluated_args.push(arg.value.interpret(symbols, current_scope)?);
                    }

                    // Execute the built-in method
                    builtin.execute(receiver_value, evaluated_args)
                        .map_err(|e| Box::new(RuntimeError::new(&e.to_string(), None, None)) as Box<dyn std::error::Error>)
                } else {
                    // User-defined method - construct args with self
                    let mut all_args = vec![KeywordArg {
                        name: "self".to_string(),
                        value: receiver_value,
                    }];
                    all_args.extend_from_slice(args);

                    interpret_call(symbols, current_scope, method_name, *fn_index, &all_args)
                }
            },
            Expr::Lambda {
                ref value,
                environment,
            } => interpret_lambda(symbols, value, *environment),
            Expr::DefineFunction { .. } => Ok(Expr::Unit), // The function got assigned in an earlier compiler pass
            Expr::DefineType { .. } => Ok(Expr::Unit), // The type definition was registered in an earlier compiler pass
            Expr::Unit => Ok(Expr::Unit),
            Expr::ListLiteral { ref data_type, ref data } => {
                // Evaluate each element in the list and convert to runtime representation
                let mut evaluated_elements = Vec::new();
                for element in data {
                    evaluated_elements.push(element.interpret(symbols, current_scope)?);
                }
                Ok(Expr::RuntimeList {
                    data_type: data_type.clone(),
                    data: evaluated_elements,
                })
            },
            Expr::MapLiteral { ref key_type, ref value_type, ref data } => {
                // Evaluate each value in the map and convert to runtime representation
                let mut evaluated_map = HashMap::new();
                for (key, value) in data {
                    let evaluated_value = value.interpret(symbols, current_scope)?;
                    evaluated_map.insert(key.clone(), evaluated_value);
                }
                Ok(Expr::RuntimeMap {
                    key_type: key_type.clone(),
                    value_type: value_type.clone(),
                    data: evaluated_map,
                })
            },
            Expr::RuntimeList { .. } => Ok(self.clone()), // Already in runtime form
            Expr::RuntimeMap { .. } => Ok(self.clone()),  // Already in runtime form
            Expr::Range(..) => Ok(self.clone()), // Range is a value type
            Expr::Index { expr, index } => {
                let evaluated_expr = expr.interpret(symbols, current_scope)?;
                let evaluated_index = index.interpret(symbols, current_scope)?;
                
                // Perform indexing based on expression type
                match &evaluated_expr {
                    Expr::RuntimeList { data, .. } => {
                        // For lists, index must be an integer
                        let index_value = match &evaluated_index {
                            Expr::Literal(LiteralData::Int(i)) | Expr::RuntimeData(LiteralData::Int(i)) => *i,
                            _ => return Err(Box::new(RuntimeError::new(
                                "List index must be an integer", 
                                None, 
                                None
                            ))),
                        };
                        
                        if index_value < 0 || index_value as usize >= data.len() {
                            return Err(Box::new(RuntimeError::new(
                                &format!("Index {} out of bounds for list of length {}", index_value, data.len()),
                                None,
                                None
                            )));
                        }
                        Ok(data[index_value as usize].clone())
                    },
                    Expr::RuntimeMap { data, .. } => {
                        // For maps, convert the index to a KeyData
                        let key = match &evaluated_index {
                            Expr::Literal(lit) | Expr::RuntimeData(lit) => {
                                match lit {
                                    LiteralData::Int(i) => crate::syntax::KeyData::Int(*i),
                                    LiteralData::Str(s) => crate::syntax::KeyData::Str(s.clone()),
                                    LiteralData::Bool(b) => crate::syntax::KeyData::Bool(*b),
                                    LiteralData::Flt(_) => return Err(Box::new(RuntimeError::new(
                                        "Map keys cannot be of type Float",
                                        None,
                                        None
                                    ))),
                                }
                            },
                            _ => return Err(Box::new(RuntimeError::new(
                                "Map key must be a literal value",
                                None,
                                None
                            ))),
                        };
                        
                        // Look up the key in the map
                        match data.get(&key) {
                            Some(value) => Ok(value.clone()),
                            None => Err(Box::new(RuntimeError::new(
                                &format!("Key {:?} not found in map", key),
                                None,
                                None
                            )))
                        }
                    },
                    _ => Err(Box::new(RuntimeError::new(
                        &format!("Cannot index into {:?}", evaluated_expr),
                        None,
                        None
                    )))
                }
            },
            Expr::UnaryExpr { op, expr } => {
                match op {
                    Operator::Not => {
                        let evaluated = expr.interpret(symbols, current_scope)?;
                        match evaluated {
                            Expr::Literal(LiteralData::Bool(b)) | Expr::RuntimeData(LiteralData::Bool(b)) => {
                                Ok(Expr::RuntimeData(LiteralData::Bool(!b)))
                            }
                            _ => Err(Box::new(RuntimeError::new(
                                &format!("'not' operator requires a boolean value, got {:?}", evaluated),
                                None,
                                None
                            )))
                        }
                    },
                    _ => panic!("Interpreter error: UnaryExpr operator {:?} not implemented", op)
                }
            },
            Expr::Len { expr } => {
                let evaluated = expr.interpret(symbols, current_scope)?;
                match evaluated {
                    Expr::RuntimeList { data, .. } => Ok(Expr::Literal(LiteralData::Int(data.len() as i64))),
                    Expr::RuntimeMap { data, .. } => Ok(Expr::Literal(LiteralData::Int(data.len() as i64))),
                    Expr::Literal(LiteralData::Str(s)) | Expr::RuntimeData(LiteralData::Str(s)) => {
                        // String length without quotes
                        let len = s.trim_matches('\'').len() as i64;
                        Ok(Expr::Literal(LiteralData::Int(len)))
                    },
                    _ => Err(Box::new(RuntimeError::new(
                        &format!("len() requires a string, list, or map, got {:?}", evaluated),
                        None,
                        None
                    )))
                }
            },
            Expr::Assign { name: _, value, index } => {
                // Evaluate the value expression
                let evaluated_value = value.interpret(symbols, current_scope)?;
                // Update the runtime value in the symbol table
                symbols.update_runtime_value(evaluated_value, index);
                // Assignment expressions return Unit
                Ok(Expr::Unit)
            },
            _ => panic!(
                "Interpreter error: interpret() not implemented for '{self:?}'"
            ),
        }
    }
} // impl

fn interpret_program(symbols: &mut SymbolTable, body: &Vec<Expr>, env: usize) -> InterpreterResult {
    interpret_body_or_block(symbols, body, env)
}

fn interpret_output(
    symbols: &mut SymbolTable,
    data: &Vec<Expr>,
    current_scope: usize,
) -> InterpreterResult {
    for e in data {
        let r = e.interpret(symbols, current_scope)?;
        print!("{r} ");
    }
    println!();
    Ok(Expr::Unit)
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
            eprintln!("Runtime error: {err}");
            return tmp_expr_result;
        }
    }
    tmp_expr_result
}

fn interpret_let(
    symbols: &mut SymbolTable,
    _var_name: &str,
    _data_type: &DataType,
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
        Expr::DefineFunction { value, .. } => {
            // Unwrap the Lambda from DefineFunction
            match value.as_ref() {
                Expr::Lambda { value: func, environment } => {
                    // Check if this is a built-in method
                    if let Some(ref builtin) = func.builtin {
                        if args.len() != func.params.len() {
                            panic!(
                                "Interpreter error: Function {fn_name} called with wrong number of arguments."
                            );
                        }

                        // Evaluate all arguments
                        let mut evaluated_args = Vec::new();
                        let mut receiver = None;

                        for (i, a) in args.iter().enumerate() {
                            let arg_value = a.value.interpret(symbols, current_scope)?;

                            // First argument should be "self" for methods
                            if i == 0 && a.name == "self" {
                                receiver = Some(arg_value);
                            } else {
                                evaluated_args.push(arg_value);
                            }
                        }

                        if let Some(recv) = receiver {
                            // Execute built-in method
                            builtin.execute(recv, evaluated_args)
                                .map_err(|e| Box::new(RuntimeError::new(&e.to_string(), None, None)) as Box<dyn std::error::Error>)
                        } else {
                            panic!("Interpreter error: Built-in method {fn_name} called without 'self' argument");
                        }
                    } else {
                        // Regular user-defined function
                        if args.len() != func.params.len() {
                            panic!(
                                "Interpreter error: Function {fn_name} called with wrong number of arguments."
                            );
                        }

                        for a in args {
                            let arg_value = a.value.interpret(symbols, current_scope)?;

                            if let Some(assign_to_index) = symbols.get_index_in_scope(&a.name, *environment) {
                                symbols.update_runtime_value(arg_value, &(*environment, assign_to_index));
                            } else {
                                panic!("Interpreter error: Keyword arg names must match the function definition parameters.");
                            }
                        }

                        interpret_lambda(symbols, func, *environment)
                    }
                }
                _ => {
                    panic!("Interpreter error: Expected Lambda inside DefineFunction for {fn_name}");
                }
            }
        }
        Expr::Lambda { value, environment } => {
            // Check if this is a built-in method
            if let Some(ref builtin) = value.builtin {
                if args.len() != value.params.len() {
                    panic!(
                        "Interpreter error: Function {fn_name} called with wrong number of arguments."
                    );
                }

                // Evaluate all arguments
                let mut evaluated_args = Vec::new();
                let mut receiver = None;

                for (i, a) in args.iter().enumerate() {
                    let arg_value = a.value.interpret(symbols, current_scope)?;

                    // First argument should be "self" for methods
                    if i == 0 && a.name == "self" {
                        receiver = Some(arg_value);
                    } else {
                        evaluated_args.push(arg_value);
                    }
                }

                if let Some(recv) = receiver {
                    // Execute built-in method
                    builtin.execute(recv, evaluated_args)
                        .map_err(|e| Box::new(RuntimeError::new(&e.to_string(), None, None)) as Box<dyn std::error::Error>)
                } else {
                    panic!("Interpreter error: Built-in method {fn_name} called without 'self' argument");
                }
            } else {
                // Regular user-defined function
                if args.len() != value.params.len() {
                    // TODO this should be in the compile pass
                    panic!(
                        "Interpreter error: Function {fn_name} called with wrong number of arguments."
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
        }
        _ => {
            if !args.is_empty() {
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
                &format!("Symbol '{name}' not found at runtime"),
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
        panic!("Can't use expression '{cond:?}' as boolean. This is an interpreter bug. The type checker should have caught this.");
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
            (Add, Str(l), Str(r)) => {
                // Strip quotes from both strings, concatenate, then add quotes back
                let l_content = l.trim_matches('\'');
                let r_content = r.trim_matches('\'');
                LiteralData::Str(format!("'{}{}'", l_content, r_content).into())
            },
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
            (And, Bool(l), Bool(r)) => Bool(*l && *r),
            (Or, Bool(l), Bool(r)) => Bool(*l || *r),
            _ => {
                // The type checker and parser should have prevented us from
                // reaching this point.
                let msg = format!("{op:?} not allowed on {self:?},{rhs:?}");
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
    // Special handling for Range operator
    if matches!(op, Operator::Range) {
        let left_val = left.interpret(symbols, current_scope)?;
        let right_val = right.interpret(symbols, current_scope)?;
        
        match (left_val, right_val) {
            (Expr::Literal(LiteralData::Int(start)), Expr::Literal(LiteralData::Int(end))) => {
                return Ok(Expr::Range(LiteralData::Int(start), LiteralData::Int(end)));
            }
            _ => {
                return Err(RuntimeError::new(
                    "Range operator '..' requires integer operands",
                    None,
                    None,
                ).into());
            }
        }
    }
    
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
            match left.interpret(symbols, current_scope)? {
                Expr::Literal(ref l_value) | Expr::RuntimeData(ref l_value) => {
                    result = l_value.apply_binary_operator(r_value, op);
                }
                _ => {
                    let msg = format!(
                        "Result of {left:?} isn't a simple primary expression. Cannot apply {op:?} to it."
                    );
                    error = Some(RuntimeError::new(&msg, None, None));
                }
            }
        }
        (Expr::Literal(l_value), _) => {
            match right.interpret(symbols, current_scope)? {
                Expr::Literal(ref r_value) | Expr::RuntimeData(ref r_value) => {
                    result = l_value.apply_binary_operator(r_value, op);
                }
                _ => {
                    let msg = format!(
                        "Result of {right:?} isn't a simple primary expression. Cannot apply {op:?} to it."
                    );
                    error = Some(RuntimeError::new(&msg, None, None));
                }
            }
        }
        (_, _) => {
            let l_value = left.interpret(symbols, current_scope)?;
            let r_value = right.interpret(symbols, current_scope)?;
            match (&l_value, &r_value) {
                (Expr::Literal(ref l_data) | Expr::RuntimeData(ref l_data),
                 Expr::Literal(ref r_data) | Expr::RuntimeData(ref r_data)) => {
                    result = l_data.apply_binary_operator(r_data, op);
                }
                _ => {
                    let msg = format!(
                        "Expressions don't evaluate to anything applicable to a binary operator: {:?}, {:?}",
                        &left, &right
                    );
                    error = Some(RuntimeError::new(&msg, None, None));
                }
            }
        }
    }
    if let Some(e) = error {
        Err(e.into())
    } else {
        result
    }
}
