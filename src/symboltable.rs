use crate::syntax::Expr;
use crate::syntax::AssignableData;
use std::collections::HashMap;
use std::rc::*;

#[derive(Clone, Debug)]
pub struct Env {
    pub parent:Option<Weak<Env>>,
    pub data: Vec<AssignableData>,
    pub name: HashMap<usize, String>,
    pub index: HashMap<String, usize>,
}

impl Env {
    pub fn new(parent: Option<Weak<Env>>) -> Self {
        Self {
            parent,
            data: Vec::new(),
            name: HashMap::new(),
            index: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: &str, value: &AssignableData) -> Result<usize, String> {
        if self.index.contains_key(name) {
            Err(format!("Symbol already defined in this scope: {}", name))
        } else {
            self.data.push(value.clone());
            let new_index = self.data.len();
            self.index.insert(name.to_string(), new_index);
            self.name.insert(new_index, name.to_string());
            Ok(new_index)
        }
    }
}
