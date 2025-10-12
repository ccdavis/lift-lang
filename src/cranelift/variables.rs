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

        // Determine the Lift type from the Let's data_type if available, otherwise infer from value
        let lift_type_raw = if !matches!(data_type, DataType::Unsolved) {
            data_type.clone()
        } else {
            determine_type_with_symbols(value, symbols, 0)
                .ok_or_else(|| format!("Cannot determine type for variable '{}'", var_name))?
        };

        // Resolve TypeRef to underlying type
        let lift_type = Self::resolve_type_alias(&lift_type_raw, symbols);

        let cranelift_type = match lift_type {
            DataType::Flt => types::F64,
            DataType::Int | DataType::Bool => types::I64,
            DataType::Str | DataType::List { .. } | DataType::Map { .. } => types::I64, // Pointers
            _ => types::I64, // Default to I64
        };

        // Create a stack slot for this variable (8 bytes for I64/F64/pointers)
        let slot =
            builder.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 0));

        // Store the value in the stack slot
        builder.ins().stack_store(val, slot, 0);

        // Remember this variable's stack slot and type
        variables.insert(
            var_name.to_string(),
            VarInfo {
                slot,
                cranelift_type,
            },
        );

        // Let expressions return Unit
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

        // Look up the variable's stack slot
        let var_info = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        // Store the new value in the stack slot
        builder.ins().stack_store(val, var_info.slot, 0);

        // Assignment returns Unit
        Ok(None)
    }
}
