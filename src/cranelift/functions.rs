// Function compilation methods for Cranelift code generation

use super::types::VarInfo;
use super::CodeGenerator;
use crate::symboltable::SymbolTable;
use crate::syntax::{Expr, Function};
use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use cranelift_module::Module;
use std::collections::{HashMap, HashSet};

impl<'a, M: Module> CodeGenerator<'a, M> {
    /// Collect all variable references in an expression tree
    fn collect_variable_refs(expr: &Expr, vars: &mut HashSet<String>) {
        match expr {
            Expr::Variable { name, .. } => {
                vars.insert(name.clone());
            }
            Expr::Program { body, .. } | Expr::Block { body, .. } => {
                for e in body {
                    Self::collect_variable_refs(e, vars);
                }
            }
            Expr::If { cond, then, final_else } => {
                Self::collect_variable_refs(cond, vars);
                Self::collect_variable_refs(then, vars);
                Self::collect_variable_refs(final_else, vars);
            }
            Expr::While { cond, body } => {
                Self::collect_variable_refs(cond, vars);
                Self::collect_variable_refs(body, vars);
            }
            Expr::Let { value, .. } => {
                Self::collect_variable_refs(value, vars);
            }
            Expr::Assign { value, .. } => {
                Self::collect_variable_refs(value, vars);
            }
            Expr::BinaryExpr { left, right, .. } => {
                Self::collect_variable_refs(left, vars);
                Self::collect_variable_refs(right, vars);
            }
            Expr::UnaryExpr { expr, .. } => {
                Self::collect_variable_refs(expr, vars);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    Self::collect_variable_refs(&arg.value, vars);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                Self::collect_variable_refs(receiver, vars);
                for arg in args {
                    Self::collect_variable_refs(&arg.value, vars);
                }
            }
            Expr::Output { data } => {
                for e in data {
                    Self::collect_variable_refs(e, vars);
                }
            }
            Expr::Len { expr } => {
                Self::collect_variable_refs(expr, vars);
            }
            Expr::Index { expr, index } => {
                Self::collect_variable_refs(expr, vars);
                Self::collect_variable_refs(index, vars);
            }
            Expr::ListLiteral { data, .. } => {
                for e in data {
                    Self::collect_variable_refs(e, vars);
                }
            }
            Expr::MapLiteral { data, .. } => {
                // Keys are KeyData (literals), only values can have variable refs
                for (_k, v) in data {
                    Self::collect_variable_refs(v, vars);
                }
            }
            // Range uses LiteralData which are literals, no variable refs
            Expr::Range(_, _) => {}
            Expr::StructLiteral { fields, .. } => {
                for (_name, value) in fields {
                    Self::collect_variable_refs(value, vars);
                }
            }
            Expr::FieldAccess { expr, .. } => {
                Self::collect_variable_refs(expr, vars);
            }
            Expr::FieldAssign { expr, value, .. } => {
                Self::collect_variable_refs(expr, vars);
                Self::collect_variable_refs(value, vars);
            }
            // Literals and other leaf nodes don't reference variables
            _ => {}
        }
    }

    /// Collect all locally declared variables in an expression tree
    fn collect_local_declarations(expr: &Expr, locals: &mut HashSet<String>) {
        match expr {
            Expr::Let { var_name, value, .. } => {
                locals.insert(var_name.clone());
                Self::collect_local_declarations(value, locals);
            }
            Expr::Program { body, .. } | Expr::Block { body, .. } => {
                for e in body {
                    Self::collect_local_declarations(e, locals);
                }
            }
            Expr::If { cond, then, final_else } => {
                Self::collect_local_declarations(cond, locals);
                Self::collect_local_declarations(then, locals);
                Self::collect_local_declarations(final_else, locals);
            }
            Expr::While { cond, body } => {
                Self::collect_local_declarations(cond, locals);
                Self::collect_local_declarations(body, locals);
            }
            _ => {}
        }
    }

    /// Collect all function calls in an expression tree
    fn collect_function_calls(expr: &Expr, calls: &mut HashSet<String>) {
        match expr {
            Expr::Call { fn_name, args, .. } => {
                calls.insert(fn_name.clone());
                for arg in args {
                    Self::collect_function_calls(&arg.value, calls);
                }
            }
            Expr::Program { body, .. } | Expr::Block { body, .. } => {
                for e in body {
                    Self::collect_function_calls(e, calls);
                }
            }
            Expr::If { cond, then, final_else } => {
                Self::collect_function_calls(cond, calls);
                Self::collect_function_calls(then, calls);
                Self::collect_function_calls(final_else, calls);
            }
            Expr::While { cond, body } => {
                Self::collect_function_calls(cond, calls);
                Self::collect_function_calls(body, calls);
            }
            Expr::Let { value, .. } => {
                Self::collect_function_calls(value, calls);
            }
            Expr::Assign { value, .. } => {
                Self::collect_function_calls(value, calls);
            }
            Expr::BinaryExpr { left, right, .. } => {
                Self::collect_function_calls(left, calls);
                Self::collect_function_calls(right, calls);
            }
            Expr::UnaryExpr { expr, .. } => {
                Self::collect_function_calls(expr, calls);
            }
            Expr::MethodCall { receiver, args, .. } => {
                Self::collect_function_calls(receiver, calls);
                for arg in args {
                    Self::collect_function_calls(&arg.value, calls);
                }
            }
            Expr::Output { data } => {
                for e in data {
                    Self::collect_function_calls(e, calls);
                }
            }
            _ => {}
        }
    }

    /// Check if a function captures any variables from outer scopes (direct captures only)
    pub(super) fn find_direct_captured_variables(function: &Function) -> Vec<String> {
        // Collect all variable references in the function body
        let mut referenced_vars = HashSet::new();
        Self::collect_variable_refs(&function.body, &mut referenced_vars);

        // Collect parameter names
        let param_names: HashSet<String> = function.params.iter().map(|p| p.name.clone()).collect();

        // Collect locally declared variables
        let mut local_vars = HashSet::new();
        Self::collect_local_declarations(&function.body, &mut local_vars);

        // Find variables that are referenced but not declared locally or as parameters
        referenced_vars
            .into_iter()
            .filter(|v| !param_names.contains(v) && !local_vars.contains(v))
            .collect()
    }

    /// Find captured variables including transitive captures from called functions
    /// This requires a map of all functions and their direct captures
    pub(super) fn find_captured_variables_with_transitive(
        function: &Function,
        all_captures: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        // Start with direct captures
        let mut captures: HashSet<String> = Self::find_direct_captured_variables(function)
            .into_iter()
            .collect();

        // Get parameter names to exclude
        let param_names: HashSet<String> = function.params.iter().map(|p| p.name.clone()).collect();

        // Collect locally declared variables
        let mut local_vars = HashSet::new();
        Self::collect_local_declarations(&function.body, &mut local_vars);

        // Find all function calls in the body
        let mut called_functions = HashSet::new();
        Self::collect_function_calls(&function.body, &mut called_functions);

        // Add transitive captures from called functions
        for called_fn in &called_functions {
            if let Some(fn_captures) = all_captures.get(called_fn) {
                for cap in fn_captures {
                    // Only add if not a parameter or local variable
                    if !param_names.contains(cap) && !local_vars.contains(cap) {
                        captures.insert(cap.clone());
                    }
                }
            }
        }

        captures.into_iter().collect()
    }

    /// Backward compatibility wrapper
    fn find_captured_variables(function: &Function) -> Vec<String> {
        Self::find_direct_captured_variables(function)
    }

    /// Collect all function definitions from an expression tree
    pub(super) fn collect_function_definitions<'e>(
        &self,
        expr: &'e Expr,
        functions: &mut Vec<(&'e str, &'e Expr)>,
    ) {
        match expr {
            Expr::DefineFunction { fn_name, value, .. } => {
                functions.push((fn_name, value));
            }
            Expr::Program { body, .. } | Expr::Block { body, .. } => {
                for e in body {
                    self.collect_function_definitions(e, functions);
                }
            }
            Expr::If {
                cond,
                then,
                final_else,
            } => {
                self.collect_function_definitions(cond, functions);
                self.collect_function_definitions(then, functions);
                self.collect_function_definitions(final_else, functions);
            }
            Expr::While { cond, body } => {
                self.collect_function_definitions(cond, functions);
                self.collect_function_definitions(body, functions);
            }
            Expr::Let { value, .. } => {
                self.collect_function_definitions(value, functions);
            }
            Expr::Assign { value, .. } => {
                self.collect_function_definitions(value, functions);
            }
            // Check Call arguments for anonymous lambdas
            Expr::Call { args, .. } => {
                for arg in args {
                    self.collect_function_definitions(&arg.value, functions);
                }
            }
            // Check MethodCall arguments for anonymous lambdas
            Expr::MethodCall { receiver, args, .. } => {
                self.collect_function_definitions(receiver, functions);
                for arg in args {
                    self.collect_function_definitions(&arg.value, functions);
                }
            }
            // Check binary expressions
            Expr::BinaryExpr { left, right, .. } => {
                self.collect_function_definitions(left, functions);
                self.collect_function_definitions(right, functions);
            }
            // Check unary expressions
            Expr::UnaryExpr { expr, .. } => {
                self.collect_function_definitions(expr, functions);
            }
            _ => {} // Other expressions don't contain function definitions
        }
    }

    /// Collect anonymous lambdas from an expression tree
    /// Returns a vector of (generated_name, lambda_expr, environment_id)
    pub(super) fn collect_anonymous_lambdas<'e>(
        &self,
        expr: &'e Expr,
        lambdas: &mut Vec<(String, &'e Expr, usize)>,
        counter: &mut usize,
    ) {
        match expr {
            // Anonymous lambda found - generate a name for it
            Expr::Lambda { environment, .. } => {
                let name = format!("__lambda_{}", *counter);
                *counter += 1;
                lambdas.push((name, expr, *environment));
            }
            // Skip named function definitions - they're handled separately
            Expr::DefineFunction { value, .. } => {
                // Don't collect the lambda inside DefineFunction as anonymous
                // But do recurse into the body to find nested anonymous lambdas
                if let Expr::Lambda { value: func, .. } = value.as_ref() {
                    self.collect_anonymous_lambdas(&func.body, lambdas, counter);
                }
            }
            // Recurse into container expressions
            Expr::Program { body, .. } | Expr::Block { body, .. } => {
                for e in body {
                    self.collect_anonymous_lambdas(e, lambdas, counter);
                }
            }
            Expr::If { cond, then, final_else } => {
                self.collect_anonymous_lambdas(cond, lambdas, counter);
                self.collect_anonymous_lambdas(then, lambdas, counter);
                self.collect_anonymous_lambdas(final_else, lambdas, counter);
            }
            Expr::While { cond, body } => {
                self.collect_anonymous_lambdas(cond, lambdas, counter);
                self.collect_anonymous_lambdas(body, lambdas, counter);
            }
            Expr::Let { value, .. } => {
                self.collect_anonymous_lambdas(value, lambdas, counter);
            }
            Expr::Assign { value, .. } => {
                self.collect_anonymous_lambdas(value, lambdas, counter);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    self.collect_anonymous_lambdas(&arg.value, lambdas, counter);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.collect_anonymous_lambdas(receiver, lambdas, counter);
                for arg in args {
                    self.collect_anonymous_lambdas(&arg.value, lambdas, counter);
                }
            }
            Expr::BinaryExpr { left, right, .. } => {
                self.collect_anonymous_lambdas(left, lambdas, counter);
                self.collect_anonymous_lambdas(right, lambdas, counter);
            }
            Expr::UnaryExpr { expr, .. } => {
                self.collect_anonymous_lambdas(expr, lambdas, counter);
            }
            Expr::ListLiteral { data, .. } => {
                for e in data {
                    self.collect_anonymous_lambdas(e, lambdas, counter);
                }
            }
            Expr::Index { expr, index } => {
                self.collect_anonymous_lambdas(expr, lambdas, counter);
                self.collect_anonymous_lambdas(index, lambdas, counter);
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, value) in fields {
                    self.collect_anonymous_lambdas(value, lambdas, counter);
                }
            }
            Expr::FieldAccess { expr, .. } => {
                self.collect_anonymous_lambdas(expr, lambdas, counter);
            }
            Expr::FieldAssign { expr, value, .. } => {
                self.collect_anonymous_lambdas(expr, lambdas, counter);
                self.collect_anonymous_lambdas(value, lambdas, counter);
            }
            Expr::Output { data } => {
                for e in data {
                    self.collect_anonymous_lambdas(e, lambdas, counter);
                }
            }
            Expr::Len { expr } => {
                self.collect_anonymous_lambdas(expr, lambdas, counter);
            }
            _ => {} // Literals, variables, etc. don't contain lambdas
        }
    }

    /// Compile a user-defined function
    pub(super) fn compile_user_function(
        &mut self,
        fn_name: &str,
        lambda_expr: &Expr,
        symbols: &SymbolTable,
    ) -> Result<(), String> {
        // Extract the Lambda
        let function = match lambda_expr {
            Expr::Lambda { value, .. } => value,
            _ => {
                return Err(format!(
                    "DefineFunction value must be a Lambda, got: {:?}",
                    lambda_expr
                ))
            }
        };

        // Get pre-computed captures (including transitive captures from called functions)
        // These were computed in compile_program before compiling any functions
        let captured_vars = self
            .function_captures
            .get(fn_name)
            .cloned()
            .unwrap_or_default();

        // Build Cranelift function signature
        let mut sig = self.module.make_signature();
        let pointer_type = self.module.target_config().pointer_type();

        // Resolve return type to check if it's a string
        let resolved_return_type = Self::resolve_type_alias(&function.return_type, symbols);
        let returns_string = matches!(resolved_return_type, crate::syntax::DataType::Str);

        // For string-returning functions, add a hidden first parameter for the result pointer
        // This is needed because string operations allocate on the callee's stack, which becomes
        // invalid after the function returns. By passing a dest pointer, the caller controls
        // where the result is stored.
        if returns_string {
            sig.params.push(AbiParam::new(pointer_type)); // Hidden dest pointer
        }

        // Add parameters (resolve TypeRef to underlying types first)
        for param in &function.params {
            let resolved_param_type = Self::resolve_type_alias(&param.data_type, symbols);
            let param_type = Self::data_type_to_cranelift_type(&resolved_param_type, pointer_type);
            sig.params.push(AbiParam::new(param_type));
        }

        // Add captured variables as hidden parameters (after regular params)
        for (_name, var_type) in &captured_vars {
            let resolved_type = Self::resolve_type_alias(var_type, symbols);
            let param_type = Self::data_type_to_cranelift_type(&resolved_type, pointer_type);
            sig.params.push(AbiParam::new(param_type));
        }

        // Add return type (all functions have a return type in Lift)
        let return_type = Self::data_type_to_cranelift_type(&resolved_return_type, pointer_type);
        sig.returns.push(AbiParam::new(return_type));

        // Declare the function
        let func_id = self
            .module
            .declare_function(fn_name, cranelift_module::Linkage::Local, &sig)
            .map_err(|e| format!("Failed to declare function {}: {}", fn_name, e))?;

        // Store function reference
        self.function_refs.insert(fn_name.to_string(), func_id);

        // Create a new context for this function
        let mut func_ctx = self.module.make_context();
        func_ctx.func.signature = sig.clone();

        // Build the function body
        {
            let mut builder = FunctionBuilder::new(&mut func_ctx.func, &mut self.builder_context);

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

            // Declare other user functions in this scope (for recursion and mutual recursion)
            let mut user_func_refs = HashMap::new();
            for (name, func_id) in &self.function_refs {
                let func_ref = self.module.declare_func_in_func(*func_id, builder.func);
                user_func_refs.insert(name.clone(), func_ref);
            }

            // Get function parameters as Cranelift values
            let block_params = builder.block_params(entry_block).to_vec();

            // For string-returning functions, first param is the hidden dest pointer
            let (dest_ptr, param_offset) = if returns_string {
                (Some(block_params[0]), 1)
            } else {
                (None, 0)
            };

            // Create variables for parameters
            let mut variables = HashMap::new();
            for (i, param) in function.params.iter().enumerate() {
                let param_value = block_params[i + param_offset];
                // Resolve TypeRef to underlying type
                let resolved_param_type = Self::resolve_type_alias(&param.data_type, symbols);
                let param_type =
                    Self::data_type_to_cranelift_type(&resolved_param_type, pointer_type);

                if param.copy {
                    // cpy parameter: allocate stack slot and store value
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8, // 8 bytes for i64/f64/pointer
                        0,
                    ));
                    builder.ins().stack_store(param_value, slot, 0);
                    variables.insert(
                        param.name.clone(),
                        VarInfo {
                            slot,
                            cranelift_type: param_type,
                            is_inline_string: false, // Parameters are pointers, not inline
                        },
                    );
                } else {
                    // Regular parameter: create stack slot for immutable access
                    // (we can't reassign to block params, so we store them)
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8,
                        0,
                    ));
                    builder.ins().stack_store(param_value, slot, 0);
                    variables.insert(
                        param.name.clone(),
                        VarInfo {
                            slot,
                            cranelift_type: param_type,
                            is_inline_string: false, // Parameters are pointers, not inline
                        },
                    );
                }
            }

            // Create variables for captured parameters (from outer scopes)
            let capture_offset = param_offset + function.params.len();
            for (i, (var_name, var_type)) in captured_vars.iter().enumerate() {
                let param_value = block_params[i + capture_offset];
                let resolved_type = Self::resolve_type_alias(var_type, symbols);
                let cranelift_type = Self::data_type_to_cranelift_type(&resolved_type, pointer_type);

                // Create stack slot for captured variable (immutable access)
                let slot = builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    8,
                    0,
                ));
                builder.ins().stack_store(param_value, slot, 0);
                variables.insert(
                    var_name.clone(),
                    VarInfo {
                        slot,
                        cranelift_type,
                        is_inline_string: false,
                    },
                );
            }

            // Compile function body with scope tracking
            let mut scope_allocations = Vec::new();
            Self::enter_scope(&mut scope_allocations);

            let result = Self::compile_expr_static(
                &mut builder,
                &function.body,
                symbols,
                &runtime_refs,
                &user_func_refs,
                &mut variables,
                &mut scope_allocations,
                &self.function_captures,
                &self.anonymous_lambdas,
            )?;

            // Handle return value - all functions must return a value
            let return_value =
                result.ok_or_else(|| format!("Function '{}' must return a value", fn_name))?;

            // Untrack return value so it doesn't get released
            Self::untrack_allocation(&mut scope_allocations, return_value);

            // Clean up all other allocations before returning
            Self::exit_scope(&mut builder, &runtime_refs, &mut scope_allocations);

            // For string-returning functions, copy the result to the dest pointer
            // This ensures the string survives after this function's stack frame is deallocated
            if let Some(dest) = dest_ptr {
                // Call lift_string_copy(dest, src) to copy the LiftString
                if let Some(&copy_func) = runtime_refs.get("lift_string_copy") {
                    builder.ins().call(copy_func, &[dest, return_value]);
                }
                // Return the dest pointer (caller's stack slot)
                builder.ins().return_(&[dest]);
            } else {
                builder.ins().return_(&[return_value]);
            }

            // Finalize
            builder.finalize();
        }

        // DEBUG: Print the Cranelift IR for this function
        if std::env::var("LIFT_DEBUG_IR").is_ok() {
            eprintln!("=== Cranelift IR for function {} ===", fn_name);
            eprintln!("{}", func_ctx.func.display());
        }

        // Define the function in the module
        self.module
            .define_function(func_id, &mut func_ctx)
            .map_err(|e| {
                eprintln!("Cranelift IR that failed verification:");
                eprintln!("{}", func_ctx.func.display());
                format!("Failed to define function {}: {}", fn_name, e)
            })?;

        // Clear context
        self.module.clear_context(&mut func_ctx);

        Ok(())
    }

    pub(super) fn compile_function_call(
        builder: &mut FunctionBuilder,
        fn_name: &str,
        args: &[crate::syntax::KeywordArg],
        index: &(usize, usize),
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
        function_captures: &HashMap<String, Vec<(String, crate::syntax::DataType)>>,
        anonymous_lambdas: &HashMap<usize, String>,
    ) -> Result<Option<Value>, String> {
        // Check if this is UFCS (Uniform Function Call Syntax)
        // UFCS calls have first argument named "self"
        if let Some(first_arg) = args.first() {
            if first_arg.name == "self" {
                // This is UFCS - convert to method call
                let remaining_args = &args[1..];
                return Self::compile_method_call(
                    builder,
                    &first_arg.value,
                    fn_name,
                    remaining_args,
                    symbols,
                    runtime_funcs,
                    user_func_refs,
                    variables,
                    scope_allocations,
                    function_captures,
                    anonymous_lambdas,
                );
            }
        }

        // Check if this is an indirect call (calling through a Fn-typed variable)
        // This happens when a function takes a function as a parameter and calls it
        if let Some(var_type) = symbols.get_symbol_type(index) {
            if let crate::syntax::DataType::Fn { params, return_type } = var_type {
                return Self::compile_indirect_call(
                    builder,
                    fn_name,
                    args,
                    &params,
                    &return_type,
                    symbols,
                    runtime_funcs,
                    user_func_refs,
                    variables,
                    scope_allocations,
                    function_captures,
                    anonymous_lambdas,
                );
            }
        }

        // Regular function call - look up the function reference
        let func_ref = user_func_refs
            .get(fn_name)
            .ok_or_else(|| format!("Undefined function: {}", fn_name))?;

        // Get function from symbol table to determine parameter order
        let func_expr = symbols
            .get_symbol_value(index)
            .ok_or_else(|| format!("Function {} not in symbol table", fn_name))?;

        // Extract the Function from DefineFunction -> Lambda or directly from Lambda
        let function = match func_expr {
            Expr::DefineFunction { value, .. } => match value.as_ref() {
                Expr::Lambda { value: f, .. } => f,
                _ => {
                    return Err(format!(
                        "{} DefineFunction does not contain Lambda",
                        fn_name
                    ))
                }
            },
            Expr::Lambda { value: f, .. } => f,
            _ => {
                return Err(format!(
                    "{} is not a function (got: {:?})",
                    fn_name, func_expr
                ))
            }
        };

        // Get parameter names in order
        let param_names = function
            .params
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>();

        // Check if function returns a string (needs hidden dest pointer)
        let resolved_return_type = Self::resolve_type_alias(&function.return_type, symbols);
        let returns_string = matches!(resolved_return_type, crate::syntax::DataType::Str);

        // For string-returning functions, allocate a result slot on caller's stack
        let result_slot = if returns_string {
            Some(builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    32, // LiftString is 32 bytes
                    8,  // 8-byte alignment
                ),
            ))
        } else {
            None
        };

        // Build argument list
        let mut arg_values = Vec::new();

        // For string-returning functions, first arg is the dest pointer
        if let Some(slot) = result_slot {
            let dest_ptr = builder.ins().stack_addr(types::I64, slot, 0);
            arg_values.push(dest_ptr);
        }

        // Evaluate arguments in parameter order
        for param_name in &param_names {
            let arg = args
                .iter()
                .find(|a| &a.name == param_name)
                .ok_or_else(|| format!("Missing argument: {}", param_name))?;

            let val = Self::compile_expr_static(
                builder,
                &arg.value,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            
                function_captures,
            anonymous_lambdas,)?
            .ok_or_else(|| format!("Function argument '{}' cannot be Unit", param_name))?;

            arg_values.push(val);
        }

        // Check for captured variables and pass them as additional arguments
        // Use pre-computed captures (includes transitive captures)
        if let Some(captured_vars) = function_captures.get(fn_name) {
            for (var_name, _var_type) in captured_vars {
                // Look up the captured variable in the current scope
                let var_info = variables.get(var_name).ok_or_else(|| {
                    format!(
                        "Captured variable '{}' not found in current scope when calling function '{}'",
                        var_name, fn_name
                    )
                })?;

                // Load the variable's value and pass it
                let val = builder
                    .ins()
                    .stack_load(var_info.cranelift_type, var_info.slot, 0);
                arg_values.push(val);
            }
        }

        // Call the function
        let inst = builder.ins().call(*func_ref, &arg_values);

        // Get return value (if any)
        let results = builder.inst_results(inst);
        if results.is_empty() {
            Ok(None) // Unit return
        } else {
            Ok(Some(results[0]))
        }
    }

    /// Compile an indirect function call through a Fn-typed variable
    ///
    /// This is used when calling a function pointer stored in a variable,
    /// such as when a HOF receives a function as a parameter and calls it.
    pub(super) fn compile_indirect_call(
        builder: &mut FunctionBuilder,
        fn_var_name: &str,
        args: &[crate::syntax::KeywordArg],
        param_types: &[crate::syntax::DataType],
        return_type: &crate::syntax::DataType,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
        function_captures: &HashMap<String, Vec<(String, crate::syntax::DataType)>>,
        anonymous_lambdas: &HashMap<usize, String>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::DataType;
        use cranelift::prelude::*;
        use cranelift_codegen::ir::AbiParam;

        // 1. Load the function pointer from the variable
        let var_info = variables
            .get(fn_var_name)
            .ok_or_else(|| format!("Undefined function variable: {}", fn_var_name))?;
        let fn_ptr = builder.ins().stack_load(types::I64, var_info.slot, 0);

        // 2. Create signature for the call based on Fn type
        let mut sig = Signature::new(cranelift_codegen::isa::CallConv::SystemV);

        // Resolve return type
        let resolved_return_type = Self::resolve_type_alias(return_type, symbols);
        let returns_string = matches!(resolved_return_type, DataType::Str);

        // For string-returning functions, add hidden dest pointer as first param
        if returns_string {
            sig.params.push(AbiParam::new(types::I64)); // dest pointer
        }

        // Add parameter types to signature
        for param_type in param_types {
            let resolved = Self::resolve_type_alias(param_type, symbols);
            let cranelift_type = Self::data_type_to_cranelift_type(&resolved, types::I64);
            sig.params.push(AbiParam::new(cranelift_type));
        }

        // Add return type
        let return_cranelift_type = Self::data_type_to_cranelift_type(&resolved_return_type, types::I64);
        sig.returns.push(AbiParam::new(return_cranelift_type));

        // Import the signature
        let sig_ref = builder.import_signature(sig);

        // 3. For string-returning functions, allocate a result slot
        let result_slot = if returns_string {
            Some(builder.create_sized_stack_slot(
                cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    32, // LiftString is 32 bytes
                    8,  // 8-byte alignment
                ),
            ))
        } else {
            None
        };

        // 4. Compile arguments in positional order
        // Fn types don't carry parameter names, so we use positional matching
        let mut arg_values = Vec::new();

        // For string-returning functions, first arg is the dest pointer
        if let Some(slot) = result_slot {
            let dest_ptr = builder.ins().stack_addr(types::I64, slot, 0);
            arg_values.push(dest_ptr);
        }

        // Compile each argument in order
        for arg in args {
            let val = Self::compile_expr_static(
                builder,
                &arg.value,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
                function_captures,
                anonymous_lambdas,
            )?
            .ok_or_else(|| format!("Function argument '{}' cannot be Unit", arg.name))?;
            arg_values.push(val);
        }

        // 5. Make indirect call
        let call = builder.ins().call_indirect(sig_ref, fn_ptr, &arg_values);

        // 6. Get return value
        let results = builder.inst_results(call);
        if results.is_empty() {
            Ok(None)
        } else {
            Ok(Some(results[0]))
        }
    }

    pub(super) fn compile_method_call(
        builder: &mut FunctionBuilder,
        receiver: &Expr,
        method_name: &str,
        args: &[crate::syntax::KeywordArg],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
        function_captures: &std::collections::HashMap<String, Vec<(String, crate::syntax::DataType)>>,
        anonymous_lambdas: &HashMap<usize, String>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic::determine_type_with_symbols;
        use crate::syntax::{BuiltinMethod, DataType};

        // Determine receiver type
        let receiver_type_raw = determine_type_with_symbols(receiver, symbols, 0)
            .ok_or("Cannot determine receiver type for method call")?;

        // Resolve TypeRef to underlying type
        let receiver_type = Self::resolve_type_alias(&receiver_type_raw, symbols);

        // Get type name for method lookup
        // For user-defined types (TypeRef -> Struct), use the TypeRef name
        let type_name = match &receiver_type {
            DataType::Str => "Str",
            DataType::List { .. } => "List",
            DataType::Map { .. } => "Map",
            DataType::Int => "Int",
            DataType::Flt => "Flt",
            DataType::Bool => "Bool",
            DataType::Range(_) => "Range",
            DataType::TypeRef(name) => name.as_str(),
            DataType::Struct { name, .. } => {
                // Structs now carry their type name
                name.as_str()
            }
            _ => return Err(format!("No methods for type: {:?}", receiver_type)),
        };

        // Check if this is a built-in method
        let builtin_opt = BuiltinMethod::from_name(type_name, method_name);

        // Compile receiver
        let receiver_val = Self::compile_expr_static(
            builder,
            receiver,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
            scope_allocations,
        
                function_captures,
            anonymous_lambdas,)?
        .ok_or("Method receiver cannot be Unit")?;

        // Compile arguments and build argument list (receiver is first arg)
        let mut arg_vals = vec![receiver_val];
        for arg in args {
            let val = Self::compile_expr_static(
                builder,
                &arg.value,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
                scope_allocations,
            
                function_captures,
            anonymous_lambdas,)?
            .ok_or_else(|| format!("Method arg '{}' cannot be Unit", arg.name))?;
            arg_vals.push(val);
        }

        // Handle built-in vs user-defined methods
        if let Some(builtin) = builtin_opt {
            // Check if this is a string method that returns a string (needs dest pointer)
            let returns_string = matches!(
                builtin,
                BuiltinMethod::StrUpper
                    | BuiltinMethod::StrLower
                    | BuiltinMethod::StrSubstring
                    | BuiltinMethod::StrTrim
                    | BuiltinMethod::StrReplace
                    | BuiltinMethod::ListJoin
            );

            if returns_string {
                // String methods that return strings use dest-pointer style
                // Allocate result LiftString on stack (32 bytes)
                let result_slot =
                    builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        32,
                        8,
                    ));
                let result_ptr = builder.ins().stack_addr(types::I64, result_slot, 0);

                // Build argument list: (dest_ptr, receiver_ptr, ...other_args)
                let mut call_args = vec![result_ptr];
                call_args.extend_from_slice(&arg_vals);

                // Get runtime function name
                let runtime_func_name = match builtin {
                    BuiltinMethod::StrUpper => "lift_string_upper",
                    BuiltinMethod::StrLower => "lift_string_lower",
                    BuiltinMethod::StrSubstring => "lift_string_substring",
                    BuiltinMethod::StrTrim => "lift_string_trim",
                    BuiltinMethod::StrReplace => "lift_string_replace",
                    BuiltinMethod::ListJoin => "lift_list_join",
                    _ => unreachable!(),
                };

                let func_ref = runtime_funcs
                    .get(runtime_func_name)
                    .ok_or_else(|| format!("Runtime function not found: {}", runtime_func_name))?;

                builder.ins().call(*func_ref, &call_args);

                return Ok(Some(result_ptr));
            }

            // Other methods use the normal calling convention
            let runtime_func_name = match builtin {
                BuiltinMethod::StrContains => "lift_string_contains",
                BuiltinMethod::StrSplit => "lift_string_split",
                BuiltinMethod::StrStartsWith => "lift_string_starts_with",
                BuiltinMethod::StrEndsWith => "lift_string_ends_with",
                BuiltinMethod::StrIsEmpty => "lift_string_is_empty",

                BuiltinMethod::ListFirst => "lift_list_first",
                BuiltinMethod::ListLast => "lift_list_last",
                BuiltinMethod::ListContains => "lift_list_contains",
                BuiltinMethod::ListSlice => "lift_list_slice",
                BuiltinMethod::ListReverse => "lift_list_reverse",
                // ListJoin is handled above with string-returning methods
                BuiltinMethod::ListIsEmpty => "lift_list_is_empty",

                BuiltinMethod::MapKeys => "lift_map_keys",
                BuiltinMethod::MapValues => "lift_map_values",
                BuiltinMethod::MapContainsKey => "lift_map_contains_key",
                BuiltinMethod::MapIsEmpty => "lift_map_is_empty",

                _ => {
                    return Err(format!("Unsupported builtin method: {:?}", builtin));
                }
            };

            // Call runtime function
            let func_ref = runtime_funcs
                .get(runtime_func_name)
                .ok_or_else(|| format!("Runtime function not found: {}", runtime_func_name))?;

            let inst = builder.ins().call(*func_ref, &arg_vals);

            // Handle return value (some methods return i8 booleans that need extending to i64)
            let results = builder.inst_results(inst);
            if results.is_empty() {
                Ok(None)
            } else {
                let result = results[0];
                // Convert i8 bool to i64 if needed
                let needs_extension = matches!(
                    builtin,
                    BuiltinMethod::StrContains
                        | BuiltinMethod::StrStartsWith
                        | BuiltinMethod::StrEndsWith
                        | BuiltinMethod::StrIsEmpty
                        | BuiltinMethod::ListContains
                        | BuiltinMethod::ListIsEmpty
                        | BuiltinMethod::MapContainsKey
                        | BuiltinMethod::MapIsEmpty
                );

                let final_result = if needs_extension {
                    builder.ins().uextend(types::I64, result)
                } else {
                    result
                };

                // Track allocations for methods that return heap-allocated objects
                let allocates_list = matches!(
                    builtin,
                    BuiltinMethod::StrSplit
                        | BuiltinMethod::ListSlice
                        | BuiltinMethod::ListReverse
                        | BuiltinMethod::MapKeys
                        | BuiltinMethod::MapValues
                );

                let allocates_string = matches!(
                    builtin,
                    BuiltinMethod::StrUpper
                        | BuiltinMethod::StrLower
                        | BuiltinMethod::StrSubstring
                        | BuiltinMethod::StrTrim
                        | BuiltinMethod::StrReplace
                        | BuiltinMethod::ListJoin
                );

                if allocates_list {
                    Self::record_allocation(scope_allocations, final_result, "list");
                } else if allocates_string {
                    // Note: Strings are not yet reference counted, but this is future-proof
                    // Self::record_allocation(scope_allocations, final_result, "string");
                }

                Ok(Some(final_result))
            }
        } else {
            // User-defined method - look it up and call as function
            // Try the original type name first (for methods defined on type aliases)
            let original_type_name = match &receiver_type_raw {
                DataType::TypeRef(name) => Some(name.as_str()),
                _ => None,
            };

            // Build candidate method names: try original type first, then resolved type
            let resolved_method_name = format!("{}.{}", type_name, method_name);
            let (func_ref, method_full_name) = if let Some(orig_name) = original_type_name {
                let original_method_name = format!("{}.{}", orig_name, method_name);
                // Try original first
                if let Some(func_ref) = user_func_refs.get(&original_method_name) {
                    (func_ref, original_method_name)
                } else if let Some(func_ref) = user_func_refs.get(&resolved_method_name) {
                    (func_ref, resolved_method_name.clone())
                } else {
                    return Err(format!(
                        "Undefined method: {} (also tried {})",
                        original_method_name, resolved_method_name
                    ));
                }
            } else {
                // No type alias, just use resolved type name
                let func_ref = user_func_refs
                    .get(&resolved_method_name)
                    .ok_or_else(|| format!("Undefined method: {}", resolved_method_name))?;
                (func_ref, resolved_method_name.clone())
            };

            // Look up the method definition to check if it returns a string
            // Methods are stored with their full name (e.g., "Message.exclaim")
            // Search from scope 0 (global scope) where methods are typically defined
            let method_returns_string = symbols
                .find_index_reachable_from(&method_full_name, 0)
                .and_then(|idx| symbols.get_symbol_value(&idx))
                .and_then(|expr| {
                    // Methods might be stored directly as Lambda or wrapped in DefineFunction
                    let lambda = match expr {
                        Expr::DefineFunction { value, .. } => match value.as_ref() {
                            Expr::Lambda { value: f, .. } => Some(f),
                            _ => None,
                        },
                        Expr::Lambda { value: f, .. } => Some(f),
                        _ => None,
                    };

                    lambda.map(|f| {
                        let resolved_ret = Self::resolve_type_alias(&f.return_type, symbols);
                        matches!(resolved_ret, DataType::Str)
                    })
                })
                .unwrap_or(false);

            // For string-returning methods, allocate a result slot on caller's stack
            let result_slot = if method_returns_string {
                Some(builder.create_sized_stack_slot(
                    cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        32, // LiftString is 32 bytes
                        8,  // 8-byte alignment
                    ),
                ))
            } else {
                None
            };

            // Build final argument list
            let mut final_args = Vec::new();

            // For string-returning methods, first arg is the dest pointer
            if let Some(slot) = result_slot {
                let dest_ptr = builder.ins().stack_addr(types::I64, slot, 0);
                final_args.push(dest_ptr);
            }

            // Add the original arguments (receiver + method args)
            final_args.extend_from_slice(&arg_vals);

            let inst = builder.ins().call(*func_ref, &final_args);
            let results = builder.inst_results(inst);
            if results.is_empty() {
                Ok(None)
            } else {
                Ok(Some(results[0]))
            }
        }
    }
}
