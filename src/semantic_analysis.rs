use crate::symboltable::SymbolTable;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::KeyData;
use crate::syntax::LiteralData;
use crate::syntax::Operator;
use crate::syntax::Param;

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
            // Try to find the function directly
            let found_directly = symbols.find_index_reachable_from(fn_name, _current_scope_id);

            if let Some(found_index) = found_directly {
                if DEBUG {
                    println!("DEBUG: During semantic analysis phase found index '{},{}' for '{}' function call.",
                    found_index.0, found_index.1,fn_name
                );
                }
                *index = found_index;
            } else {
                // UFCS: Try to find it as a method Type.fn_name
                // Process first argument to determine type
                let mut ufcs_found = false;
                if let Some(first_arg) = args.first_mut() {
                    // Process the first argument to enable type determination
                    if let Err(ref err) = add_symbols(&mut first_arg.value, symbols, _current_scope_id) {
                        let new_msg = format!("Error on argument '{}': {}", first_arg.name, err.clone());
                        return Err(CompileError::structure(&new_msg, (0, 0)));
                    }

                    if let Some(arg_type) = determine_type_with_symbols(&first_arg.value, symbols, _current_scope_id) {
                        let type_name = match arg_type {
                            DataType::Str => "Str",
                            DataType::Int => "Int",
                            DataType::Flt => "Flt",
                            DataType::Bool => "Bool",
                            DataType::List { .. } => "List",
                            DataType::Map { .. } => "Map",
                            DataType::Range(_) => "Range",
                            _ => "",
                        };

                        if !type_name.is_empty() {
                            let method_name = format!("{}.{}", type_name, fn_name);

                            // Check if it's a built-in method
                            let is_builtin = matches!(fn_name.as_str(), "upper" | "lower" | "first" | "last");

                            if let Some(found_index) = symbols.find_index_reachable_from(&method_name, _current_scope_id) {
                                if DEBUG {
                                    println!("DEBUG: UFCS - found method '{}' for call '{}'", method_name, fn_name);
                                }
                                *index = found_index;
                                ufcs_found = true;
                            } else if is_builtin {
                                // Built-in methods don't need to be in symbol table
                                *index = (0, 0);
                                ufcs_found = true;
                            }
                        }
                    }
                }

                if !ufcs_found {
                    // If still not found, error
                    let msg = format!(
                        "use of undeclared or not yet declared function '{fn_name}' at scope {_current_scope_id}"
                    );
                    if DEBUG {
                        eprintln!("{}", &msg);
                    }
                    return Err(CompileError::name(&msg, (0, 0)));
                }
            }

            // Process remaining arguments
            for a in args {
                if let Err(ref err) = add_symbols(&mut a.value, symbols, _current_scope_id) {
                    let new_msg = format!("Error on argument '{}': {}", a.name, err.clone());
                    return Err(CompileError::structure(&new_msg, (0, 0)));
                }
            }
        }
        Expr::MethodCall {
            ref mut receiver,
            ref method_name,
            ref mut fn_index,
            ref mut args,
        } => {
            // Process the receiver expression
            add_symbols(receiver, symbols, _current_scope_id)?;

            if DEBUG {
                println!("DEBUG: MethodCall - receiver after add_symbols: {:?}", receiver);
            }

            // Determine the receiver's type to find the right method
            let receiver_type = determine_type_with_symbols(receiver, symbols, _current_scope_id)
                .ok_or_else(|| {
                    if DEBUG {
                        println!("DEBUG: Failed to determine receiver type for: {:?}", receiver);
                    }
                    CompileError::typecheck("Cannot determine receiver type for method call", (0, 0))
                })?;

            if DEBUG {
                println!("DEBUG: MethodCall - receiver_type: {:?}", receiver_type);
            }

            // Build the full method name: TypeName.methodName
            // First, try with the type alias name if it's a TypeRef
            let mut method_found = false;

            // Clone receiver_type for later use in error messages
            let receiver_type_clone = receiver_type.clone();

            if let DataType::TypeRef(alias_name) = &receiver_type {
                // Try to find method on the type alias itself
                let full_method_name = format!("{}.{}", alias_name, method_name);

                if DEBUG {
                    println!("DEBUG: Looking for method '{}' on type alias", full_method_name);
                }

                if let Some(found_index) = symbols.find_index_reachable_from(&full_method_name, _current_scope_id) {
                    if DEBUG {
                        println!("DEBUG: Found method '{}' at index {:?}", full_method_name, found_index);
                    }
                    *fn_index = found_index;
                    method_found = true;
                }
            }

            // If not found on type alias, resolve to underlying type and try again
            if !method_found {
                // Resolve TypeRef to underlying type if needed
                let resolved_type = if matches!(&receiver_type, DataType::TypeRef(_)) {
                    resolve_type(&receiver_type, symbols, _current_scope_id)?
                } else {
                    receiver_type
                };

                let type_name = match resolved_type {
                    DataType::Str => "Str",
                    DataType::Int => "Int",
                    DataType::Flt => "Flt",
                    DataType::Bool => "Bool",
                    DataType::List { .. } => "List",
                    DataType::Map { .. } => "Map",
                    DataType::Range(_) => "Range",
                    _ => return Err(CompileError::typecheck(&format!("Cannot call methods on type {:?}", resolved_type), (0, 0))),
                };

                let full_method_name = format!("{}.{}", type_name, method_name);

                if DEBUG {
                    println!("DEBUG: Looking for method '{}'", full_method_name);
                }

                // Look up the method
                if let Some(found_index) = symbols.find_index_reachable_from(&full_method_name, _current_scope_id) {
                    if DEBUG {
                        println!("DEBUG: Found method '{}' at index {:?}", full_method_name, found_index);
                    }
                    *fn_index = found_index;
                    method_found = true;
                }
            }

            if !method_found {
                let type_display = match &receiver_type_clone {
                    DataType::TypeRef(name) => name.as_str(),
                    DataType::Str => "Str",
                    DataType::Int => "Int",
                    DataType::Flt => "Flt",
                    DataType::Bool => "Bool",
                    DataType::List { .. } => "List",
                    DataType::Map { .. } => "Map",
                    DataType::Range(_) => "Range",
                    _ => "Unknown",
                };

                return Err(CompileError::name(
                    &format!("Method '{}' not found for type '{}'", method_name, type_display),
                    (0, 0)
                ));
            }

            // Process arguments
            for a in args {
                add_symbols(&mut a.value, symbols, _current_scope_id)?;
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

            // For methods, auto-inject 'self' parameter at the beginning
            if let Some(ref receiver_type_name) = value.receiver_type {
                let self_type = match receiver_type_name.as_str() {
                    "Str" => DataType::Str,
                    "Int" => DataType::Int,
                    "Flt" => DataType::Flt,
                    "Bool" => DataType::Bool,
                    "List" => return Err(CompileError::typecheck("Method on generic List requires type parameter", (0, 0))),
                    "Map" => return Err(CompileError::typecheck("Method on generic Map requires type parameter", (0, 0))),
                    custom => DataType::TypeRef(custom.to_string()),
                };

                // Create self parameter
                let self_param = Param {
                    name: "self".to_string(),
                    data_type: self_type.clone(),
                    default: None,
                    index: (0, 0),
                    copy: false, // self is immutable by default
                };

                // Add self as a symbol in the function scope
                let self_expr = Expr::Let {
                    var_name: "self".to_string(),
                    data_type: self_type,
                    value: Box::new(Expr::Unit),
                    index: (0, 0),
                    mutable: false,
                };
                let self_symbol_id = symbols.add_symbol("self", self_expr, new_scope_id)?;

                // Update self param index and insert at beginning
                let mut self_param_with_index = self_param;
                self_param_with_index.index = (new_scope_id, self_symbol_id);
                value.params.insert(0, self_param_with_index);
            }

            // Add params to the new environment with their types
            for p in &mut value.params {
                // Skip self if it was already added
                if p.name == "self" && value.receiver_type.is_some() {
                    continue;
                }
                // Validate that TypeRef exists (but don't resolve it - keep the alias)
                // This preserves the association between methods and type aliases
                if matches!(&p.data_type, DataType::TypeRef(_)) {
                    resolve_type(&p.data_type, symbols, _current_scope_id)?;
                    // Note: We validate but don't store the resolved type
                }

                // Create a typed parameter representation
                // Parameters are immutable by default (pass by reference)
                // Only 'cpy' parameters are mutable (pass by value/copy)
                let param_expr = Expr::Let {
                    var_name: p.name.clone(),
                    data_type: p.data_type.clone(),
                    value: Box::new(Expr::Unit),
                    index: (0, 0),
                    mutable: p.copy,  // Only mutable if marked with 'cpy'
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
            ref mutable,
        } => {
            // First, add symbols for the value expression
            add_symbols(value, symbols, _current_scope_id)?;
            
            // If no type annotation, try to infer the type
            if matches!(data_type, DataType::Unsolved) {
                if let Some(inferred_type) = determine_type_with_symbols(value, symbols, _current_scope_id) {
                    *data_type = inferred_type;
                }
            }
            // Note: We keep TypeRef as-is for variables so that methods on type aliases work
            // Type compatibility checking is done during typecheck phase
            
            // Create a typed symbol with the resolved type
            let typed_value = Expr::Let {
                var_name: var_name.clone(),
                data_type: data_type.clone(),
                value: value.clone(),
                index: (0, 0),
                mutable: *mutable,
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
        Expr::Len { ref mut expr } => {
            // Process the inner expression to handle any variables
            add_symbols(expr, symbols, _current_scope_id)?;
        }
        Expr::Assign { ref name, ref mut value, ref mut index } => {
            // Process the value expression first
            add_symbols(value, symbols, _current_scope_id)?;

            // Find the variable in current or parent scopes
            if let Some(found_index) = symbols.find_index_reachable_from(name, _current_scope_id) {
                *index = found_index;
            } else {
                return Err(CompileError::name(
                    &format!("Cannot assign to undeclared variable '{}'", name),
                    (0, 0)
                ));
            }
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

            // Resolve type aliases to underlying types for comparison
            let left_type = if matches!(left_type, DataType::TypeRef(_)) {
                resolve_type(&left_type, symbols, _current_scope_id)?
            } else {
                left_type
            };
            let right_type = if matches!(right_type, DataType::TypeRef(_)) {
                resolve_type(&right_type, symbols, _current_scope_id)?
            } else {
                right_type
            };

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

            // Resolve TypeRefs for comparison
            let resolved_then = if matches!(&then_type, DataType::TypeRef(_)) {
                resolve_type(&then_type, symbols, _current_scope_id)?
            } else {
                then_type.clone()
            };
            let resolved_else = if matches!(&else_type, DataType::TypeRef(_)) {
                resolve_type(&else_type, symbols, _current_scope_id)?
            } else {
                else_type.clone()
            };

            if types_compatible(&resolved_then, &resolved_else) {
                // Return the then_type (which may be a TypeRef) for better error messages
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
        
        Expr::Let { var_name, value, data_type, index: _, mutable: _ } => {
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
                        if !types_compatible_with_resolution(&resolved_type, &value_type, symbols, _current_scope_id)? {
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
                // First try to infer using determine_type_with_symbols which handles special cases
                if let Some(_inferred_type) = determine_type_with_symbols(value, symbols, _current_scope_id) {
                    // Then verify with full typecheck
                    let value_type = typecheck(value, symbols, _current_scope_id)?;
                    if !matches!(value_type, DataType::Unsolved) {
                        Ok(value_type)
                    } else {
                        Err(CompileError::typecheck(
                            &format!("Cannot infer type for '{var_name}'. Please provide a type annotation."),
                            (0, 0),
                        ))
                    }
                } else {
                    // determine_type_with_symbols returned None, which means type cannot be inferred
                    // This handles cases like if without else
                    Err(CompileError::typecheck(
                        &format!("Cannot infer type for '{var_name}'. Please provide a type annotation."),
                        (0, 0),
                    ))
                }
            }
        }
        
        Expr::Assign { name, value, index } => {
            // Check if the variable is mutable
            if !symbols.is_mutable(index) {
                return Err(CompileError::typecheck(
                    &format!("Cannot assign to immutable variable '{}'. Use 'let var' to declare mutable variables.", name),
                    (0, 0),
                ));
            }

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
                // Extract the function from either DefineFunction or Lambda
                let func = match fn_expr {
                    Expr::DefineFunction { value, .. } => {
                        match value.as_ref() {
                            Expr::Lambda { value: f, .. } => f,
                            _ => return Err(CompileError::typecheck(
                                &format!("{fn_name} is not a function"),
                                (0, 0),
                            ))
                        }
                    }
                    Expr::Lambda { value: f, .. } => f,
                    _ => return Err(CompileError::typecheck(
                        &format!("{fn_name} is not a function"),
                        (0, 0),
                    ))
                };

                // Check if this is a UFCS call (method called with function syntax)
                // If the function has a receiver_type, it's a method
                let is_ufcs_call = func.receiver_type.is_some();

                let expected_arg_count = func.params.len();
                if args.len() != expected_arg_count {
                    return Err(CompileError::typecheck(
                        &format!("Function {} expects {} arguments, got {}", fn_name, expected_arg_count, args.len()),
                        (0, 0),
                    ));
                }

                for (i, arg) in args.iter().enumerate() {
                    let arg_type = typecheck(&arg.value, symbols, _current_scope_id)?;
                    let expected_type = &func.params[i].data_type;
                    if !matches!(expected_type, DataType::Unsolved) {
                        // Resolve TypeRefs for proper compatibility checking
                        let resolved_expected = if matches!(expected_type, DataType::TypeRef(_)) {
                            resolve_type(expected_type, symbols, _current_scope_id)?
                        } else {
                            expected_type.clone()
                        };

                        let resolved_arg = if matches!(&arg_type, DataType::TypeRef(_)) {
                            resolve_type(&arg_type, symbols, _current_scope_id)?
                        } else {
                            arg_type.clone()
                        };

                        if !types_compatible(&resolved_expected, &resolved_arg) {
                            // Special message for UFCS where first arg is self
                            let arg_num_display = if is_ufcs_call && i == 0 {
                                "receiver (self)".to_string()
                            } else if is_ufcs_call {
                                format!("argument {}", i)
                            } else {
                                format!("argument {}", i + 1)
                            };
                            return Err(CompileError::typecheck(
                                &format!("{} type mismatch: expected {:?}, got {:?}", arg_num_display, expected_type, arg_type),
                                (0, 0),
                            ));
                        }
                    }
                }

                Ok(func.return_type.clone())
            } else {
                Err(CompileError::name(
                    &format!("Undefined function: {fn_name}"),
                    (0, 0),
                ))
            }
        }

        Expr::MethodCall { receiver, method_name, fn_index, args } => {
            if DEBUG {
                println!("DEBUG: typecheck MethodCall - receiver: {:?}", receiver);
            }
            // Type check the receiver
            let receiver_type = typecheck(receiver, symbols, _current_scope_id)?;
            if DEBUG {
                println!("DEBUG: typecheck MethodCall - receiver_type: {:?}", receiver_type);
            }

            // Resolve TypeRef for type checking (keep original for error messages)
            let resolved_receiver_type = if matches!(&receiver_type, DataType::TypeRef(_)) {
                resolve_type(&receiver_type, symbols, _current_scope_id)?
            } else {
                receiver_type.clone()
            };

            // Check if it's a built-in method
            let is_builtin = matches!(method_name.as_str(), "upper" | "lower" | "first" | "last");

            if is_builtin {
                // Type check built-in methods using resolved type
                match method_name.as_str() {
                    "upper" | "lower" => {
                        if !matches!(resolved_receiver_type, DataType::Str) {
                            return Err(CompileError::typecheck(
                                &format!("{} can only be called on Str, got {:?}", method_name, receiver_type),
                                (0, 0),
                            ));
                        }
                        if !args.is_empty() {
                            return Err(CompileError::typecheck(
                                &format!("{} expects no arguments, got {}", method_name, args.len()),
                                (0, 0),
                            ));
                        }
                        Ok(DataType::Str)
                    }
                    "first" | "last" => {
                        match resolved_receiver_type {
                            DataType::List { element_type } => {
                                if !args.is_empty() {
                                    return Err(CompileError::typecheck(
                                        &format!("{} expects no arguments, got {}", method_name, args.len()),
                                        (0, 0),
                                    ));
                                }
                                Ok(*element_type)
                            }
                            _ => Err(CompileError::typecheck(
                                &format!("{} can only be called on List, got {:?}", method_name, receiver_type),
                                (0, 0),
                            ))
                        }
                    }
                    _ => Ok(DataType::Unsolved)
                }
            } else if let Some(fn_expr) = symbols.get_symbol_value(fn_index) {
                // Extract the function from either DefineFunction or Lambda
                let func = match fn_expr {
                    Expr::DefineFunction { value, .. } => {
                        match value.as_ref() {
                            Expr::Lambda { value: f, .. } => f,
                            _ => return Err(CompileError::typecheck(
                                &format!("{method_name} is not a method"),
                                (0, 0),
                            ))
                        }
                    }
                    Expr::Lambda { value: f, .. } => f,
                    _ => return Err(CompileError::typecheck(
                        &format!("{method_name} is not a method"),
                        (0, 0),
                    ))
                };

                // For methods, first param is implicit self
                // Check that we have the right number of explicit args (params - 1 for self)
                let expected_arg_count = if func.receiver_type.is_some() {
                    func.params.len().saturating_sub(1)
                } else {
                    func.params.len()
                };

                if args.len() != expected_arg_count {
                    return Err(CompileError::typecheck(
                        &format!("Method {} expects {} arguments, got {}", method_name, expected_arg_count, args.len()),
                        (0, 0),
                    ));
                }

                // Check receiver type matches self parameter
                if func.receiver_type.is_some() && !func.params.is_empty() {
                    let self_param_type = &func.params[0].data_type;

                    // Resolve both types for comparison
                    let resolved_self_type = if matches!(self_param_type, DataType::TypeRef(_)) {
                        resolve_type(self_param_type, symbols, _current_scope_id)?
                    } else {
                        self_param_type.clone()
                    };

                    if !types_compatible(&resolved_self_type, &resolved_receiver_type) {
                        return Err(CompileError::typecheck(
                            &format!("Method receiver type mismatch: expected {:?}, got {:?}", self_param_type, receiver_type),
                            (0, 0),
                        ));
                    }
                }

                // Check explicit argument types (skip first param which is self)
                let param_offset = if func.receiver_type.is_some() { 1 } else { 0 };
                for (i, arg) in args.iter().enumerate() {
                    let arg_type = typecheck(&arg.value, symbols, _current_scope_id)?;
                    let expected_type = &func.params[i + param_offset].data_type;

                    // Resolve TypeRefs for comparison
                    let resolved_expected = if matches!(expected_type, DataType::TypeRef(_)) {
                        resolve_type(expected_type, symbols, _current_scope_id)?
                    } else {
                        expected_type.clone()
                    };
                    let resolved_arg = if matches!(&arg_type, DataType::TypeRef(_)) {
                        resolve_type(&arg_type, symbols, _current_scope_id)?
                    } else {
                        arg_type.clone()
                    };

                    if !matches!(expected_type, DataType::Unsolved) && !types_compatible(&resolved_expected, &resolved_arg) {
                        return Err(CompileError::typecheck(
                            &format!("Argument {} type mismatch: expected {:?}, got {:?}", i+1, expected_type, arg_type),
                            (0, 0),
                        ));
                    }
                }

                Ok(func.return_type.clone())
            } else {
                Err(CompileError::name(
                    &format!("Unknown method: {method_name}"),
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
        
        Expr::Len { expr } => {
            let expr_type = typecheck(expr, symbols, _current_scope_id)?;

            // Resolve TypeRef to underlying type if needed
            let resolved_type = if matches!(&expr_type, DataType::TypeRef(_)) {
                resolve_type(&expr_type, symbols, _current_scope_id)?
            } else {
                expr_type
            };

            // len() works on Str, List, and Map types
            match resolved_type {
                DataType::Str | DataType::List { .. } | DataType::Map { .. } => Ok(DataType::Int),
                _ => Err(CompileError::typecheck(
                    &format!("len() requires Str, List, or Map argument, found {:?}", resolved_type),
                    (0, 0),
                ))
            }
        }

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
        // TypeRef compatibility - same name means compatible
        (DataType::TypeRef(name1), DataType::TypeRef(name2)) => name1 == name2,
        _ => false,
    }
}

// Helper to check type compatibility with optional symbol resolution
fn types_compatible_with_resolution(
    t1: &DataType,
    t2: &DataType,
    symbols: &SymbolTable,
    scope: usize,
) -> Result<bool, CompileError> {
    // Resolve TypeRefs first
    let resolved_t1 = if matches!(t1, DataType::TypeRef(_)) {
        resolve_type(t1, symbols, scope)?
    } else {
        t1.clone()
    };

    let resolved_t2 = if matches!(t2, DataType::TypeRef(_)) {
        resolve_type(t2, symbols, scope)?
    } else {
        t2.clone()
    };

    // For complex types, recursively resolve nested TypeRefs
    match (&resolved_t1, &resolved_t2) {
        (DataType::List { element_type: e1 }, DataType::List { element_type: e2 }) => {
            types_compatible_with_resolution(e1, e2, symbols, scope)
        }
        (DataType::Map { key_type: k1, value_type: v1 }, DataType::Map { key_type: k2, value_type: v2 }) => {
            let keys_match = types_compatible_with_resolution(k1, k2, symbols, scope)?;
            let values_match = types_compatible_with_resolution(v1, v2, symbols, scope)?;
            Ok(keys_match && values_match)
        }
        _ => Ok(types_compatible(&resolved_t1, &resolved_t2))
    }
}

// Type inference with symbol table lookup
pub fn determine_type_with_symbols(
    expression: &Expr,
    symbols: &SymbolTable,
    _scope: usize,
) -> Option<DataType> {
    match expression {
        Expr::Literal(l) => Some(match l {
            LiteralData::Int(_) => DataType::Int,
            LiteralData::Str(_) => DataType::Str,
            LiteralData::Flt(_) => DataType::Flt,
            LiteralData::Bool(_) => DataType::Bool,
        }),
        
        Expr::Variable { index, name } => {
            // Look up the variable type from the symbol table
            let var_type = symbols.get_symbol_type(index);
            if DEBUG {
                println!("DEBUG: determine_type_with_symbols - Variable '{}' with index {:?} has type {:?}", name, index, var_type);
            }
            var_type
        }
        
        Expr::BinaryExpr { left, op, right } => {
            let left_type = determine_type_with_symbols(left, symbols, _scope)?;
            let right_type = determine_type_with_symbols(right, symbols, _scope)?;
            
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
                    Expr::DefineFunction { value, .. } => {
                        match value.as_ref() {
                            Expr::Lambda { value: func, .. } => Some(func.return_type.clone()),
                            _ => None
                        }
                    }
                    Expr::Lambda { value: func, .. } => Some(func.return_type.clone()),
                    _ => None,
                }
            } else {
                None
            }
        }

        Expr::MethodCall { fn_index, method_name, receiver, .. } => {
            if DEBUG {
                println!("DEBUG: determine_type_with_symbols - MethodCall '{}' with fn_index {:?}", method_name, fn_index);
            }
            // Look up method return type
            if let Some(fn_expr) = symbols.get_symbol_value(fn_index) {
                if DEBUG {
                    println!("DEBUG: Found symbol for method call: {:?}", fn_expr);
                }
                match fn_expr {
                    Expr::DefineFunction { value, .. } => {
                        match value.as_ref() {
                            Expr::Lambda { value: func, .. } => {
                                let return_type = func.return_type.clone();
                                if DEBUG {
                                    println!("DEBUG: Method '{}' returns {:?}", method_name, return_type);
                                }
                                // For List methods, we need to infer from the receiver
                                match return_type {
                                    DataType::Unsolved => {
                                        // Methods that return the element type (first, last)
                                        if let Some(DataType::List { element_type }) = determine_type_with_symbols(receiver, symbols, _scope) {
                                            return Some(*element_type);
                                        }
                                        Some(DataType::Unsolved)
                                    }
                                    DataType::List { ref element_type } if matches!(**element_type, DataType::Unsolved) => {
                                        // Methods that return a list with the same element type (slice, reverse)
                                        if let Some(DataType::List { element_type }) = determine_type_with_symbols(receiver, symbols, _scope) {
                                            return Some(DataType::List { element_type });
                                        }
                                        Some(return_type)
                                    }
                                    _ => Some(return_type)
                                }
                            }
                            _ => None
                        }
                    }
                    Expr::Lambda { value: func, .. } => {
                        let return_type = func.return_type.clone();
                        // For List methods, we need to infer from the receiver
                        match return_type {
                            DataType::Unsolved => {
                                // Methods that return the element type (first, last)
                                if let Some(DataType::List { element_type }) = determine_type_with_symbols(receiver, symbols, _scope) {
                                    return Some(*element_type);
                                }
                                Some(DataType::Unsolved)
                            }
                            DataType::List { ref element_type } if matches!(**element_type, DataType::Unsolved) => {
                                // Methods that return a list with the same element type (slice, reverse)
                                if let Some(DataType::List { element_type }) = determine_type_with_symbols(receiver, symbols, _scope) {
                                    return Some(DataType::List { element_type });
                                }
                                Some(return_type)
                            }
                            _ => Some(return_type)
                        }
                    }
                    _ => {
                        if DEBUG {
                            println!("DEBUG: Symbol is not a DefineFunction or Lambda: {:?}", fn_expr);
                        }
                        None
                    }
                }
            } else {
                if DEBUG {
                    println!("DEBUG: No symbol found for fn_index {:?}", fn_index);
                }
                None
            }
        }
        
        Expr::Index { expr, .. } => {
            match determine_type_with_symbols(expr, symbols, _scope)? {
                DataType::List { element_type } => Some(*element_type),
                DataType::Map { value_type, .. } => Some(*value_type),
                _ => None,
            }
        }
        
        Expr::If { then, final_else, .. } => {
            // If expression type is the type of its branches
            // Check if there's no else branch (Unit represents missing else)
            if matches!(final_else.as_ref(), Expr::Unit) {
                // Cannot use if without else in contexts requiring type inference
                return None;
            }
            
            let then_type = determine_type_with_symbols(then, symbols, _scope)?;
            let else_type = determine_type_with_symbols(final_else, symbols, _scope)?;
            
            if types_compatible(&then_type, &else_type) {
                Some(then_type)
            } else {
                None
            }
        }
        
        Expr::Block { body, .. } => {
            // Block type is the type of its last expression
            if let Some(last_expr) = body.last() {
                determine_type_with_symbols(last_expr, symbols, _scope)
            } else {
                Some(DataType::Unsolved) // Empty block returns Unit
            }
        }
        
        Expr::UnaryExpr { op: Operator::Not, .. } => Some(DataType::Bool),

        Expr::Len { .. } => Some(DataType::Int),

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

        Expr::Len { .. } => Some(DataType::Int),

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
