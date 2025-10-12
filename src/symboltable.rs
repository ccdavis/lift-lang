use crate::semantic::CompileError;
use crate::syntax::BuiltinMethod;
use crate::syntax::DataType;
use crate::syntax::Expr;
use crate::syntax::LiteralData;
use crate::syntax::{Function, Param};
use std::collections::HashMap;

const TRACE: bool = false;

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
    pub fn print_debug(&self) {
        for kv in &self.index {
            println!("{} : {}", kv.0, kv.1);
        }
    }
}

pub struct SymbolTable(Vec<Scope>);

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut symbols = SymbolTable(Vec::new());
        symbols.create_scope(None);
        symbols
    }

    /// Add built-in methods to the symbol table
    pub fn add_builtins(&mut self) -> Result<(), CompileError> {
        // Check if builtins have already been added
        if self.find_index_reachable_from("Str.upper", 0).is_some() {
            // Built-ins already added
            return Ok(());
        }

        // === String Methods ===

        // Str.upper() -> Str
        self.add_builtin_method("Str", "upper", DataType::Str, DataType::Str)?;

        // Str.lower() -> Str
        self.add_builtin_method("Str", "lower", DataType::Str, DataType::Str)?;

        // Str.substring(start: Int, end: Int) -> Str
        self.add_builtin_method_with_params(
            "Str",
            "substring",
            DataType::Str,
            vec![("start", DataType::Int), ("end", DataType::Int)],
            DataType::Str,
        )?;

        // Str.contains(substring: Str) -> Bool
        self.add_builtin_method_with_params(
            "Str",
            "contains",
            DataType::Str,
            vec![("substring", DataType::Str)],
            DataType::Bool,
        )?;

        // Str.trim() -> Str
        self.add_builtin_method("Str", "trim", DataType::Str, DataType::Str)?;

        // Str.split(delimiter: Str) -> List of Str
        self.add_builtin_method_with_params(
            "Str",
            "split",
            DataType::Str,
            vec![("delimiter", DataType::Str)],
            DataType::List {
                element_type: Box::new(DataType::Str),
            },
        )?;

        // Str.replace(old: Str, new: Str) -> Str
        self.add_builtin_method_with_params(
            "Str",
            "replace",
            DataType::Str,
            vec![("old", DataType::Str), ("new", DataType::Str)],
            DataType::Str,
        )?;

        // Str.starts_with(prefix: Str) -> Bool
        self.add_builtin_method_with_params(
            "Str",
            "starts_with",
            DataType::Str,
            vec![("prefix", DataType::Str)],
            DataType::Bool,
        )?;

        // Str.ends_with(suffix: Str) -> Bool
        self.add_builtin_method_with_params(
            "Str",
            "ends_with",
            DataType::Str,
            vec![("suffix", DataType::Str)],
            DataType::Bool,
        )?;

        // Str.is_empty() -> Bool
        self.add_builtin_method("Str", "is_empty", DataType::Str, DataType::Bool)?;

        // === List Methods ===

        // List.first() -> Unsolved (will be resolved based on list element type)
        self.add_builtin_method(
            "List",
            "first",
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
            DataType::Unsolved,
        )?;

        // List.last() -> Unsolved (will be resolved based on list element type)
        self.add_builtin_method(
            "List",
            "last",
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
            DataType::Unsolved,
        )?;

        // List.contains(item: T) -> Bool
        self.add_builtin_method_with_params(
            "List",
            "contains",
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
            vec![("item", DataType::Unsolved)],
            DataType::Bool,
        )?;

        // List.slice(start: Int, end: Int) -> List of T
        self.add_builtin_method_with_params(
            "List",
            "slice",
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
            vec![("start", DataType::Int), ("end", DataType::Int)],
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
        )?;

        // List.reverse() -> List of T
        self.add_builtin_method(
            "List",
            "reverse",
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
        )?;

        // List.join(separator: Str) -> Str (only for List of Str)
        self.add_builtin_method_with_params(
            "List",
            "join",
            DataType::List {
                element_type: Box::new(DataType::Str),
            },
            vec![("separator", DataType::Str)],
            DataType::Str,
        )?;

        // List.is_empty() -> Bool
        self.add_builtin_method(
            "List",
            "is_empty",
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
            DataType::Bool,
        )?;

        // === Map Methods ===

        // Map.keys() -> List of K
        self.add_builtin_method(
            "Map",
            "keys",
            DataType::Map {
                key_type: Box::new(DataType::Unsolved),
                value_type: Box::new(DataType::Unsolved),
            },
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
        )?;

        // Map.values() -> List of V
        self.add_builtin_method(
            "Map",
            "values",
            DataType::Map {
                key_type: Box::new(DataType::Unsolved),
                value_type: Box::new(DataType::Unsolved),
            },
            DataType::List {
                element_type: Box::new(DataType::Unsolved),
            },
        )?;

        // Map.contains_key(key: K) -> Bool
        self.add_builtin_method_with_params(
            "Map",
            "contains_key",
            DataType::Map {
                key_type: Box::new(DataType::Unsolved),
                value_type: Box::new(DataType::Unsolved),
            },
            vec![("key", DataType::Unsolved)],
            DataType::Bool,
        )?;

        // Map.is_empty() -> Bool
        self.add_builtin_method(
            "Map",
            "is_empty",
            DataType::Map {
                key_type: Box::new(DataType::Unsolved),
                value_type: Box::new(DataType::Unsolved),
            },
            DataType::Bool,
        )?;

        Ok(())
    }

    fn add_builtin_method(
        &mut self,
        type_name: &str,
        method_name: &str,
        receiver_type: DataType,
        return_type: DataType,
    ) -> Result<(), CompileError> {
        self.add_builtin_method_with_params(
            type_name,
            method_name,
            receiver_type,
            vec![],
            return_type,
        )
    }

    fn add_builtin_method_with_params(
        &mut self,
        type_name: &str,
        method_name: &str,
        receiver_type: DataType,
        additional_params: Vec<(&str, DataType)>,
        return_type: DataType,
    ) -> Result<(), CompileError> {
        let full_name = format!("{}.{}", type_name, method_name);

        // Create a self parameter
        let self_param = Param {
            name: "self".to_string(),
            data_type: receiver_type.clone(),
            default: None,
            index: (0, 0),
            copy: false,
        };

        // Create additional parameters
        let mut all_params = vec![self_param];
        for (param_name, param_type) in additional_params {
            all_params.push(Param {
                name: param_name.to_string(),
                data_type: param_type,
                default: None,
                index: (0, 0),
                copy: false,
            });
        }

        // Create a function definition for the built-in
        let builtin = BuiltinMethod::from_name(type_name, method_name);
        let func = Function {
            params: all_params,
            return_type: return_type.clone(),
            body: Box::new(Expr::Unit), // Body doesn't matter - interpreter handles it
            receiver_type: Some(type_name.to_string()),
            builtin,
        };

        let lambda = Expr::Lambda {
            value: func,
            environment: 0,
        };

        let def_func = Expr::DefineFunction {
            fn_name: full_name.clone(),
            index: (0, 0),
            value: Box::new(lambda),
        };

        // Add to global scope (scope 0)
        self.add_symbol(&full_name, def_func, 0)?;

        Ok(())
    }

    pub fn print_debug(&self) {
        for (s, scope) in self.0.iter().enumerate() {
            println!("Scope {s} ------- ");
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
            println!("Find  index for {symbol_name} in scope {current_scope_id}")
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

    pub fn lookup_type(&self, name: &str, scope: usize) -> Option<DataType> {
        // First check current scope
        if let Some(index) = self.0[scope].type_index.get(name) {
            return self.0[scope].types.get(*index).cloned();
        }

        // Then check parent scopes
        let mut current_scope = scope;
        while current_scope > 0 {
            current_scope -= 1;
            if let Some(index) = self.0[current_scope].type_index.get(name) {
                return self.0[current_scope].types.get(*index).cloned();
            }
        }

        None
    }

    pub fn scope_count(&self) -> usize {
        self.0.len()
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
            Expr::Let {
                data_type,
                value,
                mutable: _,
                ..
            } => {
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
            Expr::Lambda { value: _func, .. } => Some(DataType::Unsolved), // Functions don't have a simple type yet
            Expr::Unit => {
                // This might be a function parameter placeholder
                // Look for the parameter's type in the parent scope's function definition
                Some(DataType::Unsolved)
            }
            _ => None,
        }
    }

    pub fn is_mutable(&self, index: &(usize, usize)) -> bool {
        let expr = self
            .0
            .get(index.0)
            .and_then(|scope| scope.data.get(index.1));
        match expr {
            Some(Expr::Let { mutable, .. }) => *mutable,
            _ => false,
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
                &format!("Symbol already defined in this scope: {name}"),
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
                &format!("Type already defined in this scope: {name}"),
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
