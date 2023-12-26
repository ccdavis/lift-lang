use crate::semantic_analysis::ParseError;
use crate::semantic_analysis::*;
use crate::symboltable::SymbolTable;
use crate::syntax::Expr;

impl Expr {
    pub fn interpret(&mut self, symbols: &mut SymbolTable) -> Result<(), Vec<ParseError>> {
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
} // impl
