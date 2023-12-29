use crate::semantic_analysis::ParseError;
use crate::syntax::AssignableData;
use crate::syntax::Expr;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Scope {
    pub parent: Option<usize>,
    pub data: Vec<AssignableData>,
    pub runtime_value: Vec<Expr>,
    pub name: HashMap<usize, String>,
    pub index: HashMap<String, usize>,
}

pub struct SymbolTable(Vec<Scope>);

impl SymbolTable {
    pub fn new() -> Self {
        let mut symbols = SymbolTable(Vec::new());
        symbols.create_scope(None);
        symbols
    }

    pub fn create_scope(&mut self, parent: Option<usize>) -> usize {
        self.0.push(Scope::new(parent));
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

    pub fn add_symbol(
        &mut self,
        name: &str,
        value: AssignableData,
        scope: usize,
    ) -> Result<usize, ParseError> {
        self.0[scope].add(name, value)
    }

    pub fn update_value(&mut self, value: Expr, index: &(usize, usize)) {
        self.0[index.0].runtime_value[index.1] = value;
    }

    pub fn get_compiletime_value(&self, index: &(usize, usize)) -> AssignableData{
        self.0[index.0].data[index.1].clone()
    }

    pub fn get_runtime_value(&self, index: &(usize, usize)) -> Expr {
        self.0[index.0].runtime_value[index.1].clone()
    }
}

impl Scope {
    pub fn new(parent: Option<usize>) -> Self {
        Self {
            parent,
            data: Vec::new(),
            runtime_value: Vec::new(),
            name: HashMap::new(),
            index: HashMap::new(),
        }
    }

    pub fn get_index(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    pub fn add(&mut self, name: &str, value: AssignableData) -> Result<usize, ParseError> {
        if self.index.contains_key(name) {
            Err(format!("Symbol already defined in this scope: {}", name))
        } else {
            self.data.push(value.clone());
            self.runtime_value.push(value.into());
            let new_index = self.data.len() - 1;
            self.index.insert(name.to_string(), new_index);
            self.name.insert(new_index, name.to_string());
            Ok(new_index)
        }
    }
}
