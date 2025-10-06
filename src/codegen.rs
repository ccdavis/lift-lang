// Code generation: Translates Lift AST to Cranelift IR

use crate::syntax::{Expr, LiteralData, Operator};
use crate::symboltable::SymbolTable;
use cranelift::prelude::*;
use cranelift_module::{FuncId, Module};
use cranelift_codegen::ir::StackSlot;
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

        // lift_str_concat(*const c_char, *const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_concat", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_concat: {}", e))?;
        self.runtime_funcs.insert("lift_str_concat".to_string(), func_id);

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

            // Compile the program expression
            let result = Self::compile_expr_static(&mut builder, expr, symbols, &self.runtime_funcs, &mut self.variables)?;

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
        runtime_funcs: &HashMap<String, FuncId>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        match expr {
            // Literals
            Expr::Literal(lit) => Self::compile_literal(builder, lit),
            Expr::RuntimeData(lit) => Self::compile_literal(builder, lit),

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
                let val = builder.ins().iconst(types::I8, if *b { 1 } else { 0 });
                Ok(Some(val))
            }
            LiteralData::Str(_s) => {
                // For now, return a placeholder
                // TODO: Implement string literal support with data section
                Err("String literals not yet implemented in compiler".to_string())
            }
        }
    }

    /// Compile a binary expression
    fn compile_binary_expr(
        builder: &mut FunctionBuilder,
        left: &Expr,
        op: &Operator,
        right: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncId>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
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
                builder.ins().uextend(types::I8, cmp)
            }
            Operator::Lt => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThan, left_val, right_val);
                builder.ins().uextend(types::I8, cmp)
            }
            Operator::Gte => {
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val);
                builder.ins().uextend(types::I8, cmp)
            }
            Operator::Lte => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, left_val, right_val);
                builder.ins().uextend(types::I8, cmp)
            }
            Operator::Eq => {
                let cmp = builder.ins().icmp(IntCC::Equal, left_val, right_val);
                builder.ins().uextend(types::I8, cmp)
            }
            Operator::Neq => {
                let cmp = builder.ins().icmp(IntCC::NotEqual, left_val, right_val);
                builder.ins().uextend(types::I8, cmp)
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
        runtime_funcs: &HashMap<String, FuncId>,
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
                let zero = builder.ins().iconst(types::I8, 0);
                let cmp = builder.ins().icmp(IntCC::Equal, val, zero);
                builder.ins().uextend(types::I8, cmp)
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
        runtime_funcs: &HashMap<String, FuncId>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<(), String> {
        for expr in data {
            let val_opt = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, variables)?;

            if let Some(_val) = val_opt {
                // For now, skip output implementation
                // TODO: Implement proper output with type checking
            }
        }
        Ok(())
    }

    /// Compile a block body (sequence of expressions)
    fn compile_block_body(
        builder: &mut FunctionBuilder,
        body: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncId>,
        variables: &mut HashMap<String, StackSlot>,
    ) -> Result<Option<Value>, String> {
        let mut last_value = None;

        for expr in body {
            last_value = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, variables)?;
        }

        Ok(last_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests will be added as we implement more features
}
