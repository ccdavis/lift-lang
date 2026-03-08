// Variable compilation methods for Cranelift code generation

use super::types::VarInfo;
use super::CodeGenerator;
use crate::compile_types::is_heap_type;
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
    pub(super) fn compile_let(
        builder: &mut FunctionBuilder,
        var_name: &str,
        value: &Expr,
        data_type: &DataType,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic::determine_type_with_symbols;

        // Compile the value expression
        let val = Self::compile_expr_static(
            builder,
            value,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or_else(|| format!("Let binding for '{}' requires a value", var_name))?;

        // Check if this variable already exists (rebinding case)
        if let Some(existing_var_info) = variables.get(var_name).cloned() {
            // Variable already exists - reuse its stack slot (like Rust shadowing)

            let lift_type_raw = if !matches!(data_type, DataType::Unsolved) {
                data_type.clone()
            } else {
                determine_type_with_symbols(value, symbols, 0)
                    .ok_or_else(|| format!("Cannot determine type for variable '{}'", var_name))?
            };

            let lift_type = Self::resolve_type_alias(&lift_type_raw, symbols);

            let new_cranelift_type = match lift_type {
                DataType::Flt => types::F64,
                DataType::Int | DataType::Bool => types::I64,
                DataType::Str | DataType::List { .. } | DataType::Map { .. } => types::I64,
                _ => types::I64,
            };

            if new_cranelift_type != existing_var_info.cranelift_type {
                return Err(format!(
                    "Cannot rebind variable '{}' with a different type. Original type: {:?}, New type: {:?}",
                    var_name, existing_var_info.cranelift_type, new_cranelift_type
                ));
            }

            // Release old value before rebinding (if heap type and owned)
            if !existing_var_info.is_param {
                if let Some(ref lt) = existing_var_info.lift_type {
                    if is_heap_type(lt) {
                        let old_val = builder.ins().stack_load(
                            existing_var_info.cranelift_type,
                            existing_var_info.slot,
                            0,
                        );
                        Self::emit_release(builder, old_val, lt, runtime_funcs)?;
                    }
                }
            }

            // If new value comes from a variable, retain it (shared ownership)
            if matches!(value, Expr::Variable { .. }) && is_heap_type(&lift_type) {
                Self::emit_retain(builder, val, &lift_type, runtime_funcs)?;
            }

            builder.ins().stack_store(val, existing_var_info.slot, 0);
            return Ok(None);
        }

        // New variable - create a new stack slot
        let lift_type_raw = if !matches!(data_type, DataType::Unsolved) {
            data_type.clone()
        } else {
            determine_type_with_symbols(value, symbols, 0)
                .ok_or_else(|| format!("Cannot determine type for variable '{}'", var_name))?
        };

        let lift_type = Self::resolve_type_alias(&lift_type_raw, symbols);

        let cranelift_type = match lift_type {
            DataType::Flt => types::F64,
            DataType::Int | DataType::Bool => types::I64,
            DataType::Str | DataType::List { .. } | DataType::Map { .. } => types::I64,
            _ => types::I64,
        };

        // If value comes from a variable, retain it (shared ownership)
        if matches!(value, Expr::Variable { .. }) && is_heap_type(&lift_type) {
            Self::emit_retain(builder, val, &lift_type, runtime_funcs)?;
        }

        let slot =
            builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 0));

        builder.ins().stack_store(val, slot, 0);

        variables.insert(
            var_name.to_string(),
            VarInfo {
                slot,
                cranelift_type,
                lift_type: Some(lift_type),
                is_param: false,
            },
        );

        Ok(None)
    }

    /// Compile a variable reference
    pub(super) fn compile_variable(
        builder: &mut FunctionBuilder,
        name: &str,
        variables: &HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Look up the variable's stack slot and type
        let var_info = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        // Load the value from the stack slot with the correct type
        let val = builder
            .ins()
            .stack_load(var_info.cranelift_type, var_info.slot, 0);
        Ok(Some(val))
    }

    /// Compile an assignment expression
    pub(super) fn compile_assign(
        builder: &mut FunctionBuilder,
        name: &str,
        value: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Compile the new value
        let val = Self::compile_expr_static(
            builder,
            value,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or_else(|| format!("Assignment to '{}' requires a value", name))?;

        let var_info = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?
            .clone();

        // Release old value before overwriting (if heap type and owned)
        if !var_info.is_param {
            if let Some(ref lt) = var_info.lift_type {
                if is_heap_type(lt) {
                    let old_val =
                        builder
                            .ins()
                            .stack_load(var_info.cranelift_type, var_info.slot, 0);
                    Self::emit_release(builder, old_val, lt, runtime_funcs)?;
                }
            }
        }

        // If new value comes from a variable, retain it
        if let Some(ref lt) = var_info.lift_type {
            if matches!(value, Expr::Variable { .. }) && is_heap_type(lt) {
                Self::emit_retain(builder, val, lt, runtime_funcs)?;
            }
        }

        builder.ins().stack_store(val, var_info.slot, 0);
        Ok(None)
    }
}
