// Struct compilation methods for Cranelift code generation

use super::types::{data_type_to_type_tag, VarInfo};
use super::CodeGenerator;
use crate::symboltable::SymbolTable;
use crate::syntax::Expr;
use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use cranelift_module::Module;
use std::collections::HashMap;

impl<'a, M: Module> CodeGenerator<'a, M> {
    pub(super) fn compile_struct_literal(
        builder: &mut FunctionBuilder,
        type_name: &str,
        fields: &[(String, Expr)],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::DataType;

        // Create C string for type name on stack
        let type_name_bytes = type_name.as_bytes();
        let type_name_len = type_name_bytes.len() + 1; // +1 for null terminator
        let type_name_slot =
            builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                type_name_len as u32,
                0,
            ));

        // Store type name bytes
        for (i, byte) in type_name_bytes.iter().enumerate() {
            let byte_val = builder.ins().iconst(types::I8, *byte as i64);
            builder
                .ins()
                .stack_store(byte_val, type_name_slot, i as i32);
        }
        // Null terminator
        let null_byte = builder.ins().iconst(types::I8, 0);
        builder
            .ins()
            .stack_store(null_byte, type_name_slot, type_name_bytes.len() as i32);

        let type_name_ptr = builder.ins().stack_addr(types::I64, type_name_slot, 0);

        // Create new struct: lift_struct_new(type_name, field_count)
        let field_count = builder.ins().iconst(types::I64, fields.len() as i64);
        let func_ref = runtime_funcs
            .get("lift_struct_new")
            .ok_or_else(|| "Runtime function lift_struct_new not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[type_name_ptr, field_count]);
        let struct_ptr = builder.inst_results(inst)[0];

        // Set each field in the struct
        let set_field_ref = runtime_funcs
            .get("lift_struct_set_field")
            .ok_or_else(|| "Runtime function lift_struct_set_field not found".to_string())?;

        for (field_name, field_expr) in fields {
            // Create C string for field name
            let field_name_bytes = field_name.as_bytes();
            let field_name_len = field_name_bytes.len() + 1;
            let field_name_slot =
                builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    field_name_len as u32,
                    0,
                ));

            for (i, byte) in field_name_bytes.iter().enumerate() {
                let byte_val = builder.ins().iconst(types::I8, *byte as i64);
                builder
                    .ins()
                    .stack_store(byte_val, field_name_slot, i as i32);
            }
            let null_byte = builder.ins().iconst(types::I8, 0);
            builder
                .ins()
                .stack_store(null_byte, field_name_slot, field_name_bytes.len() as i32);

            let field_name_ptr = builder.ins().stack_addr(types::I64, field_name_slot, 0);

            // Compile field value
            let field_val_raw = Self::compile_expr_static(
                builder,
                field_expr,
                symbols,
                runtime_funcs,
                user_func_refs,
                variables,
            )?
            .ok_or_else(|| format!("Struct field '{}' must produce a value", field_name))?;

            // Determine field type for type tag
            let field_type =
                crate::semantic::determine_type_with_symbols(field_expr, symbols, 0)
                    .ok_or_else(|| format!("Cannot determine type of field '{}'", field_name))?;
            let field_type_resolved = Self::resolve_type_alias(&field_type, symbols);

            // Convert value to i64 for storage
            let field_val = match field_type_resolved {
                DataType::Flt => {
                    // Bitcast f64 to i64 for storage
                    builder
                        .ins()
                        .bitcast(types::I64, MemFlags::new(), field_val_raw)
                }
                DataType::Bool
                | DataType::Int
                | DataType::Str
                | DataType::List { .. }
                | DataType::Map { .. }
                | DataType::Range(_)
                | DataType::Struct(_) => {
                    // Already I64 (integers, booleans, and pointers)
                    field_val_raw
                }
                _ => field_val_raw,
            };

            let type_tag = builder.ins().iconst(
                types::I8,
                data_type_to_type_tag(&field_type_resolved) as i64,
            );

            // Call lift_struct_set_field(struct, field_name, type_tag, value)
            builder.ins().call(
                *set_field_ref,
                &[struct_ptr, field_name_ptr, type_tag, field_val],
            );
        }

        Ok(Some(struct_ptr))
    }

    pub(super) fn compile_field_access(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        field_name: &str,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Compile the struct expression
        let struct_val = Self::compile_expr_static(
            builder,
            expr,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or_else(|| "Field access expression must produce a value".to_string())?;

        // Create C string for field name
        let field_name_bytes = field_name.as_bytes();
        let field_name_len = field_name_bytes.len() + 1;
        let field_name_slot =
            builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                field_name_len as u32,
                0,
            ));

        for (i, byte) in field_name_bytes.iter().enumerate() {
            let byte_val = builder.ins().iconst(types::I8, *byte as i64);
            builder
                .ins()
                .stack_store(byte_val, field_name_slot, i as i32);
        }
        let null_byte = builder.ins().iconst(types::I8, 0);
        builder
            .ins()
            .stack_store(null_byte, field_name_slot, field_name_bytes.len() as i32);

        let field_name_ptr = builder.ins().stack_addr(types::I64, field_name_slot, 0);

        // Call lift_struct_get_field(struct, field_name) -> i64
        let func_ref = runtime_funcs
            .get("lift_struct_get_field")
            .ok_or_else(|| "Runtime function lift_struct_get_field not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[struct_val, field_name_ptr]);
        let field_val = builder.inst_results(inst)[0];

        // TODO: May need type conversion based on field type (e.g., bitcast for floats)
        // For now, return as-is (works for Int, Bool, and pointer types)
        Ok(Some(field_val))
    }

    pub(super) fn compile_field_assign(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        field_name: &str,
        value: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::DataType;

        // Get struct pointer from variable
        // The expr should be a Variable expression
        let struct_ptr = match expr {
            Expr::Variable { name, .. } => Self::compile_variable(builder, name, variables)?
                .ok_or_else(|| format!("Variable '{}' not found", name))?,
            _ => return Err("Field assignment requires a variable expression".to_string()),
        };

        // Create C string for field name
        let field_name_bytes = field_name.as_bytes();
        let field_name_len = field_name_bytes.len() + 1;
        let field_name_slot =
            builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                field_name_len as u32,
                0,
            ));

        for (i, byte) in field_name_bytes.iter().enumerate() {
            let byte_val = builder.ins().iconst(types::I8, *byte as i64);
            builder
                .ins()
                .stack_store(byte_val, field_name_slot, i as i32);
        }
        let null_byte = builder.ins().iconst(types::I8, 0);
        builder
            .ins()
            .stack_store(null_byte, field_name_slot, field_name_bytes.len() as i32);

        let field_name_ptr = builder.ins().stack_addr(types::I64, field_name_slot, 0);

        // Compile new value
        let new_val_raw = Self::compile_expr_static(
            builder,
            value,
            symbols,
            runtime_funcs,
            user_func_refs,
            variables,
        )?
        .ok_or_else(|| "Field assignment value must produce a value".to_string())?;

        // Determine value type for type tag
        let value_type = crate::semantic::determine_type_with_symbols(value, symbols, 0)
            .ok_or_else(|| "Cannot determine type of assignment value".to_string())?;
        let value_type_resolved = Self::resolve_type_alias(&value_type, symbols);

        // Convert value to i64 for storage
        let new_val = match value_type_resolved {
            DataType::Flt => builder
                .ins()
                .bitcast(types::I64, MemFlags::new(), new_val_raw),
            DataType::Bool
            | DataType::Int
            | DataType::Str
            | DataType::List { .. }
            | DataType::Map { .. }
            | DataType::Range(_)
            | DataType::Struct(_) => new_val_raw,
            _ => new_val_raw,
        };

        let type_tag = builder.ins().iconst(
            types::I8,
            data_type_to_type_tag(&value_type_resolved) as i64,
        );

        // Call lift_struct_set_field(struct, field_name, type_tag, new_value)
        let func_ref = runtime_funcs
            .get("lift_struct_set_field")
            .ok_or_else(|| "Runtime function lift_struct_set_field not found".to_string())?;
        builder
            .ins()
            .call(*func_ref, &[struct_ptr, field_name_ptr, type_tag, new_val]);

        // Field assignment returns Unit
        Ok(None)
    }
}
