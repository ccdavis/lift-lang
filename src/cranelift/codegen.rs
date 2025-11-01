// Core code generation struct and main compilation methods

use super::types::{resolve_type_alias, VarInfo};
use crate::symboltable::SymbolTable;
use crate::syntax::{DataType, Expr};
use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use cranelift_module::{FuncId, Module};
use std::collections::HashMap;

/// Main code generator for Cranelift
pub struct CodeGenerator<'a, M: Module> {
    pub(super) module: &'a mut M,
    pub(super) builder_context: FunctionBuilderContext,
    pub(super) ctx: codegen::Context,

    // Variable management: maps Lift variable names to stack slot and type info
    pub(super) variables: HashMap<String, VarInfo>,

    // Runtime function references
    pub(super) runtime_funcs: HashMap<String, FuncId>,

    // User-defined function references: maps function names to FuncId
    pub(super) function_refs: HashMap<String, FuncId>,

    // Note: scope_allocations is now passed as a parameter through compilation functions
    // rather than stored as a struct field (was never read, only the local variable was used)
}

impl<'a, M: Module> CodeGenerator<'a, M> {
    /// Create a new code generator
    pub fn new(module: &'a mut M) -> Self {
        let ctx = module.make_context();
        Self {
            module,
            builder_context: FunctionBuilderContext::new(),
            ctx,
            variables: HashMap::new(),
            runtime_funcs: HashMap::new(),
            function_refs: HashMap::new(),
        }
    }

    /// Compile a Lift program (top-level expression)
    pub fn compile_program(
        &mut self,
        expr: &Expr,
        symbols: &SymbolTable,
    ) -> Result<FuncId, String> {
        // PREPROCESSING STEP: Collect and compile all user-defined functions first
        let mut function_defs = Vec::new();
        self.collect_function_definitions(expr, &mut function_defs);

        // Compile each function definition
        for (fn_name, lambda_expr) in function_defs {
            self.compile_user_function(fn_name, lambda_expr, symbols)?;
        }

        // Create a main function with signature: () -> i64
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I64));

        // Create the function in the module
        let func_id = self
            .module
            .declare_function(
                "main",
                cranelift_module::Linkage::Export,
                &self.ctx.func.signature,
            )
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
                let func_ref = self
                    .module
                    .declare_func_in_func(*func_id, &mut builder.func);
                runtime_refs.insert(name.clone(), func_ref);
            }

            // Declare user functions in this function's scope
            let mut user_func_refs = HashMap::new();
            for (name, func_id) in &self.function_refs {
                let func_ref = self
                    .module
                    .declare_func_in_func(*func_id, &mut builder.func);
                user_func_refs.insert(name.clone(), func_ref);
            }

            // Initialize scope tracking for the main function
            let mut scope_allocations: Vec<Vec<(Value, String)>> = Vec::new();
            Self::enter_scope(&mut scope_allocations);

            // Compile the program expression with user function support
            let result = Self::compile_expr_static(
                &mut builder,
                expr,
                symbols,
                &runtime_refs,
                &user_func_refs,
                &mut self.variables,
                &mut scope_allocations,
            )?;

            // Clean up allocations before returning
            Self::exit_scope(&mut builder, &runtime_refs, &mut scope_allocations);

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

    /// Main expression compiler - dispatches to specialized compile methods
    pub(super) fn compile_expr_static(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
    ) -> Result<Option<Value>, String> {
        match expr {
            // Literals
            Expr::Literal(lit) => Self::compile_literal_with_runtime(builder, lit, runtime_funcs),
            Expr::RuntimeData(lit) => {
                Self::compile_literal_with_runtime(builder, lit, runtime_funcs)
            }

            // Binary operations
            Expr::BinaryExpr { left, op, right } => Self::compile_binary_expr(
                builder,
                left,
                op,
                right,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Unary operations
            Expr::UnaryExpr { op, expr: inner } => Self::compile_unary_expr(
                builder,
                op,
                inner,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Output
            Expr::Output { data } => {
                Self::compile_output(
                    builder,
                    data,
                    symbols,
                    runtime_funcs,
                    user_func_refs,
                    variables,
                    scope_allocations,
                )?;
                Ok(None) // output returns Unit
            }

            // Program and Block
            Expr::Program { body, .. } => Self::compile_block_body(
                builder,
                body,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),
            Expr::Block { body, .. } => Self::compile_block_body(
                builder,
                body,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Control flow
            Expr::If {
                cond,
                then,
                final_else,
            } => Self::compile_if_expr(
                builder,
                cond,
                then,
                final_else,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            Expr::While { cond, body } => Self::compile_while_expr(
                builder,
                cond,
                body,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Variables
            Expr::Let {
                var_name,
                value,
                data_type,
                ..
            } => Self::compile_let(
                builder,
                var_name,
                value,
                data_type,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            Expr::Variable { name, .. } => Self::compile_variable(builder, name, variables),

            Expr::Assign { name, value, .. } => Self::compile_assign(
                builder,
                name,
                value,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Collections
            Expr::ListLiteral { data_type, data } => Self::compile_list_literal(
                builder,
                data_type,
                data,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            Expr::MapLiteral {
                key_type,
                value_type,
                data,
            } => Self::compile_map_literal(
                builder,
                key_type,
                value_type,
                data,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            Expr::Index { expr, index } => Self::compile_index(
                builder,
                expr,
                index,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Built-in functions
            Expr::Len { expr } => Self::compile_len(
                builder,
                expr,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            Expr::MethodCall {
                receiver,
                method_name,
                args,
                ..
            } => Self::compile_method_call(
                builder,
                receiver,
                method_name,
                args,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Range
            Expr::Range(start, end) => {
                Self::compile_range(builder, start, end, runtime_funcs, scope_allocations)
            }

            // Structs
            Expr::StructLiteral { type_name, fields } => Self::compile_struct_literal(
                builder,
                type_name,
                fields,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            Expr::FieldAccess { expr, field_name } => Self::compile_field_access(
                builder,
                expr,
                field_name,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            Expr::FieldAssign {
                expr,
                field_name,
                value,
                ..
            } => Self::compile_field_assign(
                builder,
                expr,
                field_name,
                value,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Unit
            Expr::Unit => Ok(None),

            // Function calls
            Expr::Call {
                fn_name,
                args,
                index,
                ..
            } => Self::compile_function_call(
                builder,
                fn_name,
                args,
                index,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            ),

            // Function definitions (handled in preprocessing, so return Unit here)
            Expr::DefineFunction { .. } => Ok(None),

            // Type definitions (compile-time only, return Unit)
            Expr::DefineType { .. } => Ok(None),

            _ => Err(format!("Compilation not yet implemented for: {:?}", expr)),
        }
    }

    /// Resolve type aliases to their underlying types (re-exported from types module)
    pub(super) fn resolve_type_alias(data_type: &DataType, symbols: &SymbolTable) -> DataType {
        resolve_type_alias(data_type, symbols)
    }

    /// Convert Lift DataType to Cranelift Type (re-exported from types module)
    pub(super) fn data_type_to_cranelift_type(dt: &DataType, pointer_type: Type) -> Type {
        super::types::data_type_to_cranelift_type(dt, pointer_type)
    }

    // ==================== Reference Counting Helper Methods ====================

    /// Enter a new scope for allocation tracking
    pub(super) fn enter_scope(scope_allocations: &mut Vec<Vec<(Value, String)>>) {
        scope_allocations.push(Vec::new());
    }

    /// Exit the current scope and release all allocations
    pub(super) fn exit_scope(
        builder: &mut FunctionBuilder,
        runtime_funcs: &HashMap<String, FuncRef>,
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
    ) {
        if let Some(allocations) = scope_allocations.pop() {
            for (ptr, type_name) in allocations {
                Self::emit_release_call(builder, runtime_funcs, ptr, &type_name);
            }
        }
    }

    /// Record an allocation in the current scope
    pub(super) fn record_allocation(
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
        ptr: Value,
        type_name: &str,
    ) {
        if let Some(current_scope) = scope_allocations.last_mut() {
            current_scope.push((ptr, type_name.to_string()));
        }
    }

    /// Remove an allocation from tracking (used for return values that escape the scope)
    pub(super) fn untrack_allocation(
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
        ptr: Value,
    ) {
        if let Some(current_scope) = scope_allocations.last_mut() {
            current_scope.retain(|(p, _)| *p != ptr);
        }
    }

    /// Emit a call to the appropriate release function for a heap-allocated value
    pub(super) fn emit_release_call(
        builder: &mut FunctionBuilder,
        runtime_funcs: &HashMap<String, FuncRef>,
        ptr: Value,
        type_name: &str,
    ) {
        let func_name = format!("lift_{}_release", type_name);
        if let Some(&func_ref) = runtime_funcs.get(&func_name) {
            builder.ins().call(func_ref, &[ptr]);
        }
    }
}
