use std::collections::HashMap;
use  std::rc::*;
use crate::syntax;

#[derive(Clone,Debug,PartialEq)]
pub struct Env {    
    parent: Option<Rc<Env>>,
    pub data: Vec<syntax::Expr>,
    pub name: HashMap<usize, String>,
    pub index: HashMap<String,usize>,
}

impl Env {
    pub fn new(parent: Option<Rc<Env>> ) -> Self{
        Self { 
            parent, 
            data: Vec::new(),
            name: HashMap::new(),
            index: HashMap::new(),
       }
    }
}
