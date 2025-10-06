// JIT Compiler for Lift language using Cranelift

use crate::codegen::CodeGenerator;
use crate::runtime;
use crate::syntax::Expr;
use crate::symboltable::SymbolTable;
use cranelift_jit::{JITBuilder, JITModule};
use std::error::Error;

pub struct JITCompiler {
    module: JITModule,
}

impl JITCompiler {
    /// Create a new JIT compiler with runtime functions registered
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Set up the JIT builder
        let mut builder = JITBuilder::new(cranelift_module::default_libcall_names())?;

        // Register runtime functions
        builder.symbol("lift_output_int", runtime::lift_output_int as *const u8);
        builder.symbol("lift_output_float", runtime::lift_output_float as *const u8);
        builder.symbol("lift_output_bool", runtime::lift_output_bool as *const u8);
        builder.symbol("lift_output_str", runtime::lift_output_str as *const u8);
        builder.symbol("lift_str_new", runtime::lift_str_new as *const u8);
        builder.symbol("lift_str_concat", runtime::lift_str_concat as *const u8);
        builder.symbol("lift_str_len", runtime::lift_str_len as *const u8);
        builder.symbol("lift_str_eq", runtime::lift_str_eq as *const u8);

        // Create the JIT module
        let module = JITModule::new(builder);

        Ok(Self { module })
    }

    /// Compile and execute a Lift program
    pub fn compile_and_run(
        &mut self,
        expr: &Expr,
        symbols: &SymbolTable,
    ) -> Result<i64, Box<dyn Error>> {
        // Create code generator
        let mut codegen = CodeGenerator::new(&mut self.module);

        // Declare runtime functions in the module
        codegen.declare_runtime_functions()?;

        // Compile the program
        let func_id = codegen.compile_program(expr, symbols)?;

        // Finalize the module (perform linking)
        self.module.finalize_definitions()?;

        // Get the function pointer
        let code = self.module.get_finalized_function(func_id);

        // Cast to function pointer and execute
        let main_fn = unsafe { std::mem::transmute::<_, fn() -> i64>(code) };
        let result = main_fn();

        Ok(result)
    }
}

impl Default for JITCompiler {
    fn default() -> Self {
        Self::new().expect("Failed to create JIT compiler")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{LiteralData, Operator};

    #[test]
    fn test_compile_simple_literal() {
        let mut compiler = JITCompiler::new().unwrap();
        let expr = Expr::Literal(LiteralData::Int(42));
        let mut symbols = SymbolTable::new();

        // Prepare the expression
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_compile_simple_addition() {
        let mut compiler = JITCompiler::new().unwrap();

        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Literal(LiteralData::Int(10))),
            op: Operator::Add,
            right: Box::new(Expr::Literal(LiteralData::Int(32))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_compile_complex_arithmetic() {
        let mut compiler = JITCompiler::new().unwrap();

        // (10 + 5) * 2 = 30
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Literal(LiteralData::Int(10))),
                op: Operator::Add,
                right: Box::new(Expr::Literal(LiteralData::Int(5))),
            }),
            op: Operator::Mul,
            right: Box::new(Expr::Literal(LiteralData::Int(2))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 30);
    }
}
