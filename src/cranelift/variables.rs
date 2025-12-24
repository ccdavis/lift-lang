// Variable compilation methods for Cranelift code generation

use super::types::VarInfo;
use super::CodeGenerator;
use crate::symboltable::SymbolTable;
use crate::syntax::{DataType, Expr};
use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use cranelift_module::Module;
use std::collections::HashMap;

impl<'a, M: Module> CodeGenerator<'a, M> {
    /// Compile a let binding (variable declaration)
    ///
    /// Like Rust, this supports variable rebinding: if a variable with the same name
    /// already exists, we reuse its stack slot (effectively making it an assignment).
    /// This is essential for `let` declarations inside loop bodies.
    ///
    /// For string variables, we use inline storage: the variable's stack slot contains
    /// the full 32-byte LiftString, not a pointer. This allows proper memory management
    /// when reassigning strings in loops.
    pub(super) fn compile_let(
        builder: &mut FunctionBuilder,
        var_name: &str,
        value: &Expr,
        data_type: &DataType,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
        function_captures: &std::collections::HashMap<String, Vec<(String, crate::syntax::DataType)>>,
        anonymous_lambdas: &HashMap<usize, String>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic::determine_type_with_symbols;

        // Determine the Lift type early (needed to know if this is a string)
        let lift_type_raw = if !matches!(data_type, DataType::Unsolved) {
            data_type.clone()
        } else {
            determine_type_with_symbols(value, symbols, 0)
                .ok_or_else(|| format!("Cannot determine type for variable '{}'", var_name))?
        };
        let lift_type = Self::resolve_type_alias(&lift_type_raw, symbols);
        let is_string = matches!(lift_type, DataType::Str);

        // Compile the value expression
        let val = Self::compile_expr_static(
            builder,
            value,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
            scope_allocations,
        
                function_captures,
            anonymous_lambdas,)?
        .ok_or_else(|| format!("Let binding for '{}' requires a value", var_name))?;

        // Check if this variable already exists (rebinding case)
        if let Some(existing_var_info) = variables.get(var_name).cloned() {
            // Variable already exists - reuse its stack slot (like Rust shadowing)
            // This is crucial for let declarations inside while loops

            if existing_var_info.is_inline_string {
                // For inline strings: drop old value, then copy new value, then drop temp
                let slot_addr = builder.ins().stack_addr(types::I64, existing_var_info.slot, 0);

                // Drop the old string's heap data (if any)
                if let Some(&drop_func) = runtime_funcs.get("lift_string_drop") {
                    builder.ins().call(drop_func, &[slot_addr]);
                }

                // Copy new string to the slot (this increments ref count for large strings)
                if let Some(&copy_func) = runtime_funcs.get("lift_string_copy") {
                    builder.ins().call(copy_func, &[slot_addr, val]);
                }

                // Drop the source temp string (decrements ref count back to 1)
                if let Some(&drop_func) = runtime_funcs.get("lift_string_drop") {
                    builder.ins().call(drop_func, &[val]);
                }
            } else {
                // For non-string heap types, we need to release the old value first
                // to avoid memory leaks when rebinding variables in loops
                let release_type_name = match &lift_type {
                    DataType::List { .. } => Some("list"),
                    DataType::Map { .. } => Some("map"),
                    DataType::Range(_) => Some("range"),
                    DataType::Struct { .. } => Some("struct"),
                    _ => None,
                };

                if let Some(type_name) = release_type_name {
                    // Load the old value before overwriting
                    let old_val = builder
                        .ins()
                        .stack_load(existing_var_info.cranelift_type, existing_var_info.slot, 0);
                    // Store the new value
                    builder.ins().stack_store(val, existing_var_info.slot, 0);
                    // Release the old value (after storing new to preserve memory)
                    Self::emit_release_call(builder, runtime_funcs, old_val, type_name);
                    // Untrack the new value - the variable now owns it, so scope exit
                    // shouldn't release it (it will be released on next rebind or function exit)
                    Self::untrack_allocation(scope_allocations, val);
                } else {
                    // For primitive types, just store the value
                    builder.ins().stack_store(val, existing_var_info.slot, 0);
                }
            }
            return Ok(None);
        }

        // New variable - create a new stack slot

        let cranelift_type = match &lift_type {
            DataType::Flt => types::F64,
            DataType::Int | DataType::Bool => types::I64,
            DataType::Str | DataType::List { .. } | DataType::Map { .. } => types::I64, // Pointers
            _ => types::I64, // Default to I64
        };

        if is_string {
            // String variables use inline storage (32 bytes for full LiftString)
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                32, // Full LiftString
                8,  // 8-byte alignment
            ));

            // Get address of the slot and copy the string into it
            let slot_addr = builder.ins().stack_addr(types::I64, slot, 0);
            if let Some(&copy_func) = runtime_funcs.get("lift_string_copy") {
                builder.ins().call(copy_func, &[slot_addr, val]);
            }

            // Drop the source temp string (decrements ref count back to 1)
            if let Some(&drop_func) = runtime_funcs.get("lift_string_drop") {
                builder.ins().call(drop_func, &[val]);
            }

            // Track this variable for cleanup at scope exit
            // Note: We track the slot address, not val, because the variable persists
            Self::record_allocation(scope_allocations, slot_addr, "string");

            // Remember this variable's stack slot
            variables.insert(
                var_name.to_string(),
                VarInfo {
                    slot,
                    cranelift_type,
                    is_inline_string: true,
                },
            );
        } else {
            // Non-string variables use pointer storage (8 bytes)
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8,
                0,
            ));

            // Check if this is a heap type that needs special handling for loops
            let release_type_name = match &lift_type {
                DataType::List { .. } => Some("list"),
                DataType::Map { .. } => Some("map"),
                DataType::Range(_) => Some("range"),
                DataType::Struct { .. } => Some("struct"),
                _ => None,
            };

            // Store the value in the stack slot
            builder.ins().stack_store(val, slot, 0);

            // For heap types, we need to handle cleanup.
            // The variable now owns the value - untrack from scope_allocations
            // so scope exit doesn't release it (that would cause issues in loops).
            //
            // NOTE: This doesn't handle releasing previous values in loops.
            // That's handled by the while loop compilation which tracks heap vars.
            if release_type_name.is_some() {
                Self::untrack_allocation(scope_allocations, val);
            }

            // Remember this variable's stack slot and type
            variables.insert(
                var_name.to_string(),
                VarInfo {
                    slot,
                    cranelift_type,
                    is_inline_string: false,
                },
            );
        }

        // Let expressions return Unit
        Ok(None)
    }

    /// Compile a variable reference
    ///
    /// For inline string variables, returns a pointer to the variable's slot (the LiftString).
    /// For other types, returns the value stored in the slot.
    pub(super) fn compile_variable(
        builder: &mut FunctionBuilder,
        name: &str,
        variables: &HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Look up the variable's stack slot and type
        let var_info = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        if var_info.is_inline_string {
            // For inline strings, return pointer to the slot (which contains the LiftString)
            let val = builder.ins().stack_addr(types::I64, var_info.slot, 0);
            Ok(Some(val))
        } else {
            // For other types, load the value from the stack slot
            let val = builder
                .ins()
                .stack_load(var_info.cranelift_type, var_info.slot, 0);
            Ok(Some(val))
        }
    }

    /// Compile an assignment expression
    ///
    /// For inline string variables, this:
    /// 1. Compiles the new value (which may reference the old value)
    /// 2. Drops the old string's heap data
    /// 3. Copies the new string into the variable's slot
    pub(super) fn compile_assign(
        builder: &mut FunctionBuilder,
        name: &str,
        value: &Expr,
        index: &(usize, usize),
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
        scope_allocations: &mut Vec<Vec<(Value, String)>>,
        function_captures: &HashMap<String, Vec<(String, crate::syntax::DataType)>>,
        anonymous_lambdas: &HashMap<usize, String>,
    ) -> Result<Option<Value>, String> {
        // Look up the variable's stack slot first
        let var_info = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?
            .clone();

        // Compile the new value FIRST (may reference the old value which is still valid)
        let val = Self::compile_expr_static(
            builder,
            value,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
            scope_allocations,
        
                function_captures,
            anonymous_lambdas,)?
        .ok_or_else(|| format!("Assignment to '{}' requires a value", name))?;

        if var_info.is_inline_string {
            // For inline strings: drop old heap data, then copy new value, then drop temp
            let slot_addr = builder.ins().stack_addr(types::I64, var_info.slot, 0);

            // Drop the old string's heap data (if any)
            // This is safe because we've already compiled the new value
            if let Some(&drop_func) = runtime_funcs.get("lift_string_drop") {
                builder.ins().call(drop_func, &[slot_addr]);
            }

            // Copy new string to the slot (this increments ref count for large strings)
            if let Some(&copy_func) = runtime_funcs.get("lift_string_copy") {
                builder.ins().call(copy_func, &[slot_addr, val]);
            }

            // Drop the source temp string (decrements ref count back to 1)
            // This is necessary because lift_string_copy increments the ref count,
            // but the temp slot will be overwritten without being released
            if let Some(&drop_func) = runtime_funcs.get("lift_string_drop") {
                builder.ins().call(drop_func, &[val]);
            }
        } else {
            use crate::semantic::determine_type_with_symbols;

            // Determine the variable's type to check if it needs cleanup
            // First try the symbol table, then fall back to inferring from value
            let var_type = symbols.get_symbol_type(index);
            let resolved_type = if let Some(t) = var_type {
                Self::resolve_type_alias(&t, symbols)
            } else {
                // Fall back to inferring type from the value expression
                determine_type_with_symbols(value, symbols, 0)
                    .map(|t| Self::resolve_type_alias(&t, symbols))
                    .unwrap_or(DataType::Unsolved)
            };

            // Check if this is a heap type that needs the old value released
            let release_type_name = match &resolved_type {
                DataType::List { .. } => Some("list"),
                DataType::Map { .. } => Some("map"),
                DataType::Range(_) => Some("range"),
                DataType::Struct { .. } => Some("struct"),
                DataType::TypeRef(_) => {
                    let resolved = Self::resolve_type_alias(&resolved_type, symbols);
                    match resolved {
                        DataType::List { .. } => Some("list"),
                        DataType::Map { .. } => Some("map"),
                        DataType::Range(_) => Some("range"),
                        DataType::Struct { .. } => Some("struct"),
                        _ => None,
                    }
                }
                _ => None,
            };

            // Load the old value for later release (if needed)
            let old_val = if release_type_name.is_some() {
                Some(
                    builder
                        .ins()
                        .stack_load(var_info.cranelift_type, var_info.slot, 0),
                )
            } else {
                None
            };

            // RETAIN the new value before storing (increment refcount)
            // This is critical: when we assign z := c, both z and c now reference the same object
            // Without retain, releasing the old z value would leave z's new reference with refcount=1,
            // and when c's scope ends, releasing c would free the object that z still references!
            if let Some(type_name) = release_type_name {
                Self::emit_retain_call(builder, runtime_funcs, val, type_name);
            }

            // Store the new value in the stack slot
            builder.ins().stack_store(val, var_info.slot, 0);

            // Release the old value (after new value is stored)
            if let (Some(type_name), Some(old_val)) = (release_type_name, old_val) {
                Self::emit_release_call(builder, runtime_funcs, old_val, type_name);
            }
        }

        // Assignment returns Unit
        Ok(None)
    }
}
