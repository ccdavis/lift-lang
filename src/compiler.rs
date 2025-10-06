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

    #[test]
    fn test_compile_if_true() {
        let mut compiler = JITCompiler::new().unwrap();

        let expr = Expr::If {
            cond: Box::new(Expr::Literal(LiteralData::Bool(true))),
            then: Box::new(Expr::Literal(LiteralData::Int(100))),
            final_else: Box::new(Expr::Literal(LiteralData::Int(200))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 100);
    }

    #[test]
    fn test_compile_if_false() {
        let mut compiler = JITCompiler::new().unwrap();

        let expr = Expr::If {
            cond: Box::new(Expr::Literal(LiteralData::Bool(false))),
            then: Box::new(Expr::Literal(LiteralData::Int(100))),
            final_else: Box::new(Expr::Literal(LiteralData::Int(200))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 200);
    }

    #[test]
    fn test_compile_if_with_comparison() {
        let mut compiler = JITCompiler::new().unwrap();

        let expr = Expr::If {
            cond: Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Literal(LiteralData::Int(5))),
                op: Operator::Gt,
                right: Box::new(Expr::Literal(LiteralData::Int(3))),
            }),
            then: Box::new(Expr::Literal(LiteralData::Int(42))),
            final_else: Box::new(Expr::Literal(LiteralData::Int(0))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_compile_comparison() {
        let mut compiler = JITCompiler::new().unwrap();

        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Literal(LiteralData::Int(10))),
            op: Operator::Gt,
            right: Box::new(Expr::Literal(LiteralData::Int(5))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 1); // true is represented as 1
    }

    #[test]
    fn test_compile_variable() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let x = 42; x }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "x".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Int,
                    value: Box::new(Expr::Literal(LiteralData::Int(42))),
                    mutable: false,
                },
                Expr::Variable {
                    name: "x".to_string(),
                    index: (0, 0),
                },
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_compile_variable_arithmetic() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let x = 10; let y = 20; x + y }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "x".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Int,
                    value: Box::new(Expr::Literal(LiteralData::Int(10))),
                    mutable: false,
                },
                Expr::Let {
                    var_name: "y".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Int,
                    value: Box::new(Expr::Literal(LiteralData::Int(20))),
                    mutable: false,
                },
                Expr::BinaryExpr {
                    left: Box::new(Expr::Variable {
                        name: "x".to_string(),
                        index: (0, 0),
                    }),
                    op: Operator::Add,
                    right: Box::new(Expr::Variable {
                        name: "y".to_string(),
                        index: (0, 0),
                    }),
                },
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 30);
    }

    #[test]
    fn test_compile_mutable_variable() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let var x = 5; x := 10; x }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "x".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Int,
                    value: Box::new(Expr::Literal(LiteralData::Int(5))),
                    mutable: true,
                },
                Expr::Assign {
                    name: "x".to_string(),
                    value: Box::new(Expr::Literal(LiteralData::Int(10))),
                    index: (0, 0),
                },
                Expr::Variable {
                    name: "x".to_string(),
                    index: (0, 0),
                },
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_compile_variable_in_if() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let x = 5; if x > 3 { 100 } else { 200 } }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "x".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Int,
                    value: Box::new(Expr::Literal(LiteralData::Int(5))),
                    mutable: false,
                },
                Expr::If {
                    cond: Box::new(Expr::BinaryExpr {
                        left: Box::new(Expr::Variable {
                            name: "x".to_string(),
                            index: (0, 0),
                        }),
                        op: Operator::Gt,
                        right: Box::new(Expr::Literal(LiteralData::Int(3))),
                    }),
                    then: Box::new(Expr::Literal(LiteralData::Int(100))),
                    final_else: Box::new(Expr::Literal(LiteralData::Int(200))),
                },
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 100);
    }
}
