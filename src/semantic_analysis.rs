use crate::symboltable::SymbolTable;
use crate::syntax::AssignableData;
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
                AssignableData::Lambda(value.clone(), current_scope_id),
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
            ref data_type,
            ref mut index,
        } => {
            // This is just the first pass. We will assign better values when we do the
            // type inference pass and the type checking pass.
            let partial_typed_value = match data_type {
                DataType::Bool => AssignableData::Literal(LiteralData::Bool(false)),
                DataType::Int => AssignableData::Literal(LiteralData::Int(0)),
                DataType::Flt => AssignableData::Literal(LiteralData::Flt(0.0)),
                DataType::Str => AssignableData::Literal(LiteralData::Str("".to_string())),
                DataType::List { .. } => AssignableData::ListLiteral(Vec::new()),
                _ => AssignableData::Tbd(value.clone()),
            };
            let new_symbol_id =
                symbols.add_symbol(var_name, partial_typed_value, current_scope_id)?;
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
