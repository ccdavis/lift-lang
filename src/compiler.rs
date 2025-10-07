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
        builder.symbol("lift_list_new", runtime::lift_list_new as *const u8);
        builder.symbol("lift_list_set", runtime::lift_list_set as *const u8);
        builder.symbol("lift_list_get", runtime::lift_list_get as *const u8);
        builder.symbol("lift_list_len", runtime::lift_list_len as *const u8);
        builder.symbol("lift_map_new", runtime::lift_map_new as *const u8);
        builder.symbol("lift_map_set", runtime::lift_map_set as *const u8);
        builder.symbol("lift_map_get", runtime::lift_map_get as *const u8);
        builder.symbol("lift_map_len", runtime::lift_map_len as *const u8);
        builder.symbol("lift_range_new", runtime::lift_range_new as *const u8);
        builder.symbol("lift_range_start", runtime::lift_range_start as *const u8);
        builder.symbol("lift_range_end", runtime::lift_range_end as *const u8);
        builder.symbol("lift_output_range", runtime::lift_output_range as *const u8);

        // String method symbols
        builder.symbol("lift_str_upper", runtime::lift_str_upper as *const u8);
        builder.symbol("lift_str_lower", runtime::lift_str_lower as *const u8);
        builder.symbol("lift_str_substring", runtime::lift_str_substring as *const u8);
        builder.symbol("lift_str_contains", runtime::lift_str_contains as *const u8);
        builder.symbol("lift_str_trim", runtime::lift_str_trim as *const u8);
        builder.symbol("lift_str_split", runtime::lift_str_split as *const u8);
        builder.symbol("lift_str_replace", runtime::lift_str_replace as *const u8);
        builder.symbol("lift_str_starts_with", runtime::lift_str_starts_with as *const u8);
        builder.symbol("lift_str_ends_with", runtime::lift_str_ends_with as *const u8);
        builder.symbol("lift_str_is_empty", runtime::lift_str_is_empty as *const u8);

        // List method symbols
        builder.symbol("lift_list_first", runtime::lift_list_first as *const u8);
        builder.symbol("lift_list_last", runtime::lift_list_last as *const u8);
        builder.symbol("lift_list_contains", runtime::lift_list_contains as *const u8);
        builder.symbol("lift_list_slice", runtime::lift_list_slice as *const u8);
        builder.symbol("lift_list_reverse", runtime::lift_list_reverse as *const u8);
        builder.symbol("lift_list_join", runtime::lift_list_join as *const u8);
        builder.symbol("lift_list_is_empty", runtime::lift_list_is_empty as *const u8);

        // Map method symbols
        builder.symbol("lift_map_keys", runtime::lift_map_keys as *const u8);
        builder.symbol("lift_map_values", runtime::lift_map_values as *const u8);
        builder.symbol("lift_map_contains_key", runtime::lift_map_contains_key as *const u8);
        builder.symbol("lift_map_is_empty", runtime::lift_map_is_empty as *const u8);

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

    #[test]
    fn test_compile_output_int() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output(42); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::Literal(LiteralData::Int(42))],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compile_output_bool() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output(true); output(false); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::Literal(LiteralData::Bool(true))],
                },
                Expr::Output {
                    data: vec![Expr::Literal(LiteralData::Bool(false))],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compile_output_float() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output(3.14); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::Literal(LiteralData::Flt(3.14))],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compile_output_expression() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output(2 + 3); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::BinaryExpr {
                        left: Box::new(Expr::Literal(LiteralData::Int(2))),
                        op: Operator::Add,
                        right: Box::new(Expr::Literal(LiteralData::Int(3))),
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compile_string_literal() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output('Hello'); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::Literal(LiteralData::Str("Hello".into()))],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compile_string_concat() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output('Hello' + ' ' + 'World'); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::BinaryExpr {
                        left: Box::new(Expr::BinaryExpr {
                            left: Box::new(Expr::Literal(LiteralData::Str("Hello".into()))),
                            op: Operator::Add,
                            right: Box::new(Expr::Literal(LiteralData::Str(" ".into()))),
                        }),
                        op: Operator::Add,
                        right: Box::new(Expr::Literal(LiteralData::Str("World".into()))),
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_compile_string_equality() {
        let mut compiler = JITCompiler::new().unwrap();

        // { if 'Hello' = 'Hello' { 1 } else { 0 } }
        let expr = Expr::Block {
            body: vec![Expr::If {
                cond: Box::new(Expr::BinaryExpr {
                    left: Box::new(Expr::Literal(LiteralData::Str("Hello".into()))),
                    op: Operator::Eq,
                    right: Box::new(Expr::Literal(LiteralData::Str("Hello".into()))),
                }),
                then: Box::new(Expr::Literal(LiteralData::Int(1))),
                final_else: Box::new(Expr::Literal(LiteralData::Int(0))),
            }],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_compile_string_inequality() {
        let mut compiler = JITCompiler::new().unwrap();

        // { if 'Hello' <> 'World' { 1 } else { 0 } }
        let expr = Expr::Block {
            body: vec![Expr::If {
                cond: Box::new(Expr::BinaryExpr {
                    left: Box::new(Expr::Literal(LiteralData::Str("Hello".into()))),
                    op: Operator::Neq,
                    right: Box::new(Expr::Literal(LiteralData::Str("World".into()))),
                }),
                then: Box::new(Expr::Literal(LiteralData::Int(1))),
                final_else: Box::new(Expr::Literal(LiteralData::Int(0))),
            }],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_compile_list_literal() {
        let mut compiler = JITCompiler::new().unwrap();

        // [10, 20, 30]
        let expr = Expr::ListLiteral {
            data_type: crate::syntax::DataType::Int,
            data: vec![
                Expr::Literal(LiteralData::Int(10)),
                Expr::Literal(LiteralData::Int(20)),
                Expr::Literal(LiteralData::Int(30)),
            ],
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let _result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        // List returns pointer, so we can't easily assert the value
        // Just verify it compiles and runs
    }

    #[test]
    fn test_compile_list_indexing() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let nums = [10, 20, 30]; nums[1] }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "nums".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::List {
                        element_type: Box::new(crate::syntax::DataType::Int),
                    },
                    value: Box::new(Expr::ListLiteral {
                        data_type: crate::syntax::DataType::Int,
                        data: vec![
                            Expr::Literal(LiteralData::Int(10)),
                            Expr::Literal(LiteralData::Int(20)),
                            Expr::Literal(LiteralData::Int(30)),
                        ],
                    }),
                    mutable: false,
                },
                Expr::Index {
                    expr: Box::new(Expr::Variable {
                        name: "nums".to_string(),
                        index: (0, 0),
                    }),
                    index: Box::new(Expr::Literal(LiteralData::Int(1))),
                },
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 20);
    }

    #[test]
    fn test_compile_map_literal() {
        let mut compiler = JITCompiler::new().unwrap();

        // #{1: 100, 2: 200}
        let expr = Expr::MapLiteral {
            key_type: crate::syntax::DataType::Int,
            value_type: crate::syntax::DataType::Int,
            data: vec![
                (crate::syntax::KeyData::Int(1), Expr::Literal(LiteralData::Int(100))),
                (crate::syntax::KeyData::Int(2), Expr::Literal(LiteralData::Int(200))),
            ],
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let _result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        // Map returns pointer, so we can't easily assert the value
        // Just verify it compiles and runs
    }

    #[test]
    fn test_compile_map_indexing() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let ages = #{1: 25, 2: 30}; ages[2] }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "ages".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Map {
                        key_type: Box::new(crate::syntax::DataType::Int),
                        value_type: Box::new(crate::syntax::DataType::Int),
                    },
                    value: Box::new(Expr::MapLiteral {
                        key_type: crate::syntax::DataType::Int,
                        value_type: crate::syntax::DataType::Int,
                        data: vec![
                            (crate::syntax::KeyData::Int(1), Expr::Literal(LiteralData::Int(25))),
                            (crate::syntax::KeyData::Int(2), Expr::Literal(LiteralData::Int(30))),
                        ],
                    }),
                    mutable: false,
                },
                Expr::Index {
                    expr: Box::new(Expr::Variable {
                        name: "ages".to_string(),
                        index: (0, 0),
                    }),
                    index: Box::new(Expr::Literal(LiteralData::Int(2))),
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
    fn test_compile_list_with_expressions() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let nums = [1 + 1, 2 * 3, 10 - 4]; nums[2] }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "nums".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::List {
                        element_type: Box::new(crate::syntax::DataType::Int),
                    },
                    value: Box::new(Expr::ListLiteral {
                        data_type: crate::syntax::DataType::Int,
                        data: vec![
                            Expr::BinaryExpr {
                                left: Box::new(Expr::Literal(LiteralData::Int(1))),
                                op: Operator::Add,
                                right: Box::new(Expr::Literal(LiteralData::Int(1))),
                            },
                            Expr::BinaryExpr {
                                left: Box::new(Expr::Literal(LiteralData::Int(2))),
                                op: Operator::Mul,
                                right: Box::new(Expr::Literal(LiteralData::Int(3))),
                            },
                            Expr::BinaryExpr {
                                left: Box::new(Expr::Literal(LiteralData::Int(10))),
                                op: Operator::Sub,
                                right: Box::new(Expr::Literal(LiteralData::Int(4))),
                            },
                        ],
                    }),
                    mutable: false,
                },
                Expr::Index {
                    expr: Box::new(Expr::Variable {
                        name: "nums".to_string(),
                        index: (0, 0),
                    }),
                    index: Box::new(Expr::Literal(LiteralData::Int(2))),
                },
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 6); // 10 - 4 = 6
    }

    #[test]
    fn test_compile_list_bool() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let flags = [true, false, true]; if flags[1] { 100 } else { 200 } }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "flags".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::List {
                        element_type: Box::new(crate::syntax::DataType::Bool),
                    },
                    value: Box::new(Expr::ListLiteral {
                        data_type: crate::syntax::DataType::Bool,
                        data: vec![
                            Expr::Literal(LiteralData::Bool(true)),
                            Expr::Literal(LiteralData::Bool(false)),
                            Expr::Literal(LiteralData::Bool(true)),
                        ],
                    }),
                    mutable: false,
                },
                Expr::If {
                    cond: Box::new(Expr::Index {
                        expr: Box::new(Expr::Variable {
                            name: "flags".to_string(),
                            index: (0, 0),
                        }),
                        index: Box::new(Expr::Literal(LiteralData::Int(1))),
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
        assert_eq!(result, 200); // flags[1] is false, so else branch
    }

    #[test]
    fn test_compile_list_string() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let names = ['Alice', 'Bob', 'Carol']; output(names[1]); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "names".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::List {
                        element_type: Box::new(crate::syntax::DataType::Str),
                    },
                    value: Box::new(Expr::ListLiteral {
                        data_type: crate::syntax::DataType::Str,
                        data: vec![
                            Expr::Literal(LiteralData::Str("Alice".into())),
                            Expr::Literal(LiteralData::Str("Bob".into())),
                            Expr::Literal(LiteralData::Str("Carol".into())),
                        ],
                    }),
                    mutable: false,
                },
                Expr::Output {
                    data: vec![Expr::Index {
                        expr: Box::new(Expr::Variable {
                            name: "names".to_string(),
                            index: (0, 0),
                        }),
                        index: Box::new(Expr::Literal(LiteralData::Int(1))),
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should print "Bob"
    }

    // NOTE: String keys for maps don't work yet because the runtime uses pointer equality
    // rather than string content equality. This would require string interning or
    // calling lift_str_eq in the map lookup. Leaving as a future enhancement.
    // #[test]
    // fn test_compile_map_string_keys() { ... }

    #[test]
    fn test_compile_map_bool_keys() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let vals = #{true: 100, false: 200}; vals[false] }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "vals".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Map {
                        key_type: Box::new(crate::syntax::DataType::Bool),
                        value_type: Box::new(crate::syntax::DataType::Int),
                    },
                    value: Box::new(Expr::MapLiteral {
                        key_type: crate::syntax::DataType::Bool,
                        value_type: crate::syntax::DataType::Int,
                        data: vec![
                            (crate::syntax::KeyData::Bool(true), Expr::Literal(LiteralData::Int(100))),
                            (crate::syntax::KeyData::Bool(false), Expr::Literal(LiteralData::Int(200))),
                        ],
                    }),
                    mutable: false,
                },
                Expr::Index {
                    expr: Box::new(Expr::Variable {
                        name: "vals".to_string(),
                        index: (0, 0),
                    }),
                    index: Box::new(Expr::Literal(LiteralData::Bool(false))),
                },
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 200);
    }

    #[test]
    fn test_compile_len_string() {
        let mut compiler = JITCompiler::new().unwrap();

        // len('Hello')
        let expr = Expr::Len {
            expr: Box::new(Expr::Literal(LiteralData::Str("Hello".into()))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_compile_len_list() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let nums = [10, 20, 30]; len(nums) }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "nums".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::List {
                        element_type: Box::new(crate::syntax::DataType::Int),
                    },
                    value: Box::new(Expr::ListLiteral {
                        data_type: crate::syntax::DataType::Int,
                        data: vec![
                            Expr::Literal(LiteralData::Int(10)),
                            Expr::Literal(LiteralData::Int(20)),
                            Expr::Literal(LiteralData::Int(30)),
                        ],
                    }),
                    mutable: false,
                },
                Expr::Len {
                    expr: Box::new(Expr::Variable {
                        name: "nums".to_string(),
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
        assert_eq!(result, 3);
    }

    #[test]
    fn test_compile_len_map() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let ages = #{1: 25, 2: 30, 3: 35}; len(ages) }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "ages".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Map {
                        key_type: Box::new(crate::syntax::DataType::Int),
                        value_type: Box::new(crate::syntax::DataType::Int),
                    },
                    value: Box::new(Expr::MapLiteral {
                        key_type: crate::syntax::DataType::Int,
                        value_type: crate::syntax::DataType::Int,
                        data: vec![
                            (crate::syntax::KeyData::Int(1), Expr::Literal(LiteralData::Int(25))),
                            (crate::syntax::KeyData::Int(2), Expr::Literal(LiteralData::Int(30))),
                            (crate::syntax::KeyData::Int(3), Expr::Literal(LiteralData::Int(35))),
                        ],
                    }),
                    mutable: false,
                },
                Expr::Len {
                    expr: Box::new(Expr::Variable {
                        name: "ages".to_string(),
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
        assert_eq!(result, 3);
    }

    #[test]
    fn test_compile_and_operator_true_true() {
        let mut compiler = JITCompiler::new().unwrap();

        // true and true = true (1)
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Literal(LiteralData::Bool(true))),
            op: Operator::And,
            right: Box::new(Expr::Literal(LiteralData::Bool(true))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 1); // true
    }

    #[test]
    fn test_compile_and_operator_true_false() {
        let mut compiler = JITCompiler::new().unwrap();

        // true and false = false (0)
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Literal(LiteralData::Bool(true))),
            op: Operator::And,
            right: Box::new(Expr::Literal(LiteralData::Bool(false))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0); // false
    }

    #[test]
    fn test_compile_and_operator_false_false() {
        let mut compiler = JITCompiler::new().unwrap();

        // false and false = false (0)
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Literal(LiteralData::Bool(false))),
            op: Operator::And,
            right: Box::new(Expr::Literal(LiteralData::Bool(false))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0); // false
    }

    #[test]
    fn test_compile_or_operator_true_false() {
        let mut compiler = JITCompiler::new().unwrap();

        // true or false = true (1)
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Literal(LiteralData::Bool(true))),
            op: Operator::Or,
            right: Box::new(Expr::Literal(LiteralData::Bool(false))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 1); // true
    }

    #[test]
    fn test_compile_or_operator_false_false() {
        let mut compiler = JITCompiler::new().unwrap();

        // false or false = false (0)
        let expr = Expr::BinaryExpr {
            left: Box::new(Expr::Literal(LiteralData::Bool(false))),
            op: Operator::Or,
            right: Box::new(Expr::Literal(LiteralData::Bool(false))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0); // false
    }

    #[test]
    fn test_compile_logical_in_if_condition() {
        let mut compiler = JITCompiler::new().unwrap();

        // if (5 > 3) and (10 < 20) { 100 } else { 200 }
        let expr = Expr::If {
            cond: Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::BinaryExpr {
                    left: Box::new(Expr::Literal(LiteralData::Int(5))),
                    op: Operator::Gt,
                    right: Box::new(Expr::Literal(LiteralData::Int(3))),
                }),
                op: Operator::And,
                right: Box::new(Expr::BinaryExpr {
                    left: Box::new(Expr::Literal(LiteralData::Int(10))),
                    op: Operator::Lt,
                    right: Box::new(Expr::Literal(LiteralData::Int(20))),
                }),
            }),
            then: Box::new(Expr::Literal(LiteralData::Int(100))),
            final_else: Box::new(Expr::Literal(LiteralData::Int(200))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 100); // Both conditions true, so then branch
    }

    #[test]
    fn test_compile_logical_or_in_if_condition() {
        let mut compiler = JITCompiler::new().unwrap();

        // if (5 < 3) or (10 < 20) { 100 } else { 200 }
        let expr = Expr::If {
            cond: Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::BinaryExpr {
                    left: Box::new(Expr::Literal(LiteralData::Int(5))),
                    op: Operator::Lt,
                    right: Box::new(Expr::Literal(LiteralData::Int(3))),
                }),
                op: Operator::Or,
                right: Box::new(Expr::BinaryExpr {
                    left: Box::new(Expr::Literal(LiteralData::Int(10))),
                    op: Operator::Lt,
                    right: Box::new(Expr::Literal(LiteralData::Int(20))),
                }),
            }),
            then: Box::new(Expr::Literal(LiteralData::Int(100))),
            final_else: Box::new(Expr::Literal(LiteralData::Int(200))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 100); // Second condition true, so then branch
    }

    #[test]
    fn test_compile_float_addition() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output(3.14 + 2.86); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::BinaryExpr {
                        left: Box::new(Expr::Literal(LiteralData::Flt(3.14))),
                        op: Operator::Add,
                        right: Box::new(Expr::Literal(LiteralData::Flt(2.86))),
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output 6.0
    }

    #[test]
    fn test_compile_float_arithmetic() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output(10.5 - 2.5); output(4.0 * 2.5); output(9.0 / 3.0); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::BinaryExpr {
                        left: Box::new(Expr::Literal(LiteralData::Flt(10.5))),
                        op: Operator::Sub,
                        right: Box::new(Expr::Literal(LiteralData::Flt(2.5))),
                    }],
                },
                Expr::Output {
                    data: vec![Expr::BinaryExpr {
                        left: Box::new(Expr::Literal(LiteralData::Flt(4.0))),
                        op: Operator::Mul,
                        right: Box::new(Expr::Literal(LiteralData::Flt(2.5))),
                    }],
                },
                Expr::Output {
                    data: vec![Expr::BinaryExpr {
                        left: Box::new(Expr::Literal(LiteralData::Flt(9.0))),
                        op: Operator::Div,
                        right: Box::new(Expr::Literal(LiteralData::Flt(3.0))),
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output 8.0, 10.0, 3.0
    }

    #[test]
    fn test_compile_float_comparison_gt() {
        let mut compiler = JITCompiler::new().unwrap();

        // if 3.5 > 2.1 { 1 } else { 0 }
        let expr = Expr::If {
            cond: Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Literal(LiteralData::Flt(3.5))),
                op: Operator::Gt,
                right: Box::new(Expr::Literal(LiteralData::Flt(2.1))),
            }),
            then: Box::new(Expr::Literal(LiteralData::Int(1))),
            final_else: Box::new(Expr::Literal(LiteralData::Int(0))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 1); // true
    }

    #[test]
    fn test_compile_float_comparison_eq() {
        let mut compiler = JITCompiler::new().unwrap();

        // if 3.0 = 3.0 { 1 } else { 0 }
        let expr = Expr::If {
            cond: Box::new(Expr::BinaryExpr {
                left: Box::new(Expr::Literal(LiteralData::Flt(3.0))),
                op: Operator::Eq,
                right: Box::new(Expr::Literal(LiteralData::Flt(3.0))),
            }),
            then: Box::new(Expr::Literal(LiteralData::Int(1))),
            final_else: Box::new(Expr::Literal(LiteralData::Int(0))),
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 1); // true
    }

    #[test]
    fn test_compile_float_in_variable() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let pi = 3.14159; let radius = 2.0; output(pi * radius * radius); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "pi".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Flt,
                    value: Box::new(Expr::Literal(LiteralData::Flt(3.14159))),
                    mutable: false,
                },
                Expr::Let {
                    var_name: "radius".to_string(),
                    index: (0, 0),
                    data_type: crate::syntax::DataType::Flt,
                    value: Box::new(Expr::Literal(LiteralData::Flt(2.0))),
                    mutable: false,
                },
                Expr::Output {
                    data: vec![Expr::BinaryExpr {
                        left: Box::new(Expr::BinaryExpr {
                            left: Box::new(Expr::Variable {
                                name: "pi".to_string(),
                                index: (0, 0),
                            }),
                            op: Operator::Mul,
                            right: Box::new(Expr::Variable {
                                name: "radius".to_string(),
                                index: (0, 0),
                            }),
                        }),
                        op: Operator::Mul,
                        right: Box::new(Expr::Variable {
                            name: "radius".to_string(),
                            index: (0, 0),
                        }),
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output approximately 12.56636 (pi * r^2)
    }

    #[test]
    fn test_compile_range_literal() {
        let mut compiler = JITCompiler::new().unwrap();

        // { output(1..10); 0 }
        let expr = Expr::Block {
            body: vec![
                Expr::Output {
                    data: vec![Expr::Range(
                        LiteralData::Int(1),
                        LiteralData::Int(10),
                    )],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output 1..10
    }

    #[test]
    fn test_compile_range_with_variables() {
        let mut compiler = JITCompiler::new().unwrap();

        // { let start = 5; let end = 15; let r = start..end; output(r); 0 }
        use crate::syntax::DataType;
        let expr = Expr::Block {
            body: vec![
                Expr::Let {
                    var_name: "start".to_string(),
                    index: (0, 0),
                    data_type: DataType::Int,
                    value: Box::new(Expr::Literal(LiteralData::Int(5))),
                    mutable: false,
                },
                Expr::Let {
                    var_name: "end".to_string(),
                    index: (0, 0),
                    data_type: DataType::Int,
                    value: Box::new(Expr::Literal(LiteralData::Int(15))),
                    mutable: false,
                },
                Expr::Let {
                    var_name: "r".to_string(),
                    index: (0, 0),
                    data_type: DataType::Range(Box::new(Expr::Range(
                        LiteralData::Int(0),
                        LiteralData::Int(0)
                    ))),
                    value: Box::new(Expr::BinaryExpr {
                        left: Box::new(Expr::Variable {
                            name: "start".to_string(),
                            index: (0, 0),
                        }),
                        op: Operator::Range,
                        right: Box::new(Expr::Variable {
                            name: "end".to_string(),
                            index: (0, 0),
                        }),
                    }),
                    mutable: false,
                },
                Expr::Output {
                    data: vec![Expr::Variable {
                        name: "r".to_string(),
                        index: (0, 0),
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output 5..15
    }

    #[test]
    fn test_compile_simple_function() {
        use crate::syntax::{DataType, Function, Param};

        let mut compiler = JITCompiler::new().unwrap();

        // function get_five(): Int { 5 };
        // output(get_five());
        // 0
        let expr = Expr::Program {
            body: vec![
                // Define function
                Expr::DefineFunction {
                    fn_name: "get_five".to_string(),
                    index: (0, 0),
                    value: Box::new(Expr::Lambda {
                        value: Function {
                            params: vec![],
                            return_type: DataType::Int,
                            body: Box::new(Expr::Literal(LiteralData::Int(5))),
                            receiver_type: None,
                            builtin: None,
                        },
                        environment: 0,
                    }),
                },
                // Call function and output
                Expr::Output {
                    data: vec![Expr::Call {
                        fn_name: "get_five".to_string(),
                        index: (0, 0),
                        args: vec![],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: 5
    }

    #[test]
    fn test_compile_function_with_params() {
        use crate::syntax::{DataType, Function, KeywordArg, Param};

        let mut compiler = JITCompiler::new().unwrap();

        // function add(x: Int, y: Int): Int { x + y };
        // output(add(x: 3, y: 4));
        // 0
        let expr = Expr::Program {
            body: vec![
                // Define function
                Expr::DefineFunction {
                    fn_name: "add".to_string(),
                    index: (0, 0),
                    value: Box::new(Expr::Lambda {
                        value: Function {
                            params: vec![
                                Param {
                                    name: "x".to_string(),
                                    data_type: DataType::Int,
                                    default: None,
                                    index: (0, 0),
                                    copy: false,
                                },
                                Param {
                                    name: "y".to_string(),
                                    data_type: DataType::Int,
                                    default: None,
                                    index: (0, 0),
                                    copy: false,
                                },
                            ],
                            return_type: DataType::Int,
                            body: Box::new(Expr::BinaryExpr {
                                left: Box::new(Expr::Variable {
                                    name: "x".to_string(),
                                    index: (0, 0),
                                }),
                                op: Operator::Add,
                                right: Box::new(Expr::Variable {
                                    name: "y".to_string(),
                                    index: (0, 0),
                                }),
                            }),
                            receiver_type: None,
                            builtin: None,
                        },
                        environment: 0,
                    }),
                },
                // Call function and output
                Expr::Output {
                    data: vec![Expr::Call {
                        fn_name: "add".to_string(),
                        index: (0, 0),
                        args: vec![
                            KeywordArg {
                                name: "x".to_string(),
                                value: Expr::Literal(LiteralData::Int(3)),
                            },
                            KeywordArg {
                                name: "y".to_string(),
                                value: Expr::Literal(LiteralData::Int(4)),
                            },
                        ],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: 7
    }

    #[test]
    fn test_compile_recursive_factorial() {
        use crate::syntax::{DataType, Function, KeywordArg, Param};

        let mut compiler = JITCompiler::new().unwrap();

        // function factorial(n: Int): Int {
        //     if n <= 1 { 1 } else { n * factorial(n: n - 1) }
        // };
        // output(factorial(n: 5));
        // 0
        let expr = Expr::Program {
            body: vec![
                // Define recursive function
                Expr::DefineFunction {
                    fn_name: "factorial".to_string(),
                    index: (0, 0),
                    value: Box::new(Expr::Lambda {
                        value: Function {
                            params: vec![Param {
                                name: "n".to_string(),
                                data_type: DataType::Int,
                                default: None,
                                index: (0, 0),
                                copy: false,
                            }],
                            return_type: DataType::Int,
                            body: Box::new(Expr::If {
                                cond: Box::new(Expr::BinaryExpr {
                                    left: Box::new(Expr::Variable {
                                        name: "n".to_string(),
                                        index: (0, 0),
                                    }),
                                    op: Operator::Lte,
                                    right: Box::new(Expr::Literal(LiteralData::Int(1))),
                                }),
                                then: Box::new(Expr::Literal(LiteralData::Int(1))),
                                final_else: Box::new(Expr::BinaryExpr {
                                    left: Box::new(Expr::Variable {
                                        name: "n".to_string(),
                                        index: (0, 0),
                                    }),
                                    op: Operator::Mul,
                                    right: Box::new(Expr::Call {
                                        fn_name: "factorial".to_string(),
                                        index: (0, 0),
                                        args: vec![KeywordArg {
                                            name: "n".to_string(),
                                            value: Expr::BinaryExpr {
                                                left: Box::new(Expr::Variable {
                                                    name: "n".to_string(),
                                                    index: (0, 0),
                                                }),
                                                op: Operator::Sub,
                                                right: Box::new(Expr::Literal(LiteralData::Int(1))),
                                            },
                                        }],
                                    }),
                                }),
                            }),
                            receiver_type: None,
                            builtin: None,
                        },
                        environment: 0,
                    }),
                },
                // Call function and output
                Expr::Output {
                    data: vec![Expr::Call {
                        fn_name: "factorial".to_string(),
                        index: (0, 0),
                        args: vec![KeywordArg {
                            name: "n".to_string(),
                            value: Expr::Literal(LiteralData::Int(5)),
                        }],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: 120
    }

    #[test]
    fn test_compile_str_upper() {
        use crate::syntax::{DataType, Function, KeywordArg};

        let mut compiler = JITCompiler::new().unwrap();

        // let s = 'hello';
        // output(s.upper());
        // 0
        let expr = Expr::Program {
            body: vec![
                Expr::Let {
                    var_name: "s".to_string(),
                    value: Box::new(Expr::Literal(LiteralData::Str("'hello'".into()))),
                    mutable: false,
                    index: (0, 0),
                    data_type: DataType::Str,
                },
                Expr::Output {
                    data: vec![Expr::MethodCall {
                        receiver: Box::new(Expr::Variable {
                            name: "s".to_string(),
                            index: (0, 0),
                        }),
                        method_name: "upper".to_string(),
                        fn_index: (0, 0),
                        args: vec![],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: HELLO
    }

    #[test]
    fn test_compile_str_substring() {
        use crate::syntax::KeywordArg;

        let mut compiler = JITCompiler::new().unwrap();

        // output('Hello World'.substring(start: 0, end: 5));
        // 0
        let expr = Expr::Program {
            body: vec![
                Expr::Output {
                    data: vec![Expr::MethodCall {
                        receiver: Box::new(Expr::Literal(LiteralData::Str("'Hello World'".into()))),
                        method_name: "substring".to_string(),
                        fn_index: (0, 0),
                        args: vec![
                            KeywordArg {
                                name: "start".to_string(),
                                value: Expr::Literal(LiteralData::Int(0)),
                            },
                            KeywordArg {
                                name: "end".to_string(),
                                value: Expr::Literal(LiteralData::Int(5)),
                            },
                        ],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: Hello
    }

    #[test]
    fn test_compile_list_first() {
        use crate::syntax::DataType;

        let mut compiler = JITCompiler::new().unwrap();

        // output([10, 20, 30].first());
        // 0
        let expr = Expr::Program {
            body: vec![
                Expr::Output {
                    data: vec![Expr::MethodCall {
                        receiver: Box::new(Expr::ListLiteral {
                            data_type: DataType::Int,
                            data: vec![
                                Expr::Literal(LiteralData::Int(10)),
                                Expr::Literal(LiteralData::Int(20)),
                                Expr::Literal(LiteralData::Int(30)),
                            ],
                        }),
                        method_name: "first".to_string(),
                        fn_index: (0, 0),
                        args: vec![],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: 10
    }

    #[test]
    fn test_compile_list_reverse() {
        use crate::syntax::DataType;

        let mut compiler = JITCompiler::new().unwrap();

        // let nums = [1, 2, 3];
        // let rev = nums.reverse();
        // output(rev.first());
        // 0
        let expr = Expr::Program {
            body: vec![
                Expr::Let {
                    var_name: "nums".to_string(),
                    value: Box::new(Expr::ListLiteral {
                        data_type: DataType::Int,
                        data: vec![
                            Expr::Literal(LiteralData::Int(1)),
                            Expr::Literal(LiteralData::Int(2)),
                            Expr::Literal(LiteralData::Int(3)),
                        ],
                    }),
                    mutable: false,
                    index: (0, 0),
                    data_type: DataType::List {
                        element_type: Box::new(DataType::Int),
                    },
                },
                Expr::Let {
                    var_name: "rev".to_string(),
                    value: Box::new(Expr::MethodCall {
                        receiver: Box::new(Expr::Variable {
                            name: "nums".to_string(),
                            index: (0, 0),
                        }),
                        method_name: "reverse".to_string(),
                        fn_index: (0, 0),
                        args: vec![],
                    }),
                    mutable: false,
                    index: (0, 0),
                    data_type: DataType::List {
                        element_type: Box::new(DataType::Int),
                    },
                },
                Expr::Output {
                    data: vec![Expr::MethodCall {
                        receiver: Box::new(Expr::Variable {
                            name: "rev".to_string(),
                            index: (0, 0),
                        }),
                        method_name: "first".to_string(),
                        fn_index: (0, 0),
                        args: vec![],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: 3
    }

    #[test]
    fn test_compile_map_keys() {
        use crate::syntax::{DataType, KeyData};

        let mut compiler = JITCompiler::new().unwrap();

        // let m = #{1: 10, 2: 20};
        // let k = m.keys();
        // output(k.first());
        // 0
        let expr = Expr::Program {
            body: vec![
                Expr::Let {
                    var_name: "m".to_string(),
                    value: Box::new(Expr::MapLiteral {
                        key_type: DataType::Int,
                        value_type: DataType::Int,
                        data: vec![
                            (KeyData::Int(1), Expr::Literal(LiteralData::Int(10))),
                            (KeyData::Int(2), Expr::Literal(LiteralData::Int(20))),
                        ],
                    }),
                    mutable: false,
                    index: (0, 0),
                    data_type: DataType::Map {
                        key_type: Box::new(DataType::Int),
                        value_type: Box::new(DataType::Int),
                    },
                },
                Expr::Let {
                    var_name: "k".to_string(),
                    value: Box::new(Expr::MethodCall {
                        receiver: Box::new(Expr::Variable {
                            name: "m".to_string(),
                            index: (0, 0),
                        }),
                        method_name: "keys".to_string(),
                        fn_index: (0, 0),
                        args: vec![],
                    }),
                    mutable: false,
                    index: (0, 0),
                    data_type: DataType::List {
                        element_type: Box::new(DataType::Int),
                    },
                },
                Expr::Output {
                    data: vec![Expr::MethodCall {
                        receiver: Box::new(Expr::Variable {
                            name: "k".to_string(),
                            index: (0, 0),
                        }),
                        method_name: "first".to_string(),
                        fn_index: (0, 0),
                        args: vec![],
                    }],
                },
                Expr::Literal(LiteralData::Int(0)),
            ],
            environment: 0,
        };

        let mut symbols = SymbolTable::new();
        let mut expr_mut = expr.clone();
        expr_mut.prepare(&mut symbols).unwrap();

        let result = compiler.compile_and_run(&expr_mut, &symbols).unwrap();
        assert_eq!(result, 0);
        // Should output: 1 (first key)
    }
}
