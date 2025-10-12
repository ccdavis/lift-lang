// Type Inference Module
// Provides type inference capabilities for expressions without full type checking

use crate::symboltable::SymbolTable;
use crate::syntax::{DataType, Expr, KeyData, LiteralData, Operator};
use super::{resolve_type_alias, types_compatible, DEBUG};

/// Type inference with symbol table lookup
/// This function can resolve variable types and other context-dependent type information
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
                                        if let Some(receiver_type_raw) = determine_type_with_symbols(receiver, symbols, _scope) {
                                            let receiver_type = resolve_type_alias(&receiver_type_raw, symbols);
                                            if let DataType::List { element_type } = receiver_type {
                                                return Some(*element_type);
                                            }
                                        }
                                        Some(DataType::Unsolved)
                                    }
                                    DataType::List { ref element_type } if matches!(**element_type, DataType::Unsolved) => {
                                        // Methods that return a list with the same element type (slice, reverse)
                                        if let Some(receiver_type_raw) = determine_type_with_symbols(receiver, symbols, _scope) {
                                            let receiver_type = resolve_type_alias(&receiver_type_raw, symbols);
                                            if let DataType::List { element_type } = receiver_type {
                                                return Some(DataType::List { element_type });
                                            }
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
                                if let Some(receiver_type_raw) = determine_type_with_symbols(receiver, symbols, _scope) {
                                    let receiver_type = resolve_type_alias(&receiver_type_raw, symbols);
                                    if let DataType::List { element_type } = receiver_type {
                                        return Some(*element_type);
                                    }
                                }
                                Some(DataType::Unsolved)
                            }
                            DataType::List { ref element_type } if matches!(**element_type, DataType::Unsolved) => {
                                // Methods that return a list with the same element type (slice, reverse)
                                if let Some(receiver_type_raw) = determine_type_with_symbols(receiver, symbols, _scope) {
                                    let receiver_type = resolve_type_alias(&receiver_type_raw, symbols);
                                    if let DataType::List { element_type } = receiver_type {
                                        return Some(DataType::List { element_type });
                                    }
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
            if let Some(expr_type_raw) = determine_type_with_symbols(expr, symbols, _scope) {
                let expr_type = resolve_type_alias(&expr_type_raw, symbols);
                match expr_type {
                    DataType::List { element_type } => Some(*element_type),
                    DataType::Map { value_type, .. } => Some(*value_type),
                    _ => None,
                }
            } else {
                None
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

        Expr::Program { body, .. } => {
            // Program type is the type of its last expression
            if let Some(last_expr) = body.last() {
                determine_type_with_symbols(last_expr, symbols, _scope)
            } else {
                Some(DataType::Unsolved) // Empty program returns Unit
            }
        }

        Expr::UnaryExpr { op: Operator::Not, .. } => Some(DataType::Bool),

        Expr::Len { .. } => Some(DataType::Int),

        Expr::StructLiteral { type_name, .. } => {
            // Struct literals have the type of their struct
            Some(DataType::TypeRef(type_name.clone()))
        }

        Expr::RuntimeStruct { type_name, .. } => {
            // Runtime structs have the type of their struct
            Some(DataType::TypeRef(type_name.clone()))
        }

        Expr::FieldAccess { expr, field_name } => {
            // Get the expression's type
            let expr_type = determine_type_with_symbols(expr, symbols, _scope)?;

            // Resolve TypeRef to get actual struct definition
            let resolved_type = resolve_type_alias(&expr_type, symbols);

            // Extract field type from struct
            match resolved_type {
                DataType::Struct(params) => {
                    params.iter()
                        .find(|p| p.name == *field_name)
                        .map(|p| p.data_type.clone())
                }
                _ => None
            }
        }

        _ => determine_type(expression), // Fall back to original for other cases
    }
}

/// Type inference for expressions without symbol table context
/// This is a more limited version that only uses information in the expression itself
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
