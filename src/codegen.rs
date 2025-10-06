// Code generation: Translates Lift AST to Cranelift IR

use crate::syntax::{Expr, LiteralData, Operator};
use crate::symboltable::SymbolTable;
use cranelift::prelude::*;
use cranelift_module::{FuncId, Module};
use cranelift_codegen::ir::{FuncRef, StackSlot};
use std::collections::HashMap;

pub struct CodeGenerator<'a, M: Module> {
    module: &'a mut M,
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,

    // Variable management: maps Lift variable names to Cranelift stack slots
    variables: HashMap<String, StackSlot>,

    // Runtime function references
    runtime_funcs: HashMap<String, FuncId>,
}

impl<'a, M: Module> CodeGenerator<'a, M> {
    pub fn new(module: &'a mut M) -> Self {
        let ctx = module.make_context();
        Self {
            module,
            builder_context: FunctionBuilderContext::new(),
            ctx,
            variables: HashMap::new(),
            runtime_funcs: HashMap::new(),
        }
    }

    /// Declare runtime functions in the module
    pub fn declare_runtime_functions(&mut self) -> Result<(), String> {
        let pointer_type = self.module.target_config().pointer_type();

        // lift_output_int(i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_output_int", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_int: {}", e))?;
        self.runtime_funcs.insert("lift_output_int".to_string(), func_id);

        // lift_output_float(f64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::F64));
        let func_id = self.module
            .declare_function("lift_output_float", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_float: {}", e))?;
        self.runtime_funcs.insert("lift_output_float".to_string(), func_id);

        // lift_output_bool(i8)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_output_bool", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_bool: {}", e))?;
        self.runtime_funcs.insert("lift_output_bool".to_string(), func_id);

        // lift_output_str(*const c_char)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_output_str", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_str: {}", e))?;
        self.runtime_funcs.insert("lift_output_str".to_string(), func_id);

        // lift_str_new(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_new: {}", e))?;
        self.runtime_funcs.insert("lift_str_new".to_string(), func_id);

        // lift_str_concat(*const c_char, *const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_concat", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_concat: {}", e))?;
        self.runtime_funcs.insert("lift_str_concat".to_string(), func_id);

        // lift_str_eq(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_str_eq", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_eq: {}", e))?;
        self.runtime_funcs.insert("lift_str_eq".to_string(), func_id);

        // lift_list_new(i64) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_list_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_new: {}", e))?;
        self.runtime_funcs.insert("lift_list_new".to_string(), func_id);

        // lift_list_set(*mut LiftList, i64, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_list_set", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_set: {}", e))?;
        self.runtime_funcs.insert("lift_list_set".to_string(), func_id);

        // lift_list_get(*const LiftList, i64) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_list_get", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_get: {}", e))?;
        self.runtime_funcs.insert("lift_list_get".to_string(), func_id);

        // lift_map_new(i64) -> *mut LiftMap
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_map_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_new: {}", e))?;
        self.runtime_funcs.insert("lift_map_new".to_string(), func_id);

        // lift_map_set(*mut LiftMap, i64, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_map_set", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_set: {}", e))?;
        self.runtime_funcs.insert("lift_map_set".to_string(), func_id);

        // lift_map_get(*const LiftMap, i64) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_map_get", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_get: {}", e))?;
        self.runtime_funcs.insert("lift_map_get".to_string(), func_id);

        Ok(())
    }

    /// Compile a Lift program (top-level expression)
    pub fn compile_program(
        &mut self,
        expr: &Expr,
        symbols: &SymbolTable,
    ) -> Result<FuncId, String> {
        // Create a main function with signature: () -> i64
        self.ctx.func.signature.returns.push(AbiParam::new(types::I64));

        // Create the function in the module
        let func_id = self.module
            .declare_function("main", cranelift_module::Linkage::Export, &self.ctx.func.signature)
            .map_err(|e| format!("Failed to declare main function: {}", e))?;

        // Build the function body
        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

            // Create entry block
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            // Declare runtime functions in this function's scope
            let mut runtime_refs = HashMap::new();
            for (name, func_id) in &self.runtime_funcs {
                let func_ref = self.module.declare_func_in_func(*func_id, &mut builder.func);
                runtime_refs.insert(name.clone(), func_ref);
            }

            // Compile the program expression
            let result = Self::compile_expr_static(&mut builder, expr, symbols, &runtime_refs, &mut self.variables)?;

            // Return the result (or 0 if Unit)
            let return_value = result.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().return_(&[return_value]);

            // Finalize the function
            builder.finalize();
        }

        // Define the function in the module
        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| format!("Failed to define main function: {}", e))?;

        // Clear the context for future compilations
        self.module.clear_context(&mut self.ctx);

        Ok(func_id)
    }

    /// Compile a Lift expression and return the Cranelift value (static version)
    /// Returns None for Unit expressions
    fn compile_expr_static(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        match expr {
            // Literals
            Expr::Literal(lit) => Self::compile_literal_with_runtime(builder, lit, runtime_funcs),
            Expr::RuntimeData(lit) => Self::compile_literal_with_runtime(builder, lit, runtime_funcs),

            // Binary operations
            Expr::BinaryExpr { left, op, right } => {
                Self::compile_binary_expr(builder, left, op, right, symbols, runtime_funcs, variables)
            }

            // Unary operations
            Expr::UnaryExpr { op, expr: inner } => {
                Self::compile_unary_expr(builder, op, inner, symbols, runtime_funcs, variables)
            }

            // Output
            Expr::Output { data } => {
                Self::compile_output(builder, data, symbols, runtime_funcs, variables)?;
                Ok(None) // output returns Unit
            }

            // Program and Block
            Expr::Program { body, .. } => {
                Self::compile_block_body(builder, body, symbols, runtime_funcs, variables)
            }
            Expr::Block { body, .. } => {
                Self::compile_block_body(builder, body, symbols, runtime_funcs, variables)
            }

            // Control flow
            Expr::If { cond, then, final_else } => {
                Self::compile_if_expr(builder, cond, then, final_else, symbols, runtime_funcs, variables)
            }

            Expr::While { cond, body } => {
                Self::compile_while_expr(builder, cond, body, symbols, runtime_funcs, variables)
            }

            // Variables
            Expr::Let { var_name, value, .. } => {
                Self::compile_let(builder, var_name, value, symbols, runtime_funcs, variables)
            }

            Expr::Variable { name, .. } => {
                Self::compile_variable(builder, name, variables)
            }

            Expr::Assign { name, value, .. } => {
                Self::compile_assign(builder, name, value, symbols, runtime_funcs, variables)
            }

            // Collections
            Expr::ListLiteral { data_type, data } => {
                Self::compile_list_literal(builder, data_type, data, symbols, runtime_funcs, variables)
            }

            Expr::MapLiteral { key_type, value_type, data } => {
                Self::compile_map_literal(builder, key_type, value_type, data, symbols, runtime_funcs, variables)
            }

            Expr::Index { expr, index } => {
                Self::compile_index(builder, expr, index, symbols, runtime_funcs, variables)
            }

            // Unit
            Expr::Unit => Ok(None),

            _ => Err(format!("Compilation not yet implemented for: {:?}", expr)),
        }
    }

    /// Compile a literal value
    fn compile_literal(
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

    /// Compile a literal value with access to runtime functions (for strings)
    fn compile_literal_with_runtime(
        builder: &mut FunctionBuilder,
        lit: &LiteralData,
        runtime_funcs: &HashMap<String, FuncRef>,
    ) -> Result<Option<Value>, String> {
        match lit {
            LiteralData::Str(s) => {
                // Create a stack slot big enough for the string + null terminator
                let byte_len = s.len() + 1; // +1 for null terminator
                let slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
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
                let func_ref = runtime_funcs.get("lift_str_new")
                    .ok_or_else(|| "Runtime function lift_str_new not found".to_string())?;
                let inst = builder.ins().call(*func_ref, &[str_ptr]);
                let result = builder.inst_results(inst)[0];

                Ok(Some(result))
            }
            // For non-string literals, use the simpler version
            _ => Self::compile_literal(builder, lit),
        }
    }

    /// Compile a binary expression
    fn compile_binary_expr(
        builder: &mut FunctionBuilder,
        left: &Expr,
        op: &Operator,
        right: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic_analysis::determine_type_with_symbols;
        use crate::syntax::DataType;

        // Check if we're dealing with string operations
        let left_type = determine_type_with_symbols(left, symbols, 0);
        let is_string_op = matches!(left_type, Some(DataType::Str));

        if is_string_op {
            let left_val = Self::compile_expr_static(builder, left, symbols, runtime_funcs, variables)?
                .ok_or("String operation requires non-Unit left operand")?;
            let right_val = Self::compile_expr_static(builder, right, symbols, runtime_funcs, variables)?
                .ok_or("String operation requires non-Unit right operand")?;

            match op {
                Operator::Add => {
                    // String concatenation
                    let func_ref = runtime_funcs.get("lift_str_concat")
                        .ok_or_else(|| "Runtime function lift_str_concat not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result = builder.inst_results(inst)[0];
                    return Ok(Some(result));
                }
                Operator::Eq => {
                    // String equality
                    let func_ref = runtime_funcs.get("lift_str_eq")
                        .ok_or_else(|| "Runtime function lift_str_eq not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result_i8 = builder.inst_results(inst)[0];
                    // Extend I8 to I64
                    let result_i64 = builder.ins().uextend(types::I64, result_i8);
                    return Ok(Some(result_i64));
                }
                Operator::Neq => {
                    // String inequality (not equal)
                    let func_ref = runtime_funcs.get("lift_str_eq")
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

        // For non-string operations, compile operands and perform integer/float operations
        let left_val = Self::compile_expr_static(builder, left, symbols, runtime_funcs, variables)?
            .ok_or("Binary operation requires non-Unit left operand")?;
        let right_val = Self::compile_expr_static(builder, right, symbols, runtime_funcs, variables)?
            .ok_or("Binary operation requires non-Unit right operand")?;

        let result = match op {
            Operator::Add => builder.ins().iadd(left_val, right_val),
            Operator::Sub => builder.ins().isub(left_val, right_val),
            Operator::Mul => builder.ins().imul(left_val, right_val),
            Operator::Div => builder.ins().sdiv(left_val, right_val),
            Operator::Gt => {
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Lt => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThan, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Gte => {
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Lte => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, left_val, right_val);
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
            _ => return Err(format!("Operator {:?} not yet implemented", op)),
        };

        Ok(Some(result))
    }

    /// Compile a unary expression
    fn compile_unary_expr(
        builder: &mut FunctionBuilder,
        op: &Operator,
        expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        let val = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, variables)?
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

    /// Compile an output statement
    fn compile_output(
        builder: &mut FunctionBuilder,
        data: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<(), String> {
        use crate::semantic_analysis::determine_type_with_symbols;
        use crate::syntax::DataType;

        for expr in data {
            // Determine the type of the expression
            let expr_type = determine_type_with_symbols(expr, symbols, 0)
                .ok_or_else(|| format!("Cannot determine type for output expression"))?;

            // Compile the expression to get the value
            let val = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, variables)?
                .ok_or_else(|| "Output requires non-Unit expression".to_string())?;

            // Determine which output function to call based on type
            let (func_name, needs_conversion) = match expr_type {
                DataType::Int => ("lift_output_int", false),
                DataType::Flt => ("lift_output_float", false),
                DataType::Bool => ("lift_output_bool", true), // Need to convert I64 to I8
                DataType::Str => ("lift_output_str", false),
                _ => return Err(format!("Output not yet supported for type: {:?}", expr_type)),
            };

            // Get the function reference
            let func_ref = runtime_funcs.get(func_name)
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
        Ok(())
    }

    /// Compile a block body (sequence of expressions)
    fn compile_block_body(
        builder: &mut FunctionBuilder,
        body: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        let mut last_value = None;

        for expr in body {
            last_value = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, variables)?;
        }

        Ok(last_value)
    }

    /// Compile an if/else expression
    fn compile_if_expr(
        builder: &mut FunctionBuilder,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        // Evaluate the condition
        let cond_val = Self::compile_expr_static(builder, cond, symbols, runtime_funcs, variables)?
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
        builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

        // Compile the then branch
        builder.switch_to_block(then_block);
        builder.seal_block(then_block);
        let then_val = Self::compile_expr_static(builder, then_expr, symbols, runtime_funcs, variables)?;

        if produces_value {
            let then_result = then_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().stack_store(then_result, result_slot.unwrap(), 0);
        }
        builder.ins().jump(merge_block, &[]);

        // Compile the else branch
        builder.switch_to_block(else_block);
        builder.seal_block(else_block);
        let else_val = Self::compile_expr_static(builder, else_expr, symbols, runtime_funcs, variables)?;

        if produces_value {
            let else_result = else_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().stack_store(else_result, result_slot.unwrap(), 0);
        }
        builder.ins().jump(merge_block, &[]);

        // Switch to merge block and load the result
        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);

        if produces_value {
            let result = builder.ins().stack_load(types::I64, result_slot.unwrap(), 0);
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Compile a while loop
    fn compile_while_expr(
        builder: &mut FunctionBuilder,
        cond: &Expr,
        body: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        // Create blocks for loop header, body, and exit
        let loop_header = builder.create_block();
        let loop_body = builder.create_block();
        let loop_exit = builder.create_block();

        // Jump to the loop header
        builder.ins().jump(loop_header, &[]);

        // Loop header: evaluate condition and branch
        builder.switch_to_block(loop_header);
        let cond_val = Self::compile_expr_static(builder, cond, symbols, runtime_funcs, variables)?
            .ok_or("While condition must produce a value")?;

        builder.ins().brif(cond_val, loop_body, &[], loop_exit, &[]);
        builder.seal_block(loop_header);

        // Loop body: execute body and jump back to header
        builder.switch_to_block(loop_body);
        Self::compile_expr_static(builder, body, symbols, runtime_funcs, variables)?;
        builder.ins().jump(loop_header, &[]);
        builder.seal_block(loop_body);

        // Exit block
        builder.switch_to_block(loop_exit);
        builder.seal_block(loop_exit);

        // While loops return Unit
        Ok(None)
    }

    /// Compile a let binding (variable declaration)
    fn compile_let(
        builder: &mut FunctionBuilder,
        var_name: &str,
        value: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        // Compile the value expression
        let val = Self::compile_expr_static(builder, value, symbols, runtime_funcs, variables)?
            .ok_or_else(|| format!("Let binding for '{}' requires a value", var_name))?;

        // Create a stack slot for this variable
        // For now, assume all variables are I64 (we'll extend this later)
        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            8, // 8 bytes for I64
            0,
        ));

        // Store the value in the stack slot
        builder.ins().stack_store(val, slot, 0);

        // Remember this variable's stack slot
        variables.insert(var_name.to_string(), slot);

        // Let expressions return Unit
        Ok(None)
    }

    /// Compile a variable reference (reading a variable)
    fn compile_variable(
        builder: &mut FunctionBuilder,
        name: &str,
        variables: &HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        // Look up the variable's stack slot
        let slot = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        // Load the value from the stack slot
        let val = builder.ins().stack_load(types::I64, *slot, 0);
        Ok(Some(val))
    }

    /// Compile an assignment expression (mutating a variable)
    fn compile_assign(
        builder: &mut FunctionBuilder,
        name: &str,
        value: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        // Compile the new value
        let val = Self::compile_expr_static(builder, value, symbols, runtime_funcs, variables)?
            .ok_or_else(|| format!("Assignment to '{}' requires a value", name))?;

        // Look up the variable's stack slot
        let slot = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        // Store the new value in the stack slot
        builder.ins().stack_store(val, *slot, 0);

        // Assignment returns Unit
        Ok(None)
    }

    /// Compile a list literal (for integer lists only in Phase 5)
    fn compile_list_literal(
        builder: &mut FunctionBuilder,
        data_type: &crate::syntax::DataType,
        data: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::DataType;
        use crate::semantic_analysis::determine_type_with_symbols;

        // Infer element type from first element if data_type is Unsolved
        let elem_type = if matches!(data_type, DataType::Unsolved) {
            if let Some(first_elem) = data.first() {
                determine_type_with_symbols(first_elem, symbols, 0)
                    .ok_or_else(|| "Cannot determine type of list elements".to_string())?
            } else {
                return Err("Empty list requires type annotation".to_string());
            }
        } else {
            data_type.clone()
        };

        // For now, only support integer lists
        if !matches!(elem_type, DataType::Int) {
            return Err(format!("Compiler only supports integer lists currently, got {:?}", elem_type));
        }

        // Create a new list with capacity equal to number of elements
        let capacity = builder.ins().iconst(types::I64, data.len() as i64);
        let func_ref = runtime_funcs.get("lift_list_new")
            .ok_or_else(|| "Runtime function lift_list_new not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[capacity]);
        let list_ptr = builder.inst_results(inst)[0];

        // Set each element in the list
        let set_func_ref = runtime_funcs.get("lift_list_set")
            .ok_or_else(|| "Runtime function lift_list_set not found".to_string())?;

        for (i, elem) in data.iter().enumerate() {
            // Compile the element value
            let elem_val = Self::compile_expr_static(builder, elem, symbols, runtime_funcs, variables)?
                .ok_or_else(|| "List element must produce a value".to_string())?;

            // Call lift_list_set(list, index, value)
            let index = builder.ins().iconst(types::I64, i as i64);
            builder.ins().call(*set_func_ref, &[list_ptr, index, elem_val]);
        }

        Ok(Some(list_ptr))
    }

    /// Compile a map literal (for integer key-value maps only in Phase 5)
    fn compile_map_literal(
        builder: &mut FunctionBuilder,
        key_type: &crate::syntax::DataType,
        value_type: &crate::syntax::DataType,
        data: &[(crate::syntax::KeyData, Expr)],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::{DataType, KeyData};
        use crate::semantic_analysis::determine_type_with_symbols;

        // Infer value type from first element if value_type is Unsolved
        let actual_value_type = if matches!(value_type, DataType::Unsolved) {
            if let Some((_, first_val)) = data.first() {
                determine_type_with_symbols(first_val, symbols, 0)
                    .ok_or_else(|| "Cannot determine type of map values".to_string())?
            } else {
                return Err("Empty map requires type annotation".to_string());
            }
        } else {
            value_type.clone()
        };

        // Infer key type from first element if key_type is Unsolved
        let actual_key_type = if matches!(key_type, DataType::Unsolved) {
            if let Some((first_key, _)) = data.first() {
                match first_key {
                    KeyData::Int(_) => DataType::Int,
                    KeyData::Str(_) => DataType::Str,
                    KeyData::Bool(_) => DataType::Bool,
                }
            } else {
                return Err("Empty map requires type annotation".to_string());
            }
        } else {
            key_type.clone()
        };

        // For now, only support integer key-value maps
        if !matches!(actual_key_type, DataType::Int) || !matches!(actual_value_type, DataType::Int) {
            return Err(format!("Compiler only supports integer key-value maps currently"));
        }

        // Create a new map with capacity equal to number of pairs
        let capacity = builder.ins().iconst(types::I64, data.len() as i64);
        let func_ref = runtime_funcs.get("lift_map_new")
            .ok_or_else(|| "Runtime function lift_map_new not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[capacity]);
        let map_ptr = builder.inst_results(inst)[0];

        // Set each key-value pair in the map
        let set_func_ref = runtime_funcs.get("lift_map_set")
            .ok_or_else(|| "Runtime function lift_map_set not found".to_string())?;

        for (key_data, value_expr) in data {
            // Extract the integer key
            let key_val = match key_data {
                KeyData::Int(k) => builder.ins().iconst(types::I64, *k),
                _ => return Err("Compiler only supports integer keys".to_string()),
            };

            // Compile the value expression
            let value_val = Self::compile_expr_static(builder, value_expr, symbols, runtime_funcs, variables)?
                .ok_or_else(|| "Map value must produce a value".to_string())?;

            // Call lift_map_set(map, key, value)
            builder.ins().call(*set_func_ref, &[map_ptr, key_val, value_val]);
        }

        Ok(Some(map_ptr))
    }

    /// Compile an index expression (list[i] or map[key])
    fn compile_index(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        index: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic_analysis::determine_type_with_symbols;
        use crate::syntax::DataType;

        // Compile the collection expression
        let collection = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, variables)?
            .ok_or("Index requires non-Unit collection")?;

        // Compile the index expression
        let index_val = Self::compile_expr_static(builder, index, symbols, runtime_funcs, variables)?
            .ok_or("Index requires non-Unit index value")?;

        // Determine if this is a list or map based on the type
        let expr_type = determine_type_with_symbols(expr, symbols, 0)
            .ok_or_else(|| "Cannot determine type for indexed expression".to_string())?;

        match expr_type {
            DataType::List { .. } => {
                // Call lift_list_get(list, index) -> i64
                let func_ref = runtime_funcs.get("lift_list_get")
                    .ok_or_else(|| "Runtime function lift_list_get not found".to_string())?;
                let inst = builder.ins().call(*func_ref, &[collection, index_val]);
                let result = builder.inst_results(inst)[0];
                Ok(Some(result))
            }
            DataType::Map { .. } => {
                // Call lift_map_get(map, key) -> i64
                let func_ref = runtime_funcs.get("lift_map_get")
                    .ok_or_else(|| "Runtime function lift_map_get not found".to_string())?;
                let inst = builder.ins().call(*func_ref, &[collection, index_val]);
                let result = builder.inst_results(inst)[0];
                Ok(Some(result))
            }
            _ => Err(format!("Cannot index into type: {:?}", expr_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests will be added as we implement more features
}
