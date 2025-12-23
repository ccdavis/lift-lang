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

    /// Captured variables for each function: maps function name to list of (var_name, type)
    /// These are variables from outer scopes that the function references.
    /// They are passed as hidden parameters when calling the function.
    pub(super) function_captures: HashMap<String, Vec<(String, DataType)>>,

    /// Anonymous lambdas: maps lambda environment ID to generated function name
    /// Used to compile lambdas passed as arguments to higher-order functions
    pub(super) anonymous_lambdas: HashMap<usize, String>,
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
            function_captures: HashMap::new(),
            anonymous_lambdas: HashMap::new(),
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

        // Collect anonymous lambdas (lambdas passed as arguments to HOFs)
        let mut anonymous_lambda_defs: Vec<(String, &Expr, usize)> = Vec::new();
        let mut lambda_counter = 0usize;
        self.collect_anonymous_lambdas(expr, &mut anonymous_lambda_defs, &mut lambda_counter);

        // Store the environment -> name mapping for later lookup
        for (name, _lambda_expr, env_id) in &anonymous_lambda_defs {
            self.anonymous_lambdas.insert(*env_id, name.clone());
        }

        // First pass: collect direct captures for all functions (named + anonymous)
        let mut all_direct_captures: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        // Named functions
        for (fn_name, lambda_expr) in &function_defs {
            if let Expr::Lambda { value: func, .. } = lambda_expr {
                let captures = Self::find_direct_captured_variables(func);
                all_direct_captures.insert(fn_name.to_string(), captures);
            }
        }

        // Anonymous lambdas
        for (lambda_name, lambda_expr, _env_id) in &anonymous_lambda_defs {
            if let Expr::Lambda { value: func, .. } = lambda_expr {
                let captures = Self::find_direct_captured_variables(func);
                all_direct_captures.insert(lambda_name.clone(), captures);
            }
        }

        // Second pass: compute transitive captures (iterate until fixed point)
        // We need multiple passes because A might call B which calls C, etc.
        loop {
            let mut changed = false;

            // Named functions
            for (fn_name, lambda_expr) in &function_defs {
                if let Expr::Lambda { value: func, .. } = lambda_expr {
                    let new_captures =
                        Self::find_captured_variables_with_transitive(func, &all_direct_captures);
                    let old_captures = all_direct_captures.get(*fn_name).cloned().unwrap_or_default();

                    if new_captures.len() != old_captures.len()
                        || !new_captures.iter().all(|c| old_captures.contains(c))
                    {
                        all_direct_captures.insert(fn_name.to_string(), new_captures);
                        changed = true;
                    }
                }
            }

            // Anonymous lambdas
            for (lambda_name, lambda_expr, _env_id) in &anonymous_lambda_defs {
                if let Expr::Lambda { value: func, .. } = lambda_expr {
                    let new_captures =
                        Self::find_captured_variables_with_transitive(func, &all_direct_captures);
                    let old_captures = all_direct_captures.get(lambda_name).cloned().unwrap_or_default();

                    if new_captures.len() != old_captures.len()
                        || !new_captures.iter().all(|c| old_captures.contains(c))
                    {
                        all_direct_captures.insert(lambda_name.clone(), new_captures);
                        changed = true;
                    }
                }
            }

            if !changed {
                break;
            }
        }

        // Store the final captures for use in compile_user_function
        self.function_captures = all_direct_captures
            .into_iter()
            .filter_map(|(name, caps)| {
                if caps.is_empty() {
                    None
                } else {
                    // Get types for captured variables
                    let typed_caps: Vec<(String, DataType)> = caps
                        .into_iter()
                        .map(|var_name| {
                            let var_type = symbols
                                .find_symbol_type_by_name(&var_name)
                                .unwrap_or(DataType::Int);
                            (var_name, var_type)
                        })
                        .collect();
                    Some((name, typed_caps))
                }
            })
            .collect();

        // Compile each named function definition
        for (fn_name, lambda_expr) in function_defs {
            self.compile_user_function(fn_name, lambda_expr, symbols)?;
        }

        // Compile each anonymous lambda as a function
        for (lambda_name, lambda_expr, _env_id) in anonymous_lambda_defs {
            self.compile_user_function(&lambda_name, lambda_expr, symbols)?;
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
                let func_ref = self.module.declare_func_in_func(*func_id, builder.func);
                runtime_refs.insert(name.clone(), func_ref);
            }

            // Declare user functions in this function's scope
            let mut user_func_refs = HashMap::new();
            for (name, func_id) in &self.function_refs {
                let func_ref = self.module.declare_func_in_func(*func_id, builder.func);
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
                &self.function_captures,
                &self.anonymous_lambdas,
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
            .map_err(|e| {
                eprintln!("Cranelift IR that failed verification (main):");
                eprintln!("{}", self.ctx.func.display());
                format!("Failed to define main function: {}", e)
            })?;

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
        function_captures: &HashMap<String, Vec<(String, DataType)>>,
        anonymous_lambdas: &HashMap<usize, String>,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                    function_captures,
                    anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
            ),
            Expr::Block { body, .. } => Self::compile_block_body(
                builder,
                body,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
            ),

            Expr::Variable { name, .. } => Self::compile_variable(builder, name, variables),

            Expr::Assign { name, value, index } => Self::compile_assign(
                builder,
                name,
                value,
                index,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
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
                function_captures,
                anonymous_lambdas,
            ),

            // Function definitions (handled in preprocessing, so return Unit here)
            Expr::DefineFunction { .. } => Ok(None),

            // Type definitions (compile-time only, return Unit)
            Expr::DefineType { .. } => Ok(None),

            // Lambda expressions - return function pointer
            Expr::Lambda { environment, .. } => {
                // Look up the lambda's compiled function name using its environment ID
                let fn_name = anonymous_lambdas
                    .get(environment)
                    .ok_or_else(|| format!("Anonymous lambda at scope {} not found in compiled functions", environment))?;

                // Get function reference
                let func_ref = user_func_refs
                    .get(fn_name)
                    .ok_or_else(|| format!("Function {} not declared in current scope", fn_name))?;

                // Get function address as i64 (function pointer)
                let func_addr = builder.ins().func_addr(types::I64, *func_ref);
                Ok(Some(func_addr))
            }

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
        scope_allocations: &mut [Vec<(Value, String)>],
        ptr: Value,
        type_name: &str,
    ) {
        if let Some(current_scope) = scope_allocations.last_mut() {
            current_scope.push((ptr, type_name.to_string()));
        }
    }

    /// Remove an allocation from tracking (used for return values that escape the scope)
    pub(super) fn untrack_allocation(scope_allocations: &mut [Vec<(Value, String)>], ptr: Value) {
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
        // Strings use lift_string_drop (takes pointer), other types use lift_X_release
        let func_name = if type_name == "string" {
            "lift_string_drop".to_string()
        } else {
            format!("lift_{}_release", type_name)
        };
        if let Some(&func_ref) = runtime_funcs.get(&func_name) {
            builder.ins().call(func_ref, &[ptr]);
        }
    }

    /// Emit a call to the appropriate retain function for a heap-allocated value
    /// This increments the reference count so the value survives when the original scope ends
    pub(super) fn emit_retain_call(
        builder: &mut FunctionBuilder,
        runtime_funcs: &HashMap<String, FuncRef>,
        ptr: Value,
        type_name: &str,
    ) {
        // Strings use lift_string_retain, other types use lift_X_retain
        let func_name = if type_name == "string" {
            "lift_string_retain".to_string()
        } else {
            format!("lift_{}_retain", type_name)
        };
        if let Some(&func_ref) = runtime_funcs.get(&func_name) {
            builder.ins().call(func_ref, &[ptr]);
        }
    }
}
