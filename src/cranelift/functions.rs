// Function compilation methods for Cranelift code generation

use super::CodeGenerator;
use super::types::{VarInfo, data_type_to_cranelift_type, data_type_to_type_tag};
use crate::syntax::{Expr, DataType};
use crate::symboltable::SymbolTable;
use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use cranelift_module::{Module, FuncId};
use std::collections::HashMap;
use std::ffi::CString;

impl<'a, M: Module> CodeGenerator<'a, M> {
    /// Collect all function definitions from an expression tree
    pub(super) fn collect_function_definitions<'e>(&self, expr: &'e Expr, functions: &mut Vec<(&'e str, &'e Expr)>) {
        match expr {
            Expr::DefineFunction { fn_name, value, .. } => {
                functions.push((fn_name, value));
            }
            Expr::Program { body, .. } | Expr::Block { body, .. } => {
                for e in body {
                    self.collect_function_definitions(e, functions);
                }
            }
            Expr::If { cond, then, final_else } => {
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
            _ => {}  // Other expressions don't contain function definitions
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
            _ => return Err(format!("DefineFunction value must be a Lambda, got: {:?}", lambda_expr)),
        };

        // Build Cranelift function signature
        let mut sig = self.module.make_signature();
        let pointer_type = self.module.target_config().pointer_type();

        // Add parameters (resolve TypeRef to underlying types first)
        for param in &function.params {
            let resolved_param_type = Self::resolve_type_alias(&param.data_type, symbols);
            let param_type = Self::data_type_to_cranelift_type(&resolved_param_type, pointer_type);
            sig.params.push(AbiParam::new(param_type));
        }

        // Add return type (all functions have a return type in Lift)
        // Resolve TypeRef to underlying type first
        let resolved_return_type = Self::resolve_type_alias(&function.return_type, symbols);
        let return_type = Self::data_type_to_cranelift_type(&resolved_return_type, pointer_type);
        sig.returns.push(AbiParam::new(return_type));

        // Declare the function
        let func_id = self.module
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
                let func_ref = self.module.declare_func_in_func(*func_id, &mut builder.func);
                runtime_refs.insert(name.clone(), func_ref);
            }

            // Declare other user functions in this scope (for recursion and mutual recursion)
            let mut user_func_refs = HashMap::new();
            for (name, func_id) in &self.function_refs {
                let func_ref = self.module.declare_func_in_func(*func_id, &mut builder.func);
                user_func_refs.insert(name.clone(), func_ref);
            }

            // Get function parameters as Cranelift values
            let block_params = builder.block_params(entry_block).to_vec();

            // Create variables for parameters
            let mut variables = HashMap::new();
            for (i, param) in function.params.iter().enumerate() {
                let param_value = block_params[i];
                // Resolve TypeRef to underlying type
                let resolved_param_type = Self::resolve_type_alias(&param.data_type, symbols);
                let param_type = Self::data_type_to_cranelift_type(&resolved_param_type, pointer_type);

                if param.copy {
                    // cpy parameter: allocate stack slot and store value
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8,  // 8 bytes for i64/f64/pointer
                        0
                    ));
                    builder.ins().stack_store(param_value, slot, 0);
                    variables.insert(param.name.clone(), VarInfo {
                        slot,
                        cranelift_type: param_type,
                    });
                } else {
                    // Regular parameter: create stack slot for immutable access
                    // (we can't reassign to block params, so we store them)
                    let slot = builder.create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        8,
                        0
                    ));
                    builder.ins().stack_store(param_value, slot, 0);
                    variables.insert(param.name.clone(), VarInfo {
                        slot,
                        cranelift_type: param_type,
                    });
                }
            }

            // Compile function body
            let result = Self::compile_expr_static(
                &mut builder,
                &function.body,
                symbols,
                &runtime_refs,
                &user_func_refs,
                &mut variables
            )?;

            // Handle return value - all functions must return a value
            let return_value = result.ok_or_else(|| format!("Function '{}' must return a value", fn_name))?;
            builder.ins().return_(&[return_value]);

            // Finalize
            builder.finalize();
        }

        // Define the function in the module
        self.module
            .define_function(func_id, &mut func_ctx)
            .map_err(|e| format!("Failed to define function {}: {}", fn_name, e))?;

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
                    variables
                );
            }
        }

        // Regular function call - look up the function reference
        let func_ref = user_func_refs.get(fn_name)
            .ok_or_else(|| format!("Undefined function: {}", fn_name))?;

        // Get function from symbol table to determine parameter order
        let func_expr = symbols.get_symbol_value(index)
            .ok_or_else(|| format!("Function {} not in symbol table", fn_name))?;

        // Extract the Function from DefineFunction -> Lambda or directly from Lambda
        let function = match func_expr {
            Expr::DefineFunction { value, .. } => {
                match value.as_ref() {
                    Expr::Lambda { value: f, .. } => f,
                    _ => return Err(format!("{} DefineFunction does not contain Lambda", fn_name)),
                }
            }
            Expr::Lambda { value: f, .. } => f,
            _ => return Err(format!("{} is not a function (got: {:?})", fn_name, func_expr)),
        };

        // Get parameter names in order
        let param_names = function.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>();

        // Evaluate arguments in parameter order
        let mut arg_values = Vec::new();
        for param_name in &param_names {
            let arg = args.iter()
                .find(|a| &a.name == param_name)
                .ok_or_else(|| format!("Missing argument: {}", param_name))?;

            let val = Self::compile_expr_static(
                builder,
                &arg.value,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables
            )?.ok_or_else(|| format!("Function argument '{}' cannot be Unit", param_name))?;

            arg_values.push(val);
        }

        // Call the function
        let inst = builder.ins().call(*func_ref, &arg_values);

        // Get return value (if any)
        let results = builder.inst_results(inst);
        if results.is_empty() {
            Ok(None)  // Unit return
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
    ) -> Result<Option<Value>, String> {
        use crate::semantic::determine_type_with_symbols;
        use crate::syntax::{BuiltinMethod, DataType};

        // Determine receiver type
        let receiver_type_raw = determine_type_with_symbols(receiver, symbols, 0)
            .ok_or("Cannot determine receiver type for method call")?;

        // Resolve TypeRef to underlying type
        let receiver_type = Self::resolve_type_alias(&receiver_type_raw, symbols);

        // Get type name for method lookup
        let type_name = match &receiver_type {
            DataType::Str => "Str",
            DataType::List { .. } => "List",
            DataType::Map { .. } => "Map",
            DataType::Int => "Int",
            DataType::Flt => "Flt",
            DataType::Bool => "Bool",
            DataType::Range(_) => "Range",
            DataType::TypeRef(name) => name.as_str(),
            _ => return Err(format!("No methods for type: {:?}", receiver_type)),
        };

        // Check if this is a built-in method
        let builtin_opt = BuiltinMethod::from_name(type_name, method_name);

        // Compile receiver
        let receiver_val = Self::compile_expr_static(
            builder, receiver, symbols, runtime_funcs, user_func_refs, variables
        )?.ok_or("Method receiver cannot be Unit")?;

        // Compile arguments and build argument list (receiver is first arg)
        let mut arg_vals = vec![receiver_val];
        for arg in args {
            let val = Self::compile_expr_static(
                builder, &arg.value, symbols, runtime_funcs, user_func_refs, variables
            )?.ok_or_else(|| format!("Method arg '{}' cannot be Unit", arg.name))?;
            arg_vals.push(val);
        }

        // Handle built-in vs user-defined methods
        if let Some(builtin) = builtin_opt {
            // Built-in method - map to runtime function
            let runtime_func_name = match builtin {
            BuiltinMethod::StrUpper => "lift_str_upper",
            BuiltinMethod::StrLower => "lift_str_lower",
            BuiltinMethod::StrSubstring => "lift_str_substring",
            BuiltinMethod::StrContains => "lift_str_contains",
            BuiltinMethod::StrTrim => "lift_str_trim",
            BuiltinMethod::StrSplit => "lift_str_split",
            BuiltinMethod::StrReplace => "lift_str_replace",
            BuiltinMethod::StrStartsWith => "lift_str_starts_with",
            BuiltinMethod::StrEndsWith => "lift_str_ends_with",
            BuiltinMethod::StrIsEmpty => "lift_str_is_empty",

            BuiltinMethod::ListFirst => "lift_list_first",
            BuiltinMethod::ListLast => "lift_list_last",
            BuiltinMethod::ListContains => "lift_list_contains",
            BuiltinMethod::ListSlice => "lift_list_slice",
            BuiltinMethod::ListReverse => "lift_list_reverse",
            BuiltinMethod::ListJoin => "lift_list_join",
            BuiltinMethod::ListIsEmpty => "lift_list_is_empty",

            BuiltinMethod::MapKeys => "lift_map_keys",
            BuiltinMethod::MapValues => "lift_map_values",
            BuiltinMethod::MapContainsKey => "lift_map_contains_key",
            BuiltinMethod::MapIsEmpty => "lift_map_is_empty",
        };

            // Call runtime function
            let func_ref = runtime_funcs.get(runtime_func_name)
                .ok_or_else(|| format!("Runtime function not found: {}", runtime_func_name))?;

            let inst = builder.ins().call(*func_ref, &arg_vals);

            // Handle return value (some methods return i8 booleans that need extending to i64)
            let results = builder.inst_results(inst);
            if results.is_empty() {
                Ok(None)
            } else {
                let result = results[0];
                // Convert i8 bool to i64 if needed
                let needs_extension = matches!(builtin,
                    BuiltinMethod::StrContains | BuiltinMethod::StrStartsWith |
                    BuiltinMethod::StrEndsWith | BuiltinMethod::StrIsEmpty |
                    BuiltinMethod::ListContains | BuiltinMethod::ListIsEmpty |
                    BuiltinMethod::MapContainsKey | BuiltinMethod::MapIsEmpty
                );

                if needs_extension {
                    let extended = builder.ins().uextend(types::I64, result);
                    Ok(Some(extended))
                } else {
                    Ok(Some(result))
                }
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
            let func_ref = if let Some(orig_name) = original_type_name {
                let original_method_name = format!("{}.{}", orig_name, method_name);
                // Try original first
                user_func_refs.get(&original_method_name)
                    // Fall back to resolved type
                    .or_else(|| user_func_refs.get(&resolved_method_name))
                    .ok_or_else(|| format!("Undefined method: {} (also tried {})", original_method_name, resolved_method_name))?
            } else {
                // No type alias, just use resolved type name
                user_func_refs.get(&resolved_method_name)
                    .ok_or_else(|| format!("Undefined method: {}", resolved_method_name))?
            };

            let inst = builder.ins().call(*func_ref, &arg_vals);
            let results = builder.inst_results(inst);
            if results.is_empty() {
                Ok(None)
            } else {
                Ok(Some(results[0]))
            }
        }
    }

}
