// Expression compilation methods for Cranelift code generation

use super::types::VarInfo;
use super::CodeGenerator;
use crate::symboltable::SymbolTable;
use crate::syntax::{Expr, LiteralData, Operator};
use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use cranelift_module::Module;
use std::collections::HashMap;

impl<'a, M: Module> CodeGenerator<'a, M> {
    pub(super) fn compile_literal(
        builder: &mut FunctionBuilder,
        lit: &LiteralData,
    ) -> Result<Option<Value>, String> {
        match lit {
            LiteralData::Int(i) => {
                let val = builder.ins().iconst(types::I64, *i);
                Ok(Some(val))
            }
            LiteralData::Flt(f) => {
                let val = builder.ins().f64const(*f);
                Ok(Some(val))
            }
            LiteralData::Bool(b) => {
                let val = builder.ins().iconst(types::I64, if *b { 1 } else { 0 });
                Ok(Some(val))
            }
            LiteralData::Str(_s) => {
                // String literals need access to runtime functions
                // For now, we'll return an error and handle them in compile_literal_with_runtime
                Err("String literals require runtime function access - use compile_literal_with_runtime".to_string())
            }
        }
    }

    pub(super) fn compile_literal_with_runtime(
        builder: &mut FunctionBuilder,
        lit: &LiteralData,
        runtime_funcs: &HashMap<String, FuncRef>,
    ) -> Result<Option<Value>, String> {
        match lit {
            LiteralData::Str(s) => {
                // Create a stack slot big enough for the string + null terminator
                let byte_len = s.len() + 1; // +1 for null terminator
                let slot =
                    builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        byte_len as u32,
                        0,
                    ));

                // Store each byte of the string in the stack slot
                for (i, byte) in s.bytes().enumerate() {
                    let byte_val = builder.ins().iconst(types::I8, byte as i64);
                    builder.ins().stack_store(byte_val, slot, i as i32);
                }
                // Store null terminator
                let null_byte = builder.ins().iconst(types::I8, 0);
                builder.ins().stack_store(null_byte, slot, s.len() as i32);

                // Get pointer to the stack slot
                let str_ptr = builder.ins().stack_addr(types::I64, slot, 0);

                // Call lift_str_new to create a heap-allocated string
                let func_ref = runtime_funcs
                    .get("lift_str_new")
                    .ok_or_else(|| "Runtime function lift_str_new not found".to_string())?;
                let inst = builder.ins().call(*func_ref, &[str_ptr]);
                let result = builder.inst_results(inst)[0];

                Ok(Some(result))
            }
            // For non-string literals, use the simpler version
            _ => Self::compile_literal(builder, lit),
        }
    }

    pub(super) fn compile_binary_expr(
        builder: &mut FunctionBuilder,
        left: &Expr,
        op: &Operator,
        right: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic::determine_type_with_symbols;
        use crate::syntax::DataType;

        // Check the type of the left operand to determine operation type
        // We must check the original left (before any reordering) to get the correct type
        let left_type_raw = determine_type_with_symbols(left, symbols, 0);
        let left_type = left_type_raw.map(|t| Self::resolve_type_alias(&t, symbols));

        // Also check right operand type for better type inference
        let right_type_raw = determine_type_with_symbols(right, symbols, 0);
        let right_type = right_type_raw.map(|t| Self::resolve_type_alias(&t, symbols));

        // Determine if this is a string or float operation
        let is_string_op =
            matches!(left_type, Some(DataType::Str)) || matches!(right_type, Some(DataType::Str));
        let is_float_op =
            matches!(left_type, Some(DataType::Flt)) || matches!(right_type, Some(DataType::Flt));

        if is_string_op {
            let left_val = Self::compile_expr_static(
                builder,
                left,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("String operation requires non-Unit left operand")?;
            let right_val = Self::compile_expr_static(
                builder,
                right,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("String operation requires non-Unit right operand")?;

            match op {
                Operator::Add => {
                    // String concatenation
                    let func_ref = runtime_funcs
                        .get("lift_str_concat")
                        .ok_or_else(|| "Runtime function lift_str_concat not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result = builder.inst_results(inst)[0];
                    return Ok(Some(result));
                }
                Operator::Eq => {
                    // String equality
                    let func_ref = runtime_funcs
                        .get("lift_str_eq")
                        .ok_or_else(|| "Runtime function lift_str_eq not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result_i8 = builder.inst_results(inst)[0];
                    // Extend I8 to I64
                    let result_i64 = builder.ins().uextend(types::I64, result_i8);
                    return Ok(Some(result_i64));
                }
                Operator::Neq => {
                    // String inequality (not equal)
                    let func_ref = runtime_funcs
                        .get("lift_str_eq")
                        .ok_or_else(|| "Runtime function lift_str_eq not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result_i8 = builder.inst_results(inst)[0];
                    // Extend I8 to I64
                    let eq_i64 = builder.ins().uextend(types::I64, result_i8);
                    // Negate with XOR 1 (0 becomes 1, 1 becomes 0)
                    let one = builder.ins().iconst(types::I64, 1);
                    let neq = builder.ins().bxor(eq_i64, one);
                    return Ok(Some(neq));
                }
                _ => return Err(format!("Operator {:?} not supported for strings", op)),
            }
        }

        // Handle float operations
        if is_float_op {
            let mut left_val = Self::compile_expr_static(
                builder,
                left,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("Float operation requires non-Unit left operand")?;
            let mut right_val = Self::compile_expr_static(
                builder,
                right,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("Float operation requires non-Unit right operand")?;

            // Promote Int to Flt if necessary
            // If left is Int but right is Flt, convert left to Flt
            if matches!(left_type, Some(DataType::Int)) && matches!(right_type, Some(DataType::Flt)) {
                left_val = builder.ins().fcvt_from_sint(types::F64, left_val);
            }
            // If right is Int but left is Flt, convert right to Flt
            if matches!(right_type, Some(DataType::Int)) && matches!(left_type, Some(DataType::Flt)) {
                right_val = builder.ins().fcvt_from_sint(types::F64, right_val);
            }

            let result = match op {
                Operator::Add => builder.ins().fadd(left_val, right_val),
                Operator::Sub => builder.ins().fsub(left_val, right_val),
                Operator::Mul => builder.ins().fmul(left_val, right_val),
                Operator::Div => builder.ins().fdiv(left_val, right_val),
                Operator::Gt => {
                    let cmp = builder
                        .ins()
                        .fcmp(FloatCC::GreaterThan, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Lt => {
                    let cmp = builder.ins().fcmp(FloatCC::LessThan, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Gte => {
                    let cmp = builder
                        .ins()
                        .fcmp(FloatCC::GreaterThanOrEqual, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Lte => {
                    let cmp = builder
                        .ins()
                        .fcmp(FloatCC::LessThanOrEqual, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Eq => {
                    let cmp = builder.ins().fcmp(FloatCC::Equal, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Neq => {
                    let cmp = builder.ins().fcmp(FloatCC::NotEqual, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                _ => return Err(format!("Operator {:?} not yet implemented for floats", op)),
            };
            return Ok(Some(result));
        }

        // Handle struct comparison (only Eq and Neq supported)
        let is_struct_op = matches!(left_type, Some(DataType::Struct(_)))
            || matches!(right_type, Some(DataType::Struct(_)));
        if is_struct_op && matches!(op, Operator::Eq | Operator::Neq) {
            let left_val = Self::compile_expr_static(
                builder,
                left,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("Struct comparison requires non-Unit left operand")?;
            let right_val = Self::compile_expr_static(
                builder,
                right,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("Struct comparison requires non-Unit right operand")?;

            match op {
                Operator::Eq => {
                    // Struct equality
                    let func_ref = runtime_funcs
                        .get("lift_struct_eq")
                        .ok_or_else(|| "Runtime function lift_struct_eq not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result_i8 = builder.inst_results(inst)[0];
                    // Extend I8 to I64
                    let result_i64 = builder.ins().uextend(types::I64, result_i8);
                    return Ok(Some(result_i64));
                }
                Operator::Neq => {
                    // Struct inequality (not equal)
                    let func_ref = runtime_funcs
                        .get("lift_struct_eq")
                        .ok_or_else(|| "Runtime function lift_struct_eq not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result_i8 = builder.inst_results(inst)[0];
                    // Extend I8 to I64
                    let eq_i64 = builder.ins().uextend(types::I64, result_i8);
                    // Negate with XOR 1 (0 becomes 1, 1 becomes 0)
                    let one = builder.ins().iconst(types::I64, 1);
                    let neq = builder.ins().bxor(eq_i64, one);
                    return Ok(Some(neq));
                }
                _ => return Err(format!("Operator {:?} not supported for structs", op)),
            }
        }

        // Handle Range operator
        if matches!(op, Operator::Range) {
            let left_val = Self::compile_expr_static(
                builder,
                left,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("Range operation requires non-Unit left operand")?;
            let right_val = Self::compile_expr_static(
                builder,
                right,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or("Range operation requires non-Unit right operand")?;

            // Call lift_range_new(start, end)
            let func_ref = runtime_funcs
                .get("lift_range_new")
                .ok_or_else(|| "Runtime function lift_range_new not found".to_string())?;
            let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
            let range_ptr = builder.inst_results(inst)[0];
            return Ok(Some(range_ptr));
        }

        // For integer operations, compile operands and perform integer operations
        let left_val = Self::compile_expr_static(
            builder,
            left,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or("Binary operation requires non-Unit left operand")?;
        let right_val = Self::compile_expr_static(
            builder,
            right,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or("Binary operation requires non-Unit right operand")?;

        let result = match op {
            Operator::Add => builder.ins().iadd(left_val, right_val),
            Operator::Sub => builder.ins().isub(left_val, right_val),
            Operator::Mul => builder.ins().imul(left_val, right_val),
            Operator::Div => builder.ins().sdiv(left_val, right_val),
            Operator::Gt => {
                let cmp = builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThan, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Lt => {
                let cmp = builder
                    .ins()
                    .icmp(IntCC::SignedLessThan, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Gte => {
                let cmp = builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Lte => {
                let cmp = builder
                    .ins()
                    .icmp(IntCC::SignedLessThanOrEqual, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Eq => {
                let cmp = builder.ins().icmp(IntCC::Equal, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Neq => {
                let cmp = builder.ins().icmp(IntCC::NotEqual, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::And => {
                // Logical AND: both operands must be non-zero
                // Convert each operand to boolean (0 or 1), then AND them
                let zero = builder.ins().iconst(types::I64, 0);
                let left_bool = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                let right_bool = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                let result_bool = builder.ins().band(left_bool, right_bool);
                builder.ins().uextend(types::I64, result_bool)
            }
            Operator::Or => {
                // Logical OR: at least one operand must be non-zero
                // Convert each operand to boolean (0 or 1), then OR them
                let zero = builder.ins().iconst(types::I64, 0);
                let left_bool = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                let right_bool = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                let result_bool = builder.ins().bor(left_bool, right_bool);
                builder.ins().uextend(types::I64, result_bool)
            }
            _ => return Err(format!("Operator {:?} not yet implemented", op)),
        };

        Ok(Some(result))
    }

    pub(super) fn compile_unary_expr(
        builder: &mut FunctionBuilder,
        op: &Operator,
        expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        let val = Self::compile_expr_static(
            builder,
            expr,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or("Unary operation requires non-Unit operand")?;

        let result = match op {
            Operator::Sub => {
                // Negate: 0 - val
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().isub(zero, val)
            }
            Operator::Not => {
                // Boolean not: val == 0
                let zero = builder.ins().iconst(types::I64, 0);
                let cmp = builder.ins().icmp(IntCC::Equal, val, zero);
                builder.ins().uextend(types::I64, cmp)
            }
            _ => return Err(format!("Unary operator {:?} not yet implemented", op)),
        };

        Ok(Some(result))
    }

    pub(super) fn compile_block_body(
        builder: &mut FunctionBuilder,
        body: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        let mut last_value = None;

        for expr in body {
            last_value = Self::compile_expr_static(
                builder,
                expr,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?;
        }

        Ok(last_value)
    }

    pub(super) fn compile_if_expr(
        builder: &mut FunctionBuilder,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Evaluate the condition
        let cond_val = Self::compile_expr_static(
            builder,
            cond,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or("If condition must produce a value")?;

        // Create blocks for the then branch, else branch, and merge point
        let then_block = builder.create_block();
        let else_block = builder.create_block();
        let merge_block = builder.create_block();

        // Check if this if expression produces a value
        let produces_value = !matches!(then_expr, Expr::Unit) || !matches!(else_expr, Expr::Unit);

        // Create a stack slot to store the result if needed
        let result_slot = if produces_value {
            Some(builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8, // 8 bytes for I64
                0,
            )))
        } else {
            None
        };

        // Branch based on condition
        // In Cranelift, brif branches if value is non-zero
        builder
            .ins()
            .brif(cond_val, then_block, &[], else_block, &[]);

        // Compile the then branch
        builder.switch_to_block(then_block);
        builder.seal_block(then_block);
        let then_val = Self::compile_expr_static(
            builder,
            then_expr,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?;

        if produces_value {
            let then_result = then_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder
                .ins()
                .stack_store(then_result, result_slot.unwrap(), 0);
        }
        builder.ins().jump(merge_block, &[]);

        // Compile the else branch
        builder.switch_to_block(else_block);
        builder.seal_block(else_block);
        let else_val = Self::compile_expr_static(
            builder,
            else_expr,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?;

        if produces_value {
            let else_result = else_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder
                .ins()
                .stack_store(else_result, result_slot.unwrap(), 0);
        }
        builder.ins().jump(merge_block, &[]);

        // Switch to merge block and load the result
        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);

        if produces_value {
            let result = builder
                .ins()
                .stack_load(types::I64, result_slot.unwrap(), 0);
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub(super) fn compile_while_expr(
        builder: &mut FunctionBuilder,
        cond: &Expr,
        body: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Create blocks for loop header, body, and exit
        let loop_header = builder.create_block();
        let loop_body = builder.create_block();
        let loop_exit = builder.create_block();

        // Jump to the loop header
        builder.ins().jump(loop_header, &[]);

        // Loop header: evaluate condition and branch
        builder.switch_to_block(loop_header);
        let cond_val = Self::compile_expr_static(
            builder,
            cond,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or("While condition must produce a value")?;

        builder.ins().brif(cond_val, loop_body, &[], loop_exit, &[]);
        builder.seal_block(loop_header);

        // Loop body: execute body and jump back to header
        builder.switch_to_block(loop_body);
        Self::compile_expr_static(
            builder,
            body,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?;
        builder.ins().jump(loop_header, &[]);
        builder.seal_block(loop_body);

        // Exit block
        builder.switch_to_block(loop_exit);
        builder.seal_block(loop_exit);

        // While loops return Unit
        Ok(None)
    }

    pub(super) fn compile_output(
        builder: &mut FunctionBuilder,
        data: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<(), String> {
        use crate::semantic::determine_type_with_symbols;
        use crate::syntax::DataType;

        for expr in data {
            // Determine the type of the expression
            let expr_type_raw = determine_type_with_symbols(expr, symbols, 0)
                .ok_or_else(|| format!("Cannot determine type for output expression"))?;

            // Resolve TypeRef to underlying type
            let expr_type = Self::resolve_type_alias(&expr_type_raw, symbols);

            // Compile the expression to get the value
            let val = Self::compile_expr_static(
                builder,
                expr,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or_else(|| "Output requires non-Unit expression".to_string())?;

            // Determine which output function to call based on type
            let (func_name, needs_conversion) = match expr_type {
                DataType::Int => ("lift_output_int", false),
                DataType::Flt => ("lift_output_float", false),
                DataType::Bool => ("lift_output_bool", true), // Need to convert I64 to I8
                DataType::Str => ("lift_output_str", false),
                DataType::Range(_) => ("lift_output_range", false),
                DataType::List { .. } => ("lift_output_list", false),
                DataType::Map { .. } => ("lift_output_map", false),
                DataType::Struct(_) => ("lift_output_struct", false),
                _ => {
                    return Err(format!(
                        "Output not yet supported for type: {:?}",
                        expr_type
                    ))
                }
            };

            // Get the function reference
            let func_ref = runtime_funcs
                .get(func_name)
                .ok_or_else(|| format!("Runtime function not found: {}", func_name))?;

            // Convert value if needed
            let call_val = if needs_conversion {
                builder.ins().ireduce(types::I8, val)
            } else {
                val
            };

            // Call the function
            builder.ins().call(*func_ref, &[call_val]);
        }

        // Print newline after all output items (to match interpreter behavior)
        let newline_func = runtime_funcs
            .get("lift_output_newline")
            .ok_or_else(|| "Runtime function not found: lift_output_newline".to_string())?;
        builder.ins().call(*newline_func, &[]);

        Ok(())
    }

    pub(super) fn compile_range(
        builder: &mut FunctionBuilder,
        start: &LiteralData,
        end: &LiteralData,
        runtime_funcs: &HashMap<String, FuncRef>,
    ) -> Result<Option<Value>, String> {
        // Extract integer values from start and end
        let start_val = match start {
            LiteralData::Int(i) => builder.ins().iconst(types::I64, *i),
            _ => return Err("Range start must be an integer".to_string()),
        };

        let end_val = match end {
            LiteralData::Int(i) => builder.ins().iconst(types::I64, *i),
            _ => return Err("Range end must be an integer".to_string()),
        };

        // Call lift_range_new(start, end) to create the range
        let func_ref = runtime_funcs
            .get("lift_range_new")
            .ok_or_else(|| "Runtime function lift_range_new not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[start_val, end_val]);
        let range_ptr = builder.inst_results(inst)[0];

        Ok(Some(range_ptr))
    }
}
