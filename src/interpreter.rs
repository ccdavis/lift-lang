use crate::symboltable::Env;
use crate::syntax::Expr;
use crate::syntax::*;


pub fn add_index(envr: &mut Env, e: &mut Expr) {
    match e {
        Expr::DefineFunction(_) => {
            
        },
        
        
        Expr::Let { name, value, index } => {
            envr.data.push(value.clone());
            let new_index = envr.data.size();
            index = new_index;
            envr.name[index] = name.clone();
            envr.index[name.clone()] = index
        }
        
        
        
        _ => (),
    }

}



impl Expr {
    pub fn interpret(&self, symbols: &mut Env) {

    }

    
    pub fn index_symbols(&mut self) {
        let mut vars = 0;
        match self {            
            Expr::Block { body, env} => {
                for e in &body {
                    match e {
                        Expr::Let(_) => {

                        }

                    }
                    
                    
                } // for
            }, // Block
            
            
                    
                        
                                
        } // match
    } // fn
   
} // impl