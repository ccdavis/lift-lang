use crate::symboltable::SymbolTable;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::LiteralData;

pub type ParseError = String;
// This adds symbols for the current scope and the child scopes, plus updates the index (scope id, symbol id) on the expr
pub fn add_symbols(
    e: &mut Expr,
    symbols: &mut SymbolTable,
    current_scope_id: usize,
) -> Result<(), ParseError> {
    match *e {
        Expr::DefineFunction {
            ref fn_name,
            ref mut index,
            ref value,
            ref mut environment,
        } => {
            // The function is getting defined for the current scope:
            let new_symbol_id = symbols.add_symbol(
                fn_name,
                Expr::Lambda {
                    value: value.clone(),
                    environment: current_scope_id,
                },
                current_scope_id,
            )?;
            *index = (current_scope_id, new_symbol_id);
            // The function has its own scope as well which we should create
            let new_scope_id = symbols.create_scope(Some(current_scope_id));
            *environment = new_scope_id;
        }
        // Here we set the variable's index from the already added symbol and catch
        // places where the call comes before the definition.
        Expr::Variable {
            ref name,
            ref mut index,
        } => {
            if let Some(found_index) = symbols.find_index_reachable_from(name, current_scope_id) {
                *index = found_index;
            } else {
                let msg = format!("use of undeclared or not yet declared variable '{}'", name);
                return Err(msg);
            }
        }

        Expr::Let {
            ref var_name,
            ref value,
            ref mut data_type,
            ref mut index,
        } => {
            if matches!(data_type,DataType::Any) {
                if let Some(inferred_type) = determine_type(value) {
                    *data_type = inferred_type;
                }
            }

            let new_symbol_id = symbols.add_symbol(var_name, *value.clone(), current_scope_id)?;
            *index = (current_scope_id, new_symbol_id);
        }
        Expr::Block {
            ref mut body,
            ref mut environment,
        } => {
            // Create the new scope first
            let new_scope_id = symbols.create_scope(Some(current_scope_id));
            *environment = new_scope_id;
            for ref mut e in body {
                add_symbols(e, symbols, new_scope_id)?
            }
        }
        _ => (),
    }
    Ok(())
}

pub fn determine_type(expression: &Expr) -> Option<DataType> {
    let inferred_type = match expression {
        Expr::Literal(l) => match l {
            LiteralData::Int(_) => DataType::Int,
            LiteralData::Str(_) => DataType::Str,
            LiteralData::Flt(_) => DataType::Flt,
            LiteralData::Bool(_) => DataType::Bool,
        },
        Expr::ListLiteral {
            ref data_type,
            ref data,
        } => {
            // Check first element and use that as the inferred type
            let mut element_type = data_type.clone();
            if matches!(data_type, DataType::Any) {
                if let Some(reference_expr) = data.first() {
                    if let Some(reference_type) = determine_type(reference_expr) {
                        element_type = reference_type;
                    }
                }
            }
            DataType::List {
                element_type: Box::new(element_type.clone()),
            }
        }
        _ => DataType::Any,
    }; // match
    if matches!(inferred_type, DataType::Any) {
        None
    } else {
        Some(inferred_type)
    }
}
