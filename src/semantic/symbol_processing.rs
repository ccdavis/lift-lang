// Symbol Processing Module
// Handles symbol table population and AST annotation with symbol indices

use super::type_inference::determine_type_with_symbols;
use super::{resolve_type, CompileError, DEBUG};
use crate::symboltable::SymbolTable;
use crate::syntax::{DataType, Expr, Param};

/// Adds symbols for the current scope and child scopes, and updates the index (scope id, symbol id) on the expr
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
            // First check if this is actually a struct literal (Type(field: value, ...))
            // rather than a function call
            if let Some(DataType::Struct(_params)) = symbols.lookup_type(fn_name, _current_scope_id)
            {
                // This is a struct literal! Transform Call → StructLiteral
                // Process field values
                let mut field_pairs = Vec::new();
                for arg in args.iter_mut() {
                    add_symbols(&mut arg.value, symbols, _current_scope_id)?;
                    field_pairs.push((arg.name.clone(), arg.value.clone()));
                }

                // Replace the current expression with StructLiteral
                *e = Expr::StructLiteral {
                    type_name: fn_name.clone(),
                    fields: field_pairs,
                };
                return Ok(());
            }

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
                    if let Err(ref err) =
                        add_symbols(&mut first_arg.value, symbols, _current_scope_id)
                    {
                        let new_msg =
                            format!("Error on argument '{}': {}", first_arg.name, err.clone());
                        return Err(CompileError::structure(&new_msg, (0, 0)));
                    }

                    if let Some(arg_type) =
                        determine_type_with_symbols(&first_arg.value, symbols, _current_scope_id)
                    {
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
                            let is_builtin =
                                matches!(fn_name.as_str(), "upper" | "lower" | "first" | "last");

                            if let Some(found_index) =
                                symbols.find_index_reachable_from(&method_name, _current_scope_id)
                            {
                                if DEBUG {
                                    println!(
                                        "DEBUG: UFCS - found method '{}' for call '{}'",
                                        method_name, fn_name
                                    );
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
                println!(
                    "DEBUG: MethodCall - receiver after add_symbols: {:?}",
                    receiver
                );
            }

            // Determine the receiver's type to find the right method
            let receiver_type = determine_type_with_symbols(receiver, symbols, _current_scope_id)
                .ok_or_else(|| {
                if DEBUG {
                    println!(
                        "DEBUG: Failed to determine receiver type for: {:?}",
                        receiver
                    );
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
                    println!(
                        "DEBUG: Looking for method '{}' on type alias",
                        full_method_name
                    );
                }

                if let Some(found_index) =
                    symbols.find_index_reachable_from(&full_method_name, _current_scope_id)
                {
                    if DEBUG {
                        println!(
                            "DEBUG: Found method '{}' at index {:?}",
                            full_method_name, found_index
                        );
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
                    _ => {
                        return Err(CompileError::typecheck(
                            &format!("Cannot call methods on type {:?}", resolved_type),
                            (0, 0),
                        ))
                    }
                };

                let full_method_name = format!("{}.{}", type_name, method_name);

                if DEBUG {
                    println!("DEBUG: Looking for method '{}'", full_method_name);
                }

                // Look up the method
                if let Some(found_index) =
                    symbols.find_index_reachable_from(&full_method_name, _current_scope_id)
                {
                    if DEBUG {
                        println!(
                            "DEBUG: Found method '{}' at index {:?}",
                            full_method_name, found_index
                        );
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
                    &format!(
                        "Method '{}' not found for type '{}'",
                        method_name, type_display
                    ),
                    (0, 0),
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
                    "List" => {
                        return Err(CompileError::typecheck(
                            "Method on generic List requires type parameter",
                            (0, 0),
                        ))
                    }
                    "Map" => {
                        return Err(CompileError::typecheck(
                            "Method on generic Map requires type parameter",
                            (0, 0),
                        ))
                    }
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
                    mutable: p.copy, // Only mutable if marked with 'cpy'
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
            // At first just create the symbol table entry for the function and make the value the Unit value...
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
                if let Some(inferred_type) =
                    determine_type_with_symbols(value, symbols, _current_scope_id)
                {
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

        Expr::ListLiteral {
            ref mut data,
            data_type: _,
        } => {
            // Add symbols for each element in the list
            for elem in data.iter_mut() {
                add_symbols(elem, symbols, _current_scope_id)?;
            }
            // Note: data_type here refers to the element type, not the list type itself
            // We don't need to change it in add_symbols - it's handled in typecheck
        }
        Expr::MapLiteral {
            ref mut data,
            key_type: _,
            value_type: _,
        } => {
            // Add symbols for each value in the map
            for (_, value) in data.iter_mut() {
                add_symbols(value, symbols, _current_scope_id)?;
            }
            // Note: key_type and value_type are handled in typecheck, not here
        }
        Expr::Range(..) => {
            // Range literals don't need symbol processing
        }
        Expr::Index {
            ref mut expr,
            ref mut index,
        } => {
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
        Expr::Assign {
            ref name,
            ref mut value,
            ref mut index,
        } => {
            // Process the value expression first
            add_symbols(value, symbols, _current_scope_id)?;

            // Find the variable in current or parent scopes
            if let Some(found_index) = symbols.find_index_reachable_from(name, _current_scope_id) {
                *index = found_index;
            } else {
                return Err(CompileError::name(
                    &format!("Cannot assign to undeclared variable '{}'", name),
                    (0, 0),
                ));
            }
        }
        Expr::FieldAccess { ref mut expr, .. } => {
            // Process the expression being accessed
            add_symbols(expr, symbols, _current_scope_id)?;
        }
        Expr::FieldAssign {
            ref mut expr,
            field_name: _,
            ref mut value,
            ref mut index,
        } => {
            // Process both the expression (which should be a variable) and the value
            add_symbols(expr, symbols, _current_scope_id)?;
            add_symbols(value, symbols, _current_scope_id)?;

            // Find the variable index for mutability checking
            // The expr should be a Variable - extract its index
            if let Expr::Variable {
                name: _,
                index: var_index,
            } = expr.as_ref()
            {
                *index = *var_index;
            } else {
                return Err(CompileError::typecheck(
                    "Field assignment target must be a variable",
                    (0, 0),
                ));
            }
        }
        Expr::StructLiteral { ref mut fields, .. } => {
            // Process all field value expressions
            for (_, field_value) in fields.iter_mut() {
                add_symbols(field_value, symbols, _current_scope_id)?;
            }
        }
        _ => (),
    }
    Ok(())
}
