use crate::symboltable::SymbolTable;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::Function;
use crate::syntax::LiteralData;

const DEBUG: bool = true;

#[derive(Clone, Debug)]
pub enum CompileErrorType {
    Structure,
    Name,
    TypeCheck,
}
impl CompileErrorType {
    pub fn name(&self) -> String {
        match self {
            CompileErrorType::TypeCheck { .. } => "Type check Error",
            CompileErrorType::Name { .. } => "Name Error",
            CompileErrorType::Structure { .. } => "Structure Error",
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
// TODO make a generic traversal function that takes a "visitor" lambda or selects between some different
// visitor type functions like "add_symbols", "type_check", "print" etc.
pub fn add_symbols(
    e: &mut Expr,
    symbols: &mut SymbolTable,
    current_scope_id: usize,
) -> Result<(), CompileError> {
    if DEBUG {
        println!(
            "DEBUG: adding symbols to expr '{}' at scope '{}'\n\n",
            &e, current_scope_id
        );
    }
    match *e {
        Expr::DefineType {
            type_name,
            definition,
            index,
        } => {
            let symbol_id = symbols.add_type(&type_name, &definition, current_scope_id)?;
        }
        Expr::Output { ref mut data } => {
            for mut e in data {
                add_symbols(e, symbols, current_scope_id)?;
            }
        }
        Expr::Block {
            ref mut body,
            ref mut environment,
        } => {
            let new_scope_id = symbols.create_scope(Some(current_scope_id));
            *environment = new_scope_id;
            for e in body {
                add_symbols(e, symbols, new_scope_id)?;
            }
        }
        Expr::BinaryExpr {
            ref mut left,
            ref op,
            ref mut right,
        } => {
            add_symbols(left, symbols, current_scope_id)?;
            add_symbols(right, symbols, current_scope_id)?;
        }
        Expr::If {
            ref mut cond,
            ref mut then,
            ref mut final_else,
        } => {
            add_symbols(cond, symbols, current_scope_id)?;
            add_symbols(then, symbols, current_scope_id)?;
            add_symbols(final_else, symbols, current_scope_id)?;
        }
        Expr::While {
            ref mut cond,
            ref mut body,
        } => {
            add_symbols(cond, symbols, current_scope_id)?;
            add_symbols(body, symbols, current_scope_id)?;
        }
        Expr::Call {
            ref fn_name,
            ref mut index,
            ref mut args,
        } => {
            if let Some(found_index) = symbols.find_index_reachable_from(fn_name, current_scope_id)
            {
                if DEBUG {
                    println!("DEBUG: During semantic analysis phase found index '{},{}' for '{}' function call.",
                    found_index.0, found_index.1,fn_name 
                );
                }
                *index = found_index;
            } else {
                let msg = format!(
                    "use of undeclared or not yet declared function '{}' at scope {}",
                    fn_name, current_scope_id
                );
                if DEBUG {
                    eprintln!("{}", &msg);
                }
                return Err(CompileError::name(&msg, (0, 0)));
            }
            for a in args {
                if let Err(ref err) = add_symbols(&mut a.value, symbols, current_scope_id) {
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
            let new_scope_id = symbols.create_scope(Some(current_scope_id));
            *environment = new_scope_id;

            // Add params to the new environment
            for p in &mut value.params {
                let new_symbol_id = symbols.add_symbol(&p.name, Expr::Unit, new_scope_id)?;
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
            let new_symbol_id = symbols.add_symbol(fn_name, Expr::Unit, current_scope_id)?;
            if DEBUG {
                println!("Added symbol id {} for function {}", new_symbol_id, fn_name);
            }
            // Then update the body (value) with all the right symbol indices including the function itself, to
            // support recursion...
            add_symbols(value, symbols, current_scope_id)?;
            // Now update the compile time value of the function with the correct indices for
            // all symbols.
            symbols.update_compiletime_symbol_value(
                *value.clone(),
                &(current_scope_id, new_symbol_id),
            );

            // The function is getting defined for the current scope:
            *index = (current_scope_id, new_symbol_id);
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
                return Err(CompileError::name(&msg, (0, 0)));
            }
        }

        Expr::Let {
            ref var_name,
            ref mut value,
            ref mut data_type,
            ref mut index,
        } => {
            if matches!(data_type, DataType::Unsolved) {
                if let Some(inferred_type) = determine_type(value) {
                    *data_type = inferred_type;
                }
            }
            add_symbols(value, symbols, current_scope_id)?;
            let new_symbol_id = symbols.add_symbol(var_name, *value.clone(), current_scope_id)?;
            *index = (current_scope_id, new_symbol_id);
        }
        Expr::Return(ref mut e) => add_symbols(e, symbols, current_scope_id)?,

        _ => (),
    }
    Ok(())
}
// TODO  determine_type() is incomplete. Does not address all types and does not fully traverse the tree.
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
        _ => DataType::Unsolved,
    }; // match
    if matches!(inferred_type, DataType::Unsolved) {
        None
    } else {
        Some(inferred_type)
    }
}
