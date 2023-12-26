use crate::semantic_analysis::ParseError;
use crate::semantic_analysis::*;
use crate::symboltable::SymbolTable;
use crate::syntax::Expr;

// TODO this should eventually  store line numbers, columns in source and function names
struct RuntimeError {
    stack: Vec<String>,
    msg: String,
}

impl Expr {
    pub fn prepare(&mut self, symbols: &mut SymbolTable) -> Result<(), Vec<ParseError>> {
        let mut errors = Vec::new();

        // Analyze  parse tree to index symbols across scopes.
        let result = add_symbols(self, symbols, 0);
        if let Err(ref msg) = result {
            eprintln!("Error indexing variable and function names: {}", msg);
            errors.push(msg.to_string());
        }
        // Collect other errors...

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    // Receives a "prepared" parse tree and symbol table.
    pub fn interpret(&self, symbols: &mut SymbolTable) -> Result<Option<Expr>,RuntimeError> {
        match self {
            Expr::Program{ body, environment} =>{
                let mut eval_to: Option<Expr> = None;
                let mut expressions =  body.iter().peekable();
                while let Some(expression) = expressions.next() {
                    let tmp_eval_to = match expression.interpret(symbols) {
                        Ok(maybe_expr)=> maybe_expr,
                        Err(rt) => {
                            eprintln!("Runtime error: {}",rt.msg);                            
                            return Err(rt);
                        }

                    };
                    if expressions.peek().is_none() {
                        eval_to = tmp_eval_to
                    }

                }
                Ok(eval_to)
            },
            _ => Ok(()),
        }
                
    }
} // impl
