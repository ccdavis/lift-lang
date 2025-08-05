use crate::symboltable::SymbolTable;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::KeyData;
use crate::syntax::LiteralData;
use crate::syntax::Operator;

const DEBUG: bool = false;

#[derive(Clone, Debug)]
pub enum CompileErrorType {
    Structure,
    Name,
    TypeCheck,
}
impl CompileErrorType {
    pub fn name(&self) -> String {
        match self {
            CompileErrorType::TypeCheck => "Type check Error",
            CompileErrorType::Name => "Name Error",
            CompileErrorType::Structure => "Structure Error",
        }
        .to_string()
    }
}

impl CompileError {
    pub fn structure(msg: &str, location: (usize, usize)) -> Self {
        Self {
            error_type: CompileErrorType::Structure,
            location,
            msg: msg.to_string(),
        }
    }
    pub fn name(msg: &str, location: (usize, usize)) -> Self {
        Self {
            error_type: CompileErrorType::Name,
            location,
            msg: msg.to_string(),
        }
    }
    pub fn typecheck(msg: &str, location: (usize, usize)) -> Self {
        Self {
            error_type: CompileErrorType::TypeCheck,
            location,
            msg: msg.to_string(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct CompileError {
    error_type: CompileErrorType,
    location: (usize, usize),
    msg: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (line, column) = self.location;
        write!(
            f,
            "{}: {}, {}: {}",
            &self.error_type.name(),
            line,
            column,
            self.msg
        )
    }
}
impl std::error::Error for CompileError {}

// This adds symbols for the current scope and the child scopes, plus updates the index (scope id, symbol id) on the expr
pub fn add_symbols(
    e: &mut Expr,
    symbols: &mut SymbolTable,
    _current_scope_id: usize,
) -> Result<(), CompileError> {
    if DEBUG {
        println!(
            "DEBUG: adding symbols to expr '{}' at scope '{}'\n\n",
            &e, _current_scope_id
        );
    }
    match e {
        Expr::Program {
            ref mut body,
            ref mut environment,
        } => {
            // Programs use the current scope (usually 0)
            *environment = _current_scope_id;
            
            // First pass: process all type definitions
            for e in body.iter_mut() {
                if matches!(e, Expr::DefineType { .. }) {
                    add_symbols(e, symbols, _current_scope_id)?;
                }
            }
            
            // Second pass: process everything else
            for e in body.iter_mut() {
                if !matches!(e, Expr::DefineType { .. }) {
                    add_symbols(e, symbols, _current_scope_id)?;
                }
            }
        }
        Expr::DefineType {
            type_name,
            definition,
            index: _,
        } => {
            let _symbol_id = symbols.add_type(type_name, definition, _current_scope_id)?;
        }
        Expr::Output { ref mut data } => {
            for e in data {
                add_symbols(e, symbols, _current_scope_id)?;
            }
        }
        Expr::Block {
            ref mut body,
            ref mut environment,
        } => {
            let new_scope_id = symbols.create_scope(Some(_current_scope_id));
            *environment = new_scope_id;
            for e in body {
                add_symbols(e, symbols, new_scope_id)?;
            }
        }
        Expr::BinaryExpr {
            ref mut left,
            op: _,
            ref mut right,
        } => {
            add_symbols(left, symbols, _current_scope_id)?;
            add_symbols(right, symbols, _current_scope_id)?;
        }
        Expr::If {
            ref mut cond,
            ref mut then,
            ref mut final_else,
        } => {
            add_symbols(cond, symbols, _current_scope_id)?;
            add_symbols(then, symbols, _current_scope_id)?;
            add_symbols(final_else, symbols, _current_scope_id)?;
        }
        Expr::While {
            ref mut cond,
            ref mut body,
        } => {
            add_symbols(cond, symbols, _current_scope_id)?;
            add_symbols(body, symbols, _current_scope_id)?;
        }
        Expr::Call {
            ref fn_name,
            ref mut index,
            ref mut args,
        } => {
            if let Some(found_index) = symbols.find_index_reachable_from(fn_name, _current_scope_id)
            {
                if DEBUG {
                    println!("DEBUG: During semantic analysis phase found index '{},{}' for '{}' function call.",
                    found_index.0, found_index.1,fn_name 
                );
                }
                *index = found_index;
            } else {
                let msg = format!(
                    "use of undeclared or not yet declared function '{fn_name}' at scope {_current_scope_id}"
                );
                if DEBUG {
                    eprintln!("{}", &msg);
                }
                return Err(CompileError::name(&msg, (0, 0)));
            }
            for a in args {
                if let Err(ref err) = add_symbols(&mut a.value, symbols, _current_scope_id) {
                    let new_msg = format!("Error on argument '{}': {}", a.name, err.clone());
                    return Err(CompileError::structure(&new_msg, (0, 0)));
                }
            }
        }
        Expr::Lambda {
            ref mut value,
            ref mut environment,
        } => {
            // The function has its own scope as well which we should create
            let new_scope_id = symbols.create_scope(Some(_current_scope_id));
            *environment = new_scope_id;

            // Resolve function return type
            value.return_type = resolve_type(&value.return_type, symbols, _current_scope_id)?;
            
            // Add params to the new environment with their types
            for p in &mut value.params {
                // Resolve any TypeRef in parameter types
                p.data_type = resolve_type(&p.data_type, symbols, _current_scope_id)?;
                
                // Create a typed parameter representation
                let param_expr = Expr::Let {
                    var_name: p.name.clone(),
                    data_type: p.data_type.clone(),
                    value: Box::new(Expr::Unit),
                    index: (0, 0),
                };
                let new_symbol_id = symbols.add_symbol(&p.name, param_expr, new_scope_id)?;
                p.index = (new_scope_id, new_symbol_id);
            }

            add_symbols(&mut value.body, symbols, new_scope_id)?;
        }
        Expr::DefineFunction {
            ref fn_name,
            ref mut index,
            ref mut value,
        } => {
            // At first just create the symbol table entry for the function  and make the value the Unit value...
            let new_symbol_id = symbols.add_symbol(fn_name, Expr::Unit, _current_scope_id)?;
            if DEBUG {
                println!("Added symbol id {new_symbol_id} for function {fn_name}");
            }
            // Then update the body (value) with all the right symbol indices including the function itself, to
            // support recursion...
            add_symbols(value, symbols, _current_scope_id)?;
            // Now update the compile time value of the function with the correct indices for
            // all symbols.
            symbols.update_compiletime_symbol_value(
                *value.clone(),
                &(_current_scope_id, new_symbol_id),
            );

            // The function is getting defined for the current scope:
            *index = (_current_scope_id, new_symbol_id);
        }
        // Here we set the variable's index from the already added symbol and catch
        // places where the call comes before the definition.
        Expr::Variable {
            ref name,
            ref mut index,
        } => {
            if let Some(found_index) = symbols.find_index_reachable_from(name, _current_scope_id) {
                *index = found_index;
            } else {
                let msg = format!("use of undeclared or not yet declared variable '{name}'");
                return Err(CompileError::name(&msg, (0, 0)));
            }
        }

        Expr::Let {
            ref var_name,
            ref mut value,
            ref mut data_type,
            ref mut index,
        } => {
            // First, add symbols for the value expression
            add_symbols(value, symbols, _current_scope_id)?;
            
            // If no type annotation, try to infer the type
            if matches!(data_type, DataType::Unsolved) {
                if let Some(inferred_type) = determine_type_with_symbols(value, symbols, _current_scope_id) {
                    *data_type = inferred_type;
                }
            } else {
                // Resolve any TypeRef in the data_type
                *data_type = resolve_type(data_type, symbols, _current_scope_id)?;
            }
            
            // Create a typed symbol with the resolved type
            let typed_value = Expr::Let {
                var_name: var_name.clone(),
                data_type: data_type.clone(),
                value: value.clone(),
                index: (0, 0),
            };
            let new_symbol_id = symbols.add_symbol(var_name, typed_value, _current_scope_id)?;
            *index = (_current_scope_id, new_symbol_id);
        }
        Expr::Return(ref mut e) => add_symbols(e, symbols, _current_scope_id)?,

        Expr::ListLiteral { ref mut data, data_type: _ } => {
            // Add symbols for each element in the list
            for elem in data.iter_mut() {
                add_symbols(elem, symbols, _current_scope_id)?;
            }
            // Note: data_type here refers to the element type, not the list type itself
            // We don't need to change it in add_symbols - it's handled in typecheck
        }
        Expr::MapLiteral { ref mut data, key_type: _, value_type: _ } => {
            // Add symbols for each value in the map
            for (_, value) in data.iter_mut() {
                add_symbols(value, symbols, _current_scope_id)?;
            }
            // Note: key_type and value_type are handled in typecheck, not here
        }
        Expr::Range(..) => {
            // Range literals don't need symbol processing
        }
        Expr::Index { ref mut expr, ref mut index } => {
            // Add symbols for both the expression being indexed and the index itself
            add_symbols(expr, symbols, _current_scope_id)?;
            add_symbols(index, symbols, _current_scope_id)?;
        }
        Expr::UnaryExpr { ref mut expr, .. } => {
            // Process the inner expression to handle any variables
            add_symbols(expr, symbols, _current_scope_id)?;
        }
        _ => (),
    }
    Ok(())
}
// Resolve TypeRef to actual types
pub fn resolve_type(
    data_type: &DataType,
    symbols: &SymbolTable,
    scope: usize,
) -> Result<DataType, CompileError> {
    match data_type {
        DataType::TypeRef(name) => {
            match symbols.lookup_type(name, scope) {
                Some(resolved_type) => Ok(resolved_type),
                None => Err(CompileError::name(
                    &format!("Unknown type: {}", name),
                    (0, 0),
                )),
            }
        }
        DataType::List { element_type } => {
            let resolved_element = resolve_type(element_type, symbols, scope)?;
            Ok(DataType::List {
                element_type: Box::new(resolved_element),
            })
        }
        DataType::Map { key_type, value_type } => {
            let resolved_key = resolve_type(key_type, symbols, scope)?;
            let resolved_value = resolve_type(value_type, symbols, scope)?;
            Ok(DataType::Map {
                key_type: Box::new(resolved_key),
                value_type: Box::new(resolved_value),
            })
        }
        DataType::Set(element_type) => {
            let resolved_element = resolve_type(element_type, symbols, scope)?;
            Ok(DataType::Set(Box::new(resolved_element)))
        }
        DataType::Optional(inner_type) => {
            let resolved_inner = resolve_type(inner_type, symbols, scope)?;
            Ok(DataType::Optional(Box::new(resolved_inner)))
        }
        // Built-in types and others pass through unchanged
        _ => Ok(data_type.clone()),
    }
}

// Type checking functions
pub fn typecheck(
    expr: &Expr,
    symbols: &SymbolTable,
    _current_scope_id: usize,
) -> Result<DataType, CompileError> {
    match expr {
        Expr::Literal(l) => Ok(match l {
            LiteralData::Int(_) => DataType::Int,
            LiteralData::Str(_) => DataType::Str,
            LiteralData::Flt(_) => DataType::Flt,
            LiteralData::Bool(_) => DataType::Bool,
        }),
        
        Expr::RuntimeData(l) => Ok(match l {
            LiteralData::Int(_) => DataType::Int,
            LiteralData::Str(_) => DataType::Str,
            LiteralData::Flt(_) => DataType::Flt,
            LiteralData::Bool(_) => DataType::Bool,
        }),
        
        Expr::BinaryExpr { left, op, right } => {
            let left_type = typecheck(left, symbols, _current_scope_id)?;
            let right_type = typecheck(right, symbols, _current_scope_id)?;
            
            match op {
                // Arithmetic operators
                Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                    // Check for unsolved types
                    if matches!(left_type, DataType::Unsolved) || matches!(right_type, DataType::Unsolved) {
                        return Err(CompileError::typecheck(
                            "Cannot perform arithmetic on expressions with unknown types",
                            (0, 0),
                        ));
                    }
                    
                    match (&left_type, &right_type) {
                        (DataType::Int, DataType::Int) => Ok(DataType::Int),
                        (DataType::Flt, DataType::Flt) => Ok(DataType::Flt),
                        (DataType::Int, DataType::Flt) | (DataType::Flt, DataType::Int) => Ok(DataType::Flt),
                        (DataType::Str, DataType::Str) if matches!(op, Operator::Add) => Ok(DataType::Str),
                        _ => Err(CompileError::typecheck(
                            &format!("Type mismatch in binary operation {op:?}: {left_type:?} and {right_type:?}"),
                            (0, 0),
                        )),
                    }
                }
                // Comparison operators
                Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte => {
                    if matches!(left_type, DataType::Unsolved) || matches!(right_type, DataType::Unsolved) {
                        return Err(CompileError::typecheck(
                            "Cannot compare expressions with unknown types",
                            (0, 0),
                        ));
                    }
                    
                    match (&left_type, &right_type) {
                        (DataType::Int, DataType::Int) | (DataType::Flt, DataType::Flt) |
                        (DataType::Int, DataType::Flt) | (DataType::Flt, DataType::Int) => Ok(DataType::Bool),
                        _ => Err(CompileError::typecheck(
                            &format!("Cannot compare {left_type:?} and {right_type:?}"),
                            (0, 0),
                        )),
                    }
                }
                // Equality operators
                Operator::Eq | Operator::Neq => {
                    if types_compatible(&left_type, &right_type) {
                        Ok(DataType::Bool)
                    } else {
                        Err(CompileError::typecheck(
                            &format!("Cannot compare {left_type:?} and {right_type:?} for equality"),
                            (0, 0),
                        ))
                    }
                }
                // Logical operators
                Operator::And | Operator::Or => {
                    match (&left_type, &right_type) {
                        (DataType::Bool, DataType::Bool) => Ok(DataType::Bool),
                        _ => Err(CompileError::typecheck(
                            &format!("Logical operators require boolean operands, got {left_type:?} and {right_type:?}"),
                            (0, 0),
                        )),
                    }
                }
                Operator::Not => unreachable!("Not is a unary operator"),
                Operator::Range => {
                    // Range operator requires integer operands
                    if !matches!(&left_type, DataType::Int) || !matches!(&right_type, DataType::Int) {
                        return Err(CompileError::typecheck(
                            &format!("Range operator '..' requires integer operands: {left_type:?} .. {right_type:?}"),
                            (0, 0),
                        ));
                    }
                    // Return a Range type containing the range expression
                    Ok(DataType::Range(Box::new(Expr::BinaryExpr {
                        left: left.clone(),
                        op: Operator::Range,
                        right: right.clone(),
                    })))
                }
            }
        }
        
        Expr::UnaryExpr { op, expr } => {
            let expr_type = typecheck(expr, symbols, _current_scope_id)?;
            match op {
                Operator::Not => match expr_type {
                    DataType::Bool => Ok(DataType::Bool),
                    _ => Err(CompileError::typecheck(
                        &format!("Not operator requires boolean operand, got {expr_type:?}"),
                        (0, 0),
                    )),
                },
                _ => Err(CompileError::typecheck(
                    &format!("Invalid unary operator: {op:?}"),
                    (0, 0),
                )),
            }
        }
        
        Expr::If { cond, then, final_else } => {
            let cond_type = typecheck(cond, symbols, _current_scope_id)?;
            if !matches!(cond_type, DataType::Bool) {
                return Err(CompileError::typecheck(
                    &format!("If condition must be boolean, got {cond_type:?}"),
                    (0, 0),
                ));
            }
            
            let then_type = typecheck(then, symbols, _current_scope_id)?;
            let else_type = typecheck(final_else, symbols, _current_scope_id)?;
            
            if types_compatible(&then_type, &else_type) {
                Ok(then_type)
            } else {
                Err(CompileError::typecheck(
                    &format!("If branches must have compatible types, got {then_type:?} and {else_type:?}"),
                    (0, 0),
                ))
            }
        }
        
        Expr::While { cond, body } => {
            let cond_type = typecheck(cond, symbols, _current_scope_id)?;
            if !matches!(cond_type, DataType::Bool) {
                return Err(CompileError::typecheck(
                    &format!("While condition must be boolean, got {cond_type:?}"),
                    (0, 0),
                ));
            }
            typecheck(body, symbols, _current_scope_id)?;
            Ok(DataType::Unsolved) // While loops don't return a meaningful value
        }
        
        Expr::Variable { name, index } => {
            // Variables must have been defined before use
            if let Some(var_type) = symbols.get_symbol_type(index) {
                match var_type {
                    DataType::Unsolved => Err(CompileError::typecheck(
                        &format!("Cannot determine type of variable: {name}. Consider adding a type annotation."),
                        (0, 0),
                    )),
                    _ => Ok(var_type),
                }
            } else {
                Err(CompileError::name(
                    &format!("Undefined variable: {name}"),
                    (0, 0),
                ))
            }
        }
        
        Expr::Let { var_name, value, data_type, index: _ } => {
            // If a type is specified, use it and check the value is compatible
            if !matches!(data_type, DataType::Unsolved) {
                // Resolve any TypeRef in the data_type
                let resolved_type = resolve_type(data_type, symbols, _current_scope_id)?;
                
                // Special handling for empty collections with type annotations
                match value.as_ref() {
                    Expr::ListLiteral { data, .. } if data.is_empty() => {
                        // Empty list with type annotation is valid
                        return Ok(resolved_type);
                    }
                    Expr::MapLiteral { data, .. } if data.is_empty() => {
                        // Empty map with type annotation is valid
                        return Ok(resolved_type);
                    }
                    _ => {
                        // For non-empty values, check type compatibility
                        let value_type = typecheck(value, symbols, _current_scope_id)?;
                        if !types_compatible(&resolved_type, &value_type) {
                            return Err(CompileError::typecheck(
                                &format!("Type annotation mismatch for {var_name}: expected {:?}, got {value_type:?}", resolved_type),
                                (0, 0),
                            ));
                        }
                    }
                }
                Ok(resolved_type)
            } else {
                // No type annotation, must infer from value
                let value_type = typecheck(value, symbols, _current_scope_id)?;
                
                // Check if value type is fully resolved
                if matches!(value_type, DataType::Unsolved) {
                    return Err(CompileError::typecheck(
                        &format!("Cannot infer type for '{var_name}'. Please provide a type annotation."),
                        (0, 0),
                    ));
                }
                
                Ok(value_type)
            }
        }
        
        Expr::Assign { name, value, index } => {
            let value_type = typecheck(value, symbols, _current_scope_id)?;
            if let Some(var_type) = symbols.get_symbol_type(index) {
                if types_compatible(&var_type, &value_type) {
                    Ok(value_type)
                } else {
                    Err(CompileError::typecheck(
                        &format!("Cannot assign {value_type:?} to variable {name} of type {var_type:?}"),
                        (0, 0),
                    ))
                }
            } else {
                Err(CompileError::name(
                    &format!("Undefined variable: {name}"),
                    (0, 0),
                ))
            }
        }
        
        Expr::ListLiteral { data_type, data } => {
            if data.is_empty() {
                // Empty lists need explicit type annotation
                if matches!(data_type, DataType::Unsolved) {
                    return Err(CompileError::typecheck(
                        "Cannot infer type for empty list. Please provide a type annotation.",
                        (0, 0),
                    ));
                }
                return Ok(DataType::List {
                    element_type: Box::new(data_type.clone()),
                });
            }
            
            // Check all elements have compatible types
            let mut element_types = Vec::new();
            for elem in data {
                element_types.push(typecheck(elem, symbols, _current_scope_id)?);
            }
            
            let first_type = &element_types[0];
            for elem_type in &element_types[1..] {
                if !types_compatible(first_type, elem_type) {
                    return Err(CompileError::typecheck(
                        &format!("List elements must have compatible types, found {first_type:?} and {elem_type:?}"),
                        (0, 0),
                    ));
                }
            }
            
            Ok(DataType::List {
                element_type: Box::new(first_type.clone()),
            })
        }
        
        Expr::RuntimeList { data_type, data: _ } => {
            Ok(DataType::List {
                element_type: Box::new(data_type.clone()),
            })
        }
        
        Expr::MapLiteral { key_type, value_type, data } => {
            // Infer key type if not specified
            let actual_key_type = if !matches!(key_type, DataType::Unsolved) {
                key_type.clone()
            } else if !data.is_empty() {
                // Determine key type from first entry
                match &data[0].0 {
                    KeyData::Int(_) => DataType::Int,
                    KeyData::Str(_) => DataType::Str,
                    KeyData::Bool(_) => DataType::Bool,
                }
            } else {
                return Err(CompileError::typecheck(
                    "Cannot infer key type for empty map. Please provide a type annotation.",
                    (0, 0),
                ));
            };
            
            // Infer value type if not specified
            let actual_value_type = if !matches!(value_type, DataType::Unsolved) {
                value_type.clone()
            } else if let Some((_, first_value)) = data.first() {
                typecheck(first_value, symbols, _current_scope_id)?
            } else {
                return Err(CompileError::typecheck(
                    "Cannot infer value type for empty map. Please provide a type annotation.",
                    (0, 0),
                ));
            };
            
            // Check all values match the inferred type
            for (_key, value) in data {
                let value_type_actual = typecheck(value, symbols, _current_scope_id)?;
                if !types_compatible(&actual_value_type, &value_type_actual) {
                    return Err(CompileError::typecheck(
                        &format!("Map value type mismatch: expected {:?}, got {:?}", actual_value_type, value_type_actual),
                        (0, 0),
                    ));
                }
            }
            
            Ok(DataType::Map {
                key_type: Box::new(actual_key_type),
                value_type: Box::new(actual_value_type),
            })
        }
        
        Expr::RuntimeMap { key_type, value_type, .. } => {
            Ok(DataType::Map {
                key_type: Box::new(key_type.clone()),
                value_type: Box::new(value_type.clone()),
            })
        }
        
        Expr::Range(start, end) => {
            // Range literals are always integers
            match (start, end) {
                (LiteralData::Int(_), LiteralData::Int(_)) => Ok(DataType::Range(Box::new(Expr::Range(start.clone(), end.clone())))),
                _ => Err(CompileError::typecheck(
                    "Range literals must have integer bounds",
                    (0, 0),
                )),
            }
        }
        
        Expr::Call { fn_name, index, args } => {
            // Get function type from symbol table
            if let Some(fn_expr) = symbols.get_symbol_value(index) {
                match fn_expr {
                    Expr::Lambda { value: func, .. } => {
                        // Check argument types
                        if args.len() != func.params.len() {
                            return Err(CompileError::typecheck(
                                &format!("Function {} expects {} arguments, got {}", fn_name, func.params.len(), args.len()),
                                (0, 0),
                            ));
                        }
                        
                        for (i, arg) in args.iter().enumerate() {
                            let arg_type = typecheck(&arg.value, symbols, _current_scope_id)?;
                            let expected_type = &func.params[i].data_type;
                            if !matches!(expected_type, DataType::Unsolved) && !types_compatible(expected_type, &arg_type) {
                                return Err(CompileError::typecheck(
                                    &format!("Argument {} type mismatch: expected {:?}, got {:?}", i+1, expected_type, arg_type),
                                    (0, 0),
                                ));
                            }
                        }
                        
                        Ok(func.return_type.clone())
                    }
                    _ => Err(CompileError::typecheck(
                        &format!("{fn_name} is not a function"),
                        (0, 0),
                    )),
                }
            } else {
                Err(CompileError::name(
                    &format!("Undefined function: {fn_name}"),
                    (0, 0),
                ))
            }
        }
        
        Expr::Lambda { value: func, .. } => {
            // Type check the body in a new scope
            let body_type = typecheck(&func.body, symbols, _current_scope_id)?;
            
            // Check return type matches if specified
            if !matches!(func.return_type, DataType::Unsolved) && !types_compatible(&func.return_type, &body_type) {
                return Err(CompileError::typecheck(
                    &format!("Function body returns {body_type:?} but return type is {:?}", func.return_type),
                    (0, 0),
                ));
            }
            
            Ok(DataType::Unsolved) // Lambda expressions themselves don't have a simple type representation yet
        }
        
        Expr::DefineFunction { value, .. } => {
            typecheck(value, symbols, _current_scope_id)?;
            Ok(DataType::Unsolved) // Function definitions don't return a value
        }
        
        Expr::DefineType { .. } => Ok(DataType::Unsolved),
        
        Expr::Block { body, environment } => {
            // Type check all expressions in the block
            if body.is_empty() {
                Ok(DataType::Unsolved)
            } else {
                // Type check all expressions
                for expr in &body[..body.len()-1] {
                    typecheck(expr, symbols, *environment)?;
                }
                // Return the type of the last expression
                typecheck(&body[body.len()-1], symbols, *environment)
            }
        }
        
        Expr::Program { body, environment } => {
            // Type check all expressions in the program
            for expr in body {
                typecheck(expr, symbols, *environment)?;
            }
            Ok(DataType::Unsolved)
        }
        
        Expr::Output { data } => {
            // Type check all output expressions
            for expr in data {
                typecheck(expr, symbols, _current_scope_id)?;
            }
            Ok(DataType::Unsolved)
        }
        
        Expr::Return(expr) => typecheck(expr, symbols, _current_scope_id),
        
        Expr::Match { .. } => Ok(DataType::Unsolved), // TODO: Implement match type checking
        
        Expr::Unit => Ok(DataType::Unsolved), // Unit type could be a specific type in the future
        
        Expr::Index { expr, index } => {
            let expr_type = typecheck(expr, symbols, _current_scope_id)?;
            let index_type = typecheck(index, symbols, _current_scope_id)?;
            
            // Handle different collection types
            match expr_type {
                DataType::List { element_type } => {
                    // For lists, index must be Int
                    if index_type != DataType::Int {
                        return Err(CompileError::typecheck(
                            &format!("List index must be of type Int, found {:?}", index_type),
                            (0, 0),
                        ));
                    }
                    Ok(*element_type)
                },
                DataType::Map { key_type, value_type } => {
                    // For maps, index must match key type
                    if index_type != *key_type {
                        return Err(CompileError::typecheck(
                            &format!("Map key must be of type {:?}, found {:?}", key_type, index_type),
                            (0, 0),
                        ));
                    }
                    // Maps can only have Int, Str, or Bool keys (not Float)
                    match &index_type {
                        DataType::Int | DataType::Str | DataType::Bool => Ok(*value_type),
                        DataType::Flt => Err(CompileError::typecheck(
                            "Map keys cannot be of type Flt",
                            (0, 0),
                        )),
                        _ => Err(CompileError::typecheck(
                            &format!("Invalid map key type: {:?}", index_type),
                            (0, 0),
                        ))
                    }
                },
                _ => Err(CompileError::typecheck(
                    &format!("Cannot index into type {:?}, only List and Map types can be indexed", expr_type),
                    (0, 0),
                ))
            }
        }
    }
}

// Helper function to check if two types are compatible
fn types_compatible(t1: &DataType, t2: &DataType) -> bool {
    match (t1, t2) {
        (DataType::Unsolved, _) | (_, DataType::Unsolved) => true,
        (DataType::Int, DataType::Int) => true,
        (DataType::Flt, DataType::Flt) => true,
        (DataType::Str, DataType::Str) => true,
        (DataType::Bool, DataType::Bool) => true,
        (DataType::Int, DataType::Flt) | (DataType::Flt, DataType::Int) => true, // Numeric compatibility
        (DataType::List { element_type: e1 }, DataType::List { element_type: e2 }) => {
            types_compatible(e1, e2)
        }
        (DataType::Map { key_type: k1, value_type: v1 }, DataType::Map { key_type: k2, value_type: v2 }) => {
            types_compatible(k1, k2) && types_compatible(v1, v2)
        }
        (DataType::Range(_), DataType::Range(_)) => {
            // For now, all ranges are compatible with each other
            // In the future we might want to check the bounds
            true
        }
        _ => false,
    }
}

// Type inference with symbol table lookup
pub fn determine_type_with_symbols(
    expression: &Expr,
    symbols: &SymbolTable,
    scope: usize,
) -> Option<DataType> {
    match expression {
        Expr::Literal(l) => Some(match l {
            LiteralData::Int(_) => DataType::Int,
            LiteralData::Str(_) => DataType::Str,
            LiteralData::Flt(_) => DataType::Flt,
            LiteralData::Bool(_) => DataType::Bool,
        }),
        
        Expr::Variable { index, .. } => {
            // Look up the variable type from the symbol table
            symbols.get_symbol_type(index)
        }
        
        Expr::BinaryExpr { left, op, right } => {
            let left_type = determine_type_with_symbols(left, symbols, scope)?;
            let right_type = determine_type_with_symbols(right, symbols, scope)?;
            
            match op {
                Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                    match (&left_type, &right_type) {
                        (DataType::Int, DataType::Int) => Some(DataType::Int),
                        (DataType::Flt, DataType::Flt) => Some(DataType::Flt),
                        (DataType::Int, DataType::Flt) | (DataType::Flt, DataType::Int) => Some(DataType::Flt),
                        (DataType::Str, DataType::Str) if matches!(op, Operator::Add) => Some(DataType::Str),
                        _ => None,
                    }
                }
                Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte |
                Operator::Eq | Operator::Neq => Some(DataType::Bool),
                Operator::And | Operator::Or => Some(DataType::Bool),
                Operator::Not => unreachable!("Not is unary"),
                Operator::Range => {
                    match (&left_type, &right_type) {
                        (DataType::Int, DataType::Int) => Some(DataType::Range(Box::new(expression.clone()))),
                        _ => None,
                    }
                }
            }
        }
        
        Expr::Call { index, .. } => {
            // Look up function return type
            if let Some(fn_expr) = symbols.get_symbol_value(index) {
                match fn_expr {
                    Expr::Lambda { value: func, .. } => Some(func.return_type.clone()),
                    _ => None,
                }
            } else {
                None
            }
        }
        
        Expr::Index { expr, .. } => {
            match determine_type_with_symbols(expr, symbols, scope)? {
                DataType::List { element_type } => Some(*element_type),
                DataType::Map { value_type, .. } => Some(*value_type),
                _ => None,
            }
        }
        
        _ => determine_type(expression), // Fall back to original for other cases
    }
}

// Type inference for expressions
pub fn determine_type(expression: &Expr) -> Option<DataType> {
    match expression {
        Expr::Literal(l) => Some(match l {
            LiteralData::Int(_) => DataType::Int,
            LiteralData::Str(_) => DataType::Str,
            LiteralData::Flt(_) => DataType::Flt,
            LiteralData::Bool(_) => DataType::Bool,
        }),
        
        Expr::RuntimeData(l) => Some(match l {
            LiteralData::Int(_) => DataType::Int,
            LiteralData::Str(_) => DataType::Str,
            LiteralData::Flt(_) => DataType::Flt,
            LiteralData::Bool(_) => DataType::Bool,
        }),
        
        Expr::BinaryExpr { left, op, right } => {
            let left_type = determine_type(left)?;
            let right_type = determine_type(right)?;
            
            match op {
                Operator::Add | Operator::Sub | Operator::Mul | Operator::Div => {
                    match (&left_type, &right_type) {
                        (DataType::Int, DataType::Int) => Some(DataType::Int),
                        (DataType::Flt, DataType::Flt) => Some(DataType::Flt),
                        (DataType::Int, DataType::Flt) | (DataType::Flt, DataType::Int) => Some(DataType::Flt),
                        (DataType::Str, DataType::Str) if matches!(op, Operator::Add) => Some(DataType::Str),
                        _ => None,
                    }
                }
                Operator::Gt | Operator::Lt | Operator::Gte | Operator::Lte |
                Operator::Eq | Operator::Neq => Some(DataType::Bool),
                Operator::And | Operator::Or => Some(DataType::Bool),
                Operator::Not => unreachable!("Not is unary"),
                Operator::Range => {
                    // Range operator produces a Range type
                    match (&left_type, &right_type) {
                        (DataType::Int, DataType::Int) => Some(DataType::Range(Box::new(Expr::BinaryExpr {
                            left: left.clone(),
                            op: Operator::Range,
                            right: right.clone(),
                        }))),
                        _ => None,
                    }
                }
            }
        }
        
        Expr::UnaryExpr { op: Operator::Not, .. } => Some(DataType::Bool),
        Expr::UnaryExpr { .. } => None,
        
        Expr::ListLiteral { data_type, data } => {
            let element_type = if !matches!(data_type, DataType::Unsolved) {
                data_type.clone()
            } else if let Some(first) = data.first() {
                determine_type(first)?
            } else {
                return None;
            };
            
            Some(DataType::List {
                element_type: Box::new(element_type),
            })
        }
        
        Expr::If { then, final_else, .. } => {
            // If expression type is the type of its branches
            let then_type = determine_type(then)?;
            let else_type = determine_type(final_else)?;
            if types_compatible(&then_type, &else_type) {
                Some(then_type)
            } else {
                None
            }
        }
        
        Expr::Block { body, .. } => {
            // Block type is the type of its last expression
            if let Some(last_expr) = body.last() {
                determine_type(last_expr)
            } else {
                None
            }
        }
        
        Expr::MapLiteral { key_type, value_type, data } => {
            let actual_key_type = if !matches!(key_type, DataType::Unsolved) {
                key_type.clone()
            } else if !data.is_empty() {
                // Determine key type from first entry
                match &data[0].0 {
                    KeyData::Int(_) => DataType::Int,
                    KeyData::Str(_) => DataType::Str,
                    KeyData::Bool(_) => DataType::Bool,
                }
            } else {
                return None;
            };
            
            let actual_value_type = if !matches!(value_type, DataType::Unsolved) {
                value_type.clone()
            } else if let Some((_, first_value)) = data.first() {
                determine_type(first_value)?
            } else {
                return None;
            };
            
            Some(DataType::Map {
                key_type: Box::new(actual_key_type),
                value_type: Box::new(actual_value_type),
            })
        }
        
        Expr::Range(start, end) => {
            match (start, end) {
                (LiteralData::Int(_), LiteralData::Int(_)) => Some(DataType::Range(Box::new(Expr::Range(start.clone(), end.clone())))),
                _ => None,
            }
        }
        
        Expr::Index { expr, .. } => {
            match determine_type(expr)? {
                DataType::List { element_type } => Some(*element_type),
                DataType::Map { value_type, .. } => Some(*value_type),
                _ => None,
            }
        }
        
        _ => None,
    }
}
