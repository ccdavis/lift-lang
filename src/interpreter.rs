use crate::symboltable::Env;
use crate::syntax::Expr;
use crate::syntax::DataType;
use crate::syntax::AssignableData;
use crate::syntax::LiteralData;
use std::collections::HashMap;
use std::rc::Rc;


pub type ParseError = String;
pub fn add_index(envr: &mut Rc<Env>, e: &mut Expr)->Result<(), ParseError> {
    match *e {
        Expr::DefineFunction { ref fn_name, ref mut index, ref value, ..} => {            
            *index = envr.add(fn_name, &AssignableData::Lambda(value.clone()))?
        },                
        Expr::Let { ref var_name, ref value, ref data_type, ref mut index } => {            
                let data = match data_type {
                    DataType::Bool => AssignableData::Literal(LiteralData::Bool(false)),
                    DataType::Int=> AssignableData::Literal(LiteralData::Int(0)),
                    DataType::Flt => AssignableData::Literal(LiteralData::Flt(0.0)),
                    DataType::Str => AssignableData::Literal(LiteralData::Str("".to_string())),
                    DataType::List { ..} => AssignableData::ListLiteral(Vec::new()),                    
                    _ => AssignableData::Tbd(value.clone()),
                };
                *index = envr.add(var_name, &data)?
            
        },
        Expr::Block { ref mut body, ref mut symbols} => {
            symbols.parent = Some(Rc::downgrade(envr));
            for e in body {
                add_index(symbols, e)?;
            }                                    
        }        
        _ => (),
    }
    Ok(())
}



impl Expr {
    pub fn interpret(&mut self, symbols: &mut Rc<Env>) {
        let result = add_index(symbols, self);
        match result {
            Err(msg) => eprintln!("Error indexing variable and function names: {}",msg),
            _=> {}

        }

    }

    

} // impl