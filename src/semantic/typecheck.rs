// Type Checking Module
// Main dispatcher for type checking expressions

use crate::semantic::{determine_type_with_symbols, resolve_type, CompileError};
use crate::symboltable::SymbolTable;
use crate::syntax::{DataType, Expr, KeyData, LiteralData, Operator};

const DEBUG: bool = false;

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

        Expr::If {
            cond,
            then,
            final_else,
        } => {
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

        Expr::Let {
            var_name,
            value,
            data_type,
            index: _,
            mutable: _,
        } => {
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
                        if !types_compatible_with_resolution(
                            &resolved_type,
                            &value_type,
                            symbols,
                            _current_scope_id,
                        )? {
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
                if let Some(_inferred_type) =
                    determine_type_with_symbols(value, symbols, _current_scope_id)
                {
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
                        &format!(
                            "Cannot infer type for '{var_name}'. Please provide a type annotation."
                        ),
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
                        &format!(
                            "Cannot assign {value_type:?} to variable {name} of type {var_type:?}"
                        ),
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

        Expr::FieldAssign {
            expr,
            field_name,
            value,
            index,
        } => {
            // 1. Check if the struct variable is mutable
            if !symbols.is_mutable(index) {
                return Err(CompileError::typecheck(
                    "Cannot assign to field of immutable struct. Use 'let var' to declare mutable structs.",
                    (0, 0),
                ));
            }

            // 2. Get the struct type
            let struct_type = typecheck(expr, symbols, _current_scope_id)?;

            // 3. Resolve TypeRef to get actual struct definition
            let resolved_type = if matches!(&struct_type, DataType::TypeRef(_)) {
                resolve_type(&struct_type, symbols, _current_scope_id)?
            } else {
                struct_type.clone()
            };

            // 4. Verify it's a struct and get field type
            let field_type = match resolved_type {
                DataType::Struct(params) => params
                    .iter()
                    .find(|p| p.name == *field_name)
                    .map(|p| p.data_type.clone())
                    .ok_or_else(|| {
                        CompileError::typecheck(
                            &format!("Struct has no field '{}'", field_name),
                            (0, 0),
                        )
                    })?,
                _ => {
                    return Err(CompileError::typecheck(
                        &format!(
                            "Cannot assign to field '{}' on non-struct type {:?}",
                            field_name, struct_type
                        ),
                        (0, 0),
                    ))
                }
            };

            // 5. Type check the value
            let value_type = typecheck(value, symbols, _current_scope_id)?;

            // 6. Resolve and check compatibility
            let resolved_field_type = if matches!(&field_type, DataType::TypeRef(_)) {
                resolve_type(&field_type, symbols, _current_scope_id)?
            } else {
                field_type.clone()
            };
            let resolved_value_type = if matches!(&value_type, DataType::TypeRef(_)) {
                resolve_type(&value_type, symbols, _current_scope_id)?
            } else {
                value_type.clone()
            };

            if !types_compatible(&resolved_field_type, &resolved_value_type) {
                return Err(CompileError::typecheck(
                    &format!(
                        "Cannot assign {:?} to field '{}' of type {:?}",
                        value_type, field_name, field_type
                    ),
                    (0, 0),
                ));
            }

            // Field assignment returns Unit
            Ok(DataType::Unsolved)
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

        Expr::RuntimeList { data_type, data: _ } => Ok(DataType::List {
            element_type: Box::new(data_type.clone()),
        }),

        Expr::MapLiteral {
            key_type,
            value_type,
            data,
        } => {
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
                        &format!(
                            "Map value type mismatch: expected {:?}, got {:?}",
                            actual_value_type, value_type_actual
                        ),
                        (0, 0),
                    ));
                }
            }

            Ok(DataType::Map {
                key_type: Box::new(actual_key_type),
                value_type: Box::new(actual_value_type),
            })
        }

        Expr::RuntimeMap {
            key_type,
            value_type,
            ..
        } => Ok(DataType::Map {
            key_type: Box::new(key_type.clone()),
            value_type: Box::new(value_type.clone()),
        }),

        Expr::Range(start, end) => {
            // Range literals are always integers
            match (start, end) {
                (LiteralData::Int(_), LiteralData::Int(_)) => Ok(DataType::Range(Box::new(
                    Expr::Range(start.clone(), end.clone()),
                ))),
                _ => Err(CompileError::typecheck(
                    "Range literals must have integer bounds",
                    (0, 0),
                )),
            }
        }

        Expr::Call {
            fn_name,
            index,
            args,
        } => {
            // Get function type from symbol table
            if let Some(fn_expr) = symbols.get_symbol_value(index) {
                // Extract the function from either DefineFunction or Lambda
                let func = match fn_expr {
                    Expr::DefineFunction { value, .. } => match value.as_ref() {
                        Expr::Lambda { value: f, .. } => f,
                        _ => {
                            return Err(CompileError::typecheck(
                                &format!("{fn_name} is not a function"),
                                (0, 0),
                            ))
                        }
                    },
                    Expr::Lambda { value: f, .. } => f,
                    _ => {
                        return Err(CompileError::typecheck(
                            &format!("{fn_name} is not a function"),
                            (0, 0),
                        ))
                    }
                };

                // Check if this is a UFCS call (method called with function syntax)
                // If the function has a receiver_type, it's a method
                let is_ufcs_call = func.receiver_type.is_some();

                let expected_arg_count = func.params.len();
                if args.len() != expected_arg_count {
                    return Err(CompileError::typecheck(
                        &format!(
                            "Function {} expects {} arguments, got {}",
                            fn_name,
                            expected_arg_count,
                            args.len()
                        ),
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
                                &format!(
                                    "{} type mismatch: expected {:?}, got {:?}",
                                    arg_num_display, expected_type, arg_type
                                ),
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

        Expr::MethodCall {
            receiver,
            method_name,
            fn_index,
            args,
        } => {
            if DEBUG {
                println!("DEBUG: typecheck MethodCall - receiver: {:?}", receiver);
            }
            // Type check the receiver
            let receiver_type = typecheck(receiver, symbols, _current_scope_id)?;
            if DEBUG {
                println!(
                    "DEBUG: typecheck MethodCall - receiver_type: {:?}",
                    receiver_type
                );
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
                                &format!(
                                    "{} can only be called on Str, got {:?}",
                                    method_name, receiver_type
                                ),
                                (0, 0),
                            ));
                        }
                        if !args.is_empty() {
                            return Err(CompileError::typecheck(
                                &format!(
                                    "{} expects no arguments, got {}",
                                    method_name,
                                    args.len()
                                ),
                                (0, 0),
                            ));
                        }
                        Ok(DataType::Str)
                    }
                    "first" | "last" => match resolved_receiver_type {
                        DataType::List { element_type } => {
                            if !args.is_empty() {
                                return Err(CompileError::typecheck(
                                    &format!(
                                        "{} expects no arguments, got {}",
                                        method_name,
                                        args.len()
                                    ),
                                    (0, 0),
                                ));
                            }
                            Ok(*element_type)
                        }
                        _ => Err(CompileError::typecheck(
                            &format!(
                                "{} can only be called on List, got {:?}",
                                method_name, receiver_type
                            ),
                            (0, 0),
                        )),
                    },
                    _ => Ok(DataType::Unsolved),
                }
            } else if let Some(fn_expr) = symbols.get_symbol_value(fn_index) {
                // Extract the function from either DefineFunction or Lambda
                let func = match fn_expr {
                    Expr::DefineFunction { value, .. } => match value.as_ref() {
                        Expr::Lambda { value: f, .. } => f,
                        _ => {
                            return Err(CompileError::typecheck(
                                &format!("{method_name} is not a method"),
                                (0, 0),
                            ))
                        }
                    },
                    Expr::Lambda { value: f, .. } => f,
                    _ => {
                        return Err(CompileError::typecheck(
                            &format!("{method_name} is not a method"),
                            (0, 0),
                        ))
                    }
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
                        &format!(
                            "Method {} expects {} arguments, got {}",
                            method_name,
                            expected_arg_count,
                            args.len()
                        ),
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
                            &format!(
                                "Method receiver type mismatch: expected {:?}, got {:?}",
                                self_param_type, receiver_type
                            ),
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

                    if !matches!(expected_type, DataType::Unsolved)
                        && !types_compatible(&resolved_expected, &resolved_arg)
                    {
                        return Err(CompileError::typecheck(
                            &format!(
                                "Argument {} type mismatch: expected {:?}, got {:?}",
                                i + 1,
                                expected_type,
                                arg_type
                            ),
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

            // Check return type matches if specified (resolve type aliases for comparison)
            if !matches!(func.return_type, DataType::Unsolved)
                && !types_compatible_with_resolution(
                    &func.return_type,
                    &body_type,
                    symbols,
                    _current_scope_id,
                )?
            {
                return Err(CompileError::typecheck(
                    &format!(
                        "Function body returns {body_type:?} but return type is {:?}",
                        func.return_type
                    ),
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
                for expr in &body[..body.len() - 1] {
                    typecheck(expr, symbols, *environment)?;
                }
                // Return the type of the last expression
                typecheck(&body[body.len() - 1], symbols, *environment)
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
                    &format!(
                        "len() requires Str, List, or Map argument, found {:?}",
                        resolved_type
                    ),
                    (0, 0),
                )),
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
                }
                DataType::Map {
                    key_type,
                    value_type,
                } => {
                    // For maps, index must match key type
                    if index_type != *key_type {
                        return Err(CompileError::typecheck(
                            &format!(
                                "Map key must be of type {:?}, found {:?}",
                                key_type, index_type
                            ),
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
                        )),
                    }
                }
                _ => Err(CompileError::typecheck(
                    &format!(
                        "Cannot index into type {:?}, only List and Map types can be indexed",
                        expr_type
                    ),
                    (0, 0),
                )),
            }
        }

        Expr::StructLiteral { type_name, fields } => {
            // Look up the struct type definition
            let struct_def = symbols
                .lookup_type(type_name, _current_scope_id)
                .ok_or_else(|| {
                    CompileError::name(&format!("Unknown struct type: {}", type_name), (0, 0))
                })?;

            // Extract field definitions from the struct type
            let expected_fields = match struct_def {
                DataType::Struct(params) => params,
                _ => {
                    return Err(CompileError::typecheck(
                        &format!("{} is not a struct type", type_name),
                        (0, 0),
                    ))
                }
            };

            // Build a map of field names to their expected types
            let mut expected_field_map: std::collections::HashMap<String, DataType> =
                expected_fields
                    .iter()
                    .map(|param| (param.name.clone(), param.data_type.clone()))
                    .collect();

            // Check that all provided fields exist and have correct types
            for (field_name, field_value) in fields {
                if let Some(expected_type) = expected_field_map.remove(field_name) {
                    // Type check the field value
                    let actual_type = typecheck(field_value, symbols, _current_scope_id)?;

                    // Resolve type aliases for comparison
                    let resolved_expected = if matches!(&expected_type, DataType::TypeRef(_)) {
                        resolve_type(&expected_type, symbols, _current_scope_id)?
                    } else {
                        expected_type.clone()
                    };
                    let resolved_actual = if matches!(&actual_type, DataType::TypeRef(_)) {
                        resolve_type(&actual_type, symbols, _current_scope_id)?
                    } else {
                        actual_type.clone()
                    };

                    if !types_compatible(&resolved_expected, &resolved_actual) {
                        return Err(CompileError::typecheck(
                            &format!(
                                "Field '{}' in struct '{}' has wrong type: expected {:?}, got {:?}",
                                field_name, type_name, expected_type, actual_type
                            ),
                            (0, 0),
                        ));
                    }
                } else {
                    return Err(CompileError::typecheck(
                        &format!("Struct '{}' has no field named '{}'", type_name, field_name),
                        (0, 0),
                    ));
                }
            }

            // Check that all required fields were provided
            if !expected_field_map.is_empty() {
                let missing_fields: Vec<_> = expected_field_map.keys().collect();
                return Err(CompileError::typecheck(
                    &format!(
                        "Struct '{}' is missing required fields: {:?}",
                        type_name, missing_fields
                    ),
                    (0, 0),
                ));
            }

            // Return the struct type
            Ok(DataType::TypeRef(type_name.clone()))
        }

        Expr::RuntimeStruct { type_name, .. } => {
            // Runtime structs already have their type determined
            Ok(DataType::TypeRef(type_name.clone()))
        }

        Expr::FieldAccess { expr, field_name } => {
            // 1. Type check the expression
            let expr_type = typecheck(expr, symbols, _current_scope_id)?;

            // 2. Resolve if it's a TypeRef
            let resolved_type = if matches!(&expr_type, DataType::TypeRef(_)) {
                resolve_type(&expr_type, symbols, _current_scope_id)?
            } else {
                expr_type.clone()
            };

            // 3. Verify it's a struct and get field type
            match resolved_type {
                DataType::Struct(params) => {
                    // 4. Find the field and return its type
                    params
                        .iter()
                        .find(|p| p.name == *field_name)
                        .map(|p| p.data_type.clone())
                        .ok_or_else(|| {
                            CompileError::typecheck(
                                &format!("Struct has no field '{}'", field_name),
                                (0, 0),
                            )
                        })
                }
                _ => Err(CompileError::typecheck(
                    &format!(
                        "Cannot access field '{}' on non-struct type {:?}",
                        field_name, expr_type
                    ),
                    (0, 0),
                )),
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
        (
            DataType::Map {
                key_type: k1,
                value_type: v1,
            },
            DataType::Map {
                key_type: k2,
                value_type: v2,
            },
        ) => types_compatible(k1, k2) && types_compatible(v1, v2),
        (DataType::Range(_), DataType::Range(_)) => {
            // For now, all ranges are compatible with each other
            // In the future we might want to check the bounds
            true
        }
        // Struct compatibility - check field names and types match
        (DataType::Struct(params1), DataType::Struct(params2)) => {
            if params1.len() != params2.len() {
                return false;
            }
            // Check that all field names and types match (order matters)
            params1.iter().zip(params2.iter()).all(|(p1, p2)| {
                p1.name == p2.name && types_compatible(&p1.data_type, &p2.data_type)
            })
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
        (
            DataType::Map {
                key_type: k1,
                value_type: v1,
            },
            DataType::Map {
                key_type: k2,
                value_type: v2,
            },
        ) => {
            let keys_match = types_compatible_with_resolution(k1, k2, symbols, scope)?;
            let values_match = types_compatible_with_resolution(v1, v2, symbols, scope)?;
            Ok(keys_match && values_match)
        }
        _ => Ok(types_compatible(&resolved_t1, &resolved_t2)),
    }
}
