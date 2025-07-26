use crate::semantic_analysis::CompileError;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::LiteralData;
use std::collections::HashMap;

const TRACE: bool = true;

#[derive(Clone, Debug)]
pub struct Scope {
    pub parent: Option<usize>,
    pub data: Vec<Expr>,
    pub types: Vec<DataType>,
    pub runtime_value: Vec<Expr>,
    pub name: HashMap<usize, String>,
    pub index: HashMap<String, usize>,
    pub type_name: HashMap<usize, String>,
    pub type_index: HashMap<String, usize>,
}

impl Scope {
    pub fn borrow_runtime_data(&self, index: usize) -> &Expr {
        &self.runtime_value[index]
    }

    pub fn print_debug(&self) {
        for kv in &self.index {
            println!("{} : {}", kv.0, kv.1);
        }
    }
}

pub struct SymbolTable(Vec<Scope>);

impl SymbolTable {
    pub fn new() -> Self {
        let mut symbols = SymbolTable(Vec::new());
        symbols.create_scope(None);
        symbols
    }

    pub fn print_debug(&self) {
        for (s, scope) in self.0.iter().enumerate() {
            println!("Scope {} ------- ", s);
            scope.print_debug();
        }
    }

    pub fn create_scope(&mut self, parent: Option<usize>) -> usize {
        self.0.push(Scope::new(parent));
        if TRACE {
            println!("Add scope {} with parent {:?}", self.0.len() - 1, &parent);
        }
        self.0.len() - 1
    }

    // Determine if a symbol is in the current scope or any of its parent scopes.
    pub fn get_index_in_scope(&self, symbol_name: &str, current_scope_id: usize) -> Option<usize> {
        self.0[current_scope_id].get_index(symbol_name)
    }

    pub fn find_index_reachable_from(
        &self,
        symbol_name: &str,
        current_scope_id: usize,
    ) -> Option<(usize, usize)> {
        if TRACE {
            println!(
                "Find  index for {} in scope {}",
                symbol_name, current_scope_id
            )
        }
        match self.get_index_in_scope(symbol_name, current_scope_id) {
            Some(index) => Some((current_scope_id, index)),
            None => {
                let parent_scope_id = self.0[current_scope_id].parent;
                match parent_scope_id {
                    None => None,
                    Some(scope_id) => self.find_index_reachable_from(symbol_name, scope_id),
                }
            }
        }
    }

    pub fn add_type(
        &mut self,
        name: &str,
        value: &DataType,
        scope: usize,
    ) -> Result<usize, CompileError> {
        let added_index = self.0[scope].add_type(name, value.clone());
        if TRACE {
            println!(
                "Added '{}' to symbol table:scope {},  at index {:?} with value '{:?}'",
                name,
                &scope,
                &added_index,
                &value.clone()
            )
        }
        added_index
    }

    pub fn add_symbol(
        &mut self,
        name: &str,
        value: Expr,
        scope: usize,
    ) -> Result<usize, CompileError> {
        let added_index = self.0[scope].add(name, value.clone());
        if TRACE {
            println!(
                "Added '{}' to symbol table:scope {},  at index {:?} with value '{:?}'",
                name,
                &scope,
                &added_index,
                &value.clone()
            )
        }
        added_index
    }

    pub fn update_compiletime_symbol_value(&mut self, value: Expr, index: &(usize, usize)) {
        self.0[index.0].data[index.1] = value;
    }

    pub fn update_runtime_value(&mut self, value: Expr, index: &(usize, usize)) {
        self.0[index.0].runtime_value[index.1] = value;
    }

    pub fn get_compiletime_value(&self, index: &(usize, usize)) -> Option<Expr> {
        Some(self.0.get(index.0)?.data.get(index.1)?.clone())
    }

    pub fn get_runtime_value(&self, index: &(usize, usize)) -> Option<Expr> {
        Some(self.0.get(index.0)?.runtime_value.get(index.1)?.clone())
    }

    pub fn borrow_runtime_value(&self, index: (usize, usize)) -> &Expr {
        &self.0[index.0].runtime_value[index.1]
    }
    
    pub fn get_symbol_value(&self, index: &(usize, usize)) -> Option<&Expr> {
        self.0.get(index.0)?.data.get(index.1)
    }
    
    pub fn get_symbol_type(&self, index: &(usize, usize)) -> Option<DataType> {
        let expr = self.0.get(index.0)?.data.get(index.1)?;
        match expr {
            Expr::Let { data_type, value, .. } => {
                // If type annotation is provided, use it; otherwise infer from value
                if !matches!(data_type, DataType::Unsolved) {
                    Some(data_type.clone())
                } else {
                    // Try to infer type from value
                    match value.as_ref() {
                        Expr::Literal(l) => Some(match l {
                            LiteralData::Int(_) => DataType::Int,
                            LiteralData::Flt(_) => DataType::Flt,
                            LiteralData::Str(_) => DataType::Str,
                            LiteralData::Bool(_) => DataType::Bool,
                        }),
                        _ => Some(DataType::Unsolved),
                    }
                }
            }
            Expr::Lambda { value: func, .. } => Some(DataType::Unsolved), // Functions don't have a simple type yet
            Expr::Unit => {
                // This might be a function parameter placeholder
                // Look for the parameter's type in the parent scope's function definition
                Some(DataType::Unsolved)
            }
            _ => None,
        }
    }
}

impl Scope {
    pub fn new(parent: Option<usize>) -> Self {
        Self {
            parent,
            data: Vec::new(),
            types: Vec::new(),
            runtime_value: Vec::new(),
            name: HashMap::new(),
            type_name: HashMap::new(),
            index: HashMap::new(),
            type_index: HashMap::new(),
        }
    }

    pub fn get_index(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    pub fn add(&mut self, name: &str, value: Expr) -> Result<usize, CompileError> {
        if self.index.contains_key(name) {
            Err(CompileError::name(
                &format!("Symbol already defined in this scope: {}", name),
                (0, 0),
            ))
        } else {
            self.data.push(value.clone());
            self.runtime_value.push(value.copy_to_runtime_data());
            let new_index = self.data.len() - 1;
            self.index.insert(name.to_string(), new_index);
            self.name.insert(new_index, name.to_string());
            Ok(new_index)
        }
    }

    pub fn add_type(&mut self, name: &str, value: DataType) -> Result<usize, CompileError> {
        if self.type_index.contains_key(name) {
            Err(CompileError::name(
                &format!("Type already defined in this scope: {}", name),
                (0, 0),
            ))
        } else {
            self.types.push(value.clone());
            let new_index = self.types.len() - 1;
            self.type_index.insert(name.to_string(), new_index);
            self.type_name.insert(new_index, name.to_string());
            Ok(new_index)
        }
    }
}
