// Collection compilation methods for Cranelift code generation

use super::CodeGenerator;
use super::types::{VarInfo, data_type_to_type_tag};
use crate::syntax::{Expr, DataType};
use crate::symboltable::SymbolTable;
use cranelift::prelude::*;
use cranelift_codegen::ir::FuncRef;
use cranelift_module::Module;
use std::collections::HashMap;

impl<'a, M: Module> CodeGenerator<'a, M> {
    pub(super) fn compile_list_literal(
        builder: &mut FunctionBuilder,
        data_type: &crate::syntax::DataType,
        data: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::DataType;
        use crate::semantic::determine_type_with_symbols;

        // Infer element type from first element if data_type is Unsolved
        let elem_type_raw = if matches!(data_type, DataType::Unsolved) {
            if let Some(first_elem) = data.first() {
                determine_type_with_symbols(first_elem, symbols, 0)
                    .ok_or_else(|| "Cannot determine type of list elements".to_string())?
            } else {
                // Empty list with Unsolved type - use Int as placeholder since it's empty anyway
                // The actual element type doesn't matter for an empty list
                DataType::Int
            }
        } else {
            data_type.clone()
        };

        // Resolve TypeRef to underlying type
        let elem_type = Self::resolve_type_alias(&elem_type_raw, symbols);

        // Create a new list with capacity equal to number of elements
        let capacity = builder.ins().iconst(types::I64, data.len() as i64);
        let type_tag = builder.ins().iconst(types::I8, data_type_to_type_tag(&elem_type) as i64);
        let func_ref = runtime_funcs.get("lift_list_new")
            .ok_or_else(|| "Runtime function lift_list_new not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[capacity, type_tag]);
        let list_ptr = builder.inst_results(inst)[0];

        // Set each element in the list
        let set_func_ref = runtime_funcs.get("lift_list_set")
            .ok_or_else(|| "Runtime function lift_list_set not found".to_string())?;

        for (i, elem) in data.iter().enumerate() {
            // Compile the element value
            let elem_val_raw = Self::compile_expr_static(builder, elem, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or_else(|| "List element must produce a value".to_string())?;

            // Convert value to i64 for storage (handles all types)
            let elem_val = match elem_type {
                DataType::Flt => {
                    // Bitcast f64 to i64 for storage
                    builder.ins().bitcast(types::I64, MemFlags::new(), elem_val_raw)
                }
                DataType::Bool => {
                    // Bool is already I64 in our representation
                    elem_val_raw
                }
                DataType::Int | DataType::Str | DataType::List { .. } | DataType::Map { .. } => {
                    // Already I64 (integers and pointers)
                    elem_val_raw
                }
                _ => elem_val_raw,
            };

            // Call lift_list_set(list, index, value)
            let index = builder.ins().iconst(types::I64, i as i64);
            builder.ins().call(*set_func_ref, &[list_ptr, index, elem_val]);
        }

        Ok(Some(list_ptr))
    }

    pub(super) fn compile_map_literal(
        builder: &mut FunctionBuilder,
        key_type: &crate::syntax::DataType,
        value_type: &crate::syntax::DataType,
        data: &[(crate::syntax::KeyData, Expr)],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::{DataType, KeyData};
        use crate::semantic::determine_type_with_symbols;

        // Infer value type from first element if value_type is Unsolved
        let actual_value_type_raw = if matches!(value_type, DataType::Unsolved) {
            if let Some((_, first_val)) = data.first() {
                determine_type_with_symbols(first_val, symbols, 0)
                    .ok_or_else(|| "Cannot determine type of map values".to_string())?
            } else {
                // Empty map with Unsolved type - use Int as placeholder
                DataType::Int
            }
        } else {
            value_type.clone()
        };

        // Resolve TypeRef to underlying type for value type
        let actual_value_type = Self::resolve_type_alias(&actual_value_type_raw, symbols);

        // Infer key type from first element if key_type is Unsolved
        let actual_key_type_raw = if matches!(key_type, DataType::Unsolved) {
            if let Some((first_key, _)) = data.first() {
                match first_key {
                    KeyData::Int(_) => DataType::Int,
                    KeyData::Str(_) => DataType::Str,
                    KeyData::Bool(_) => DataType::Bool,
                }
            } else {
                // Empty map with Unsolved type - use Int as placeholder
                DataType::Int
            }
        } else {
            key_type.clone()
        };

        // Resolve TypeRef to underlying type for key type
        let actual_key_type = Self::resolve_type_alias(&actual_key_type_raw, symbols);

        // Validate that key type is scalar (Int, Bool, or Str)
        if !matches!(actual_key_type, DataType::Int | DataType::Bool | DataType::Str) {
            return Err(format!("Map keys must be scalar types (Int, Bool, or Str), got {:?}", actual_key_type));
        }

        // Create a new map with capacity equal to number of pairs
        let capacity = builder.ins().iconst(types::I64, data.len() as i64);
        let key_type_tag = builder.ins().iconst(types::I8, data_type_to_type_tag(&actual_key_type) as i64);
        let value_type_tag = builder.ins().iconst(types::I8, data_type_to_type_tag(&actual_value_type) as i64);
        let func_ref = runtime_funcs.get("lift_map_new")
            .ok_or_else(|| "Runtime function lift_map_new not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[capacity, key_type_tag, value_type_tag]);
        let map_ptr = builder.inst_results(inst)[0];

        // Set each key-value pair in the map
        let set_func_ref = runtime_funcs.get("lift_map_set")
            .ok_or_else(|| "Runtime function lift_map_set not found".to_string())?;

        for (key_data, value_expr) in data {
            // Convert key to i64 based on key type
            let key_val = match key_data {
                KeyData::Int(k) => builder.ins().iconst(types::I64, *k),
                KeyData::Bool(b) => builder.ins().iconst(types::I64, if *b { 1 } else { 0 }),
                KeyData::Str(s) => {
                    // For string keys, we need to create a string and use its pointer
                    // This is a simplified approach - in production would need proper string interning
                    let byte_len = s.len() + 1;
                    let slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                        byte_len as u32,
                        0,
                    ));
                    for (i, byte) in s.as_bytes().iter().enumerate() {
                        let byte_val = builder.ins().iconst(types::I8, *byte as i64);
                        builder.ins().stack_store(byte_val, slot, i as i32);
                    }
                    let null_byte = builder.ins().iconst(types::I8, 0);
                    builder.ins().stack_store(null_byte, slot, s.len() as i32);
                    let str_ptr = builder.ins().stack_addr(types::I64, slot, 0);

                    // Call lift_str_new to create heap string
                    let str_new_ref = runtime_funcs.get("lift_str_new")
                        .ok_or_else(|| "Runtime function lift_str_new not found".to_string())?;
                    let inst = builder.ins().call(*str_new_ref, &[str_ptr]);
                    builder.inst_results(inst)[0]
                }
            };

            // Compile the value expression
            let value_val_raw = Self::compile_expr_static(builder, value_expr, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or_else(|| "Map value must produce a value".to_string())?;

            // Convert value to i64 for storage (handles all types)
            let value_val = match actual_value_type {
                DataType::Flt => {
                    // Bitcast f64 to i64 for storage
                    builder.ins().bitcast(types::I64, MemFlags::new(), value_val_raw)
                }
                DataType::Bool => {
                    // Bool is already I64 in our representation
                    value_val_raw
                }
                DataType::Int | DataType::Str | DataType::List { .. } | DataType::Map { .. } => {
                    // Already I64 (integers and pointers)
                    value_val_raw
                }
                _ => value_val_raw,
            };

            // Call lift_map_set(map, key, value)
            builder.ins().call(*set_func_ref, &[map_ptr, key_val, value_val]);
        }

        Ok(Some(map_ptr))
    }

    pub(super) fn compile_index(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        index: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic::determine_type_with_symbols;
        use crate::syntax::DataType;

        // Compile the collection expression
        let collection = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("Index requires non-Unit collection")?;

        // Compile the index expression
        let index_val = Self::compile_expr_static(builder, index, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("Index requires non-Unit index value")?;

        // Determine if this is a list or map based on the type
        let expr_type_raw = determine_type_with_symbols(expr, symbols, 0)
            .ok_or_else(|| "Cannot determine type for indexed expression".to_string())?;

        // Resolve TypeRef to underlying type
        let expr_type = Self::resolve_type_alias(&expr_type_raw, symbols);

        match expr_type {
            DataType::List { element_type } => {
                // Call lift_list_get(list, index) -> i64
                let func_ref = runtime_funcs.get("lift_list_get")
                    .ok_or_else(|| "Runtime function lift_list_get not found".to_string())?;
                let inst = builder.ins().call(*func_ref, &[collection, index_val]);
                let result_i64 = builder.inst_results(inst)[0];

                // Convert i64 back to the proper element type
                let result = match *element_type {
                    DataType::Flt => {
                        // Bitcast i64 back to f64
                        builder.ins().bitcast(types::F64, MemFlags::new(), result_i64)
                    }
                    DataType::Bool | DataType::Int | DataType::Str | DataType::List { .. } | DataType::Map { .. } => {
                        // Already correct type (I64 for bool/int/pointers)
                        result_i64
                    }
                    _ => result_i64,
                };
                Ok(Some(result))
            }
            DataType::Map { value_type, .. } => {
                // Call lift_map_get(map, key) -> i64
                let func_ref = runtime_funcs.get("lift_map_get")
                    .ok_or_else(|| "Runtime function lift_map_get not found".to_string())?;
                let inst = builder.ins().call(*func_ref, &[collection, index_val]);
                let result_i64 = builder.inst_results(inst)[0];

                // Convert i64 back to the proper value type
                let result = match *value_type {
                    DataType::Flt => {
                        // Bitcast i64 back to f64
                        builder.ins().bitcast(types::F64, MemFlags::new(), result_i64)
                    }
                    DataType::Bool | DataType::Int | DataType::Str | DataType::List { .. } | DataType::Map { .. } => {
                        // Already correct type (I64 for bool/int/pointers)
                        result_i64
                    }
                    _ => result_i64,
                };
                Ok(Some(result))
            }
            _ => Err(format!("Cannot index into type: {:?}", expr_type)),
        }
    }

    pub(super) fn compile_len(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic::determine_type_with_symbols;
        use crate::syntax::DataType;

        // Compile the expression
        let val = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("len() requires non-Unit expression")?;

        // Determine the type to call the right len function
        let expr_type_raw = determine_type_with_symbols(expr, symbols, 0)
            .ok_or_else(|| "Cannot determine type for len() expression".to_string())?;

        // Resolve TypeRef to underlying type
        let expr_type = Self::resolve_type_alias(&expr_type_raw, symbols);

        let func_name = match expr_type {
            DataType::Str => "lift_str_len",
            DataType::List { .. } => "lift_list_len",
            DataType::Map { .. } => "lift_map_len",
            _ => return Err(format!("len() not supported for type: {:?}", expr_type)),
        };

        let func_ref = runtime_funcs.get(func_name)
            .ok_or_else(|| format!("Runtime function {} not found", func_name))?;
        let inst = builder.ins().call(*func_ref, &[val]);
        let result = builder.inst_results(inst)[0];
        Ok(Some(result))
    }

}
