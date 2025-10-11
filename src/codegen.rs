// Code generation: Translates Lift AST to Cranelift IR

use crate::syntax::{Expr, LiteralData, Operator};
use crate::symboltable::SymbolTable;
use cranelift::prelude::*;
use cranelift_module::{FuncId, Module};
use cranelift_codegen::ir::{FuncRef, StackSlot};
use std::collections::HashMap;

/// Information about a variable in the compiled code
#[derive(Clone, Copy)]
struct VarInfo {
    slot: StackSlot,
    cranelift_type: Type,  // I64, F64, or pointer type
}

/// Convert DataType to runtime type tag (matches constants in runtime.rs)
fn data_type_to_type_tag(data_type: &crate::syntax::DataType) -> i8 {
    use crate::syntax::DataType;
    match data_type {
        DataType::Int => 0,      // TYPE_INT
        DataType::Flt => 1,      // TYPE_FLT
        DataType::Bool => 2,     // TYPE_BOOL
        DataType::Str => 3,      // TYPE_STR
        DataType::List { .. } => 4,  // TYPE_LIST
        DataType::Map { .. } => 5,   // TYPE_MAP
        DataType::Range(_) => 6,     // TYPE_RANGE
        _ => 0,  // Fallback to Int for unknown types
    }
}

pub struct CodeGenerator<'a, M: Module> {
    module: &'a mut M,
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,

    // Variable management: maps Lift variable names to stack slot and type info
    variables: HashMap<String, VarInfo>,

    // Runtime function references
    runtime_funcs: HashMap<String, FuncId>,

    // User-defined function references: maps function names to FuncId
    function_refs: HashMap<String, FuncId>,
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
            function_refs: HashMap::new(),
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

        // lift_output_newline()
        let sig = self.module.make_signature();
        let func_id = self.module
            .declare_function("lift_output_newline", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_newline: {}", e))?;
        self.runtime_funcs.insert("lift_output_newline".to_string(), func_id);

        // lift_output_list(*const LiftList)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_output_list", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_list: {}", e))?;
        self.runtime_funcs.insert("lift_output_list".to_string(), func_id);

        // lift_output_map(*const LiftMap)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_output_map", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_map: {}", e))?;
        self.runtime_funcs.insert("lift_output_map".to_string(), func_id);

        // lift_str_new(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_new: {}", e))?;
        self.runtime_funcs.insert("lift_str_new".to_string(), func_id);

        // lift_str_concat(*const c_char, *const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_concat", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_concat: {}", e))?;
        self.runtime_funcs.insert("lift_str_concat".to_string(), func_id);

        // lift_str_eq(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_str_eq", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_eq: {}", e))?;
        self.runtime_funcs.insert("lift_str_eq".to_string(), func_id);

        // lift_list_new(i64, i8) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I8));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_list_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_new: {}", e))?;
        self.runtime_funcs.insert("lift_list_new".to_string(), func_id);

        // lift_list_set(*mut LiftList, i64, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_list_set", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_set: {}", e))?;
        self.runtime_funcs.insert("lift_list_set".to_string(), func_id);

        // lift_list_get(*const LiftList, i64) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_list_get", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_get: {}", e))?;
        self.runtime_funcs.insert("lift_list_get".to_string(), func_id);

        // lift_map_new(i64, i8, i8) -> *mut LiftMap
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I8));
        sig.params.push(AbiParam::new(types::I8));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_map_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_new: {}", e))?;
        self.runtime_funcs.insert("lift_map_new".to_string(), func_id);

        // lift_map_set(*mut LiftMap, i64, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_map_set", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_set: {}", e))?;
        self.runtime_funcs.insert("lift_map_set".to_string(), func_id);

        // lift_map_get(*const LiftMap, i64) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_map_get", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_get: {}", e))?;
        self.runtime_funcs.insert("lift_map_get".to_string(), func_id);

        // lift_str_len(*const c_char) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_str_len", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_len: {}", e))?;
        self.runtime_funcs.insert("lift_str_len".to_string(), func_id);

        // lift_list_len(*const LiftList) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_list_len", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_len: {}", e))?;
        self.runtime_funcs.insert("lift_list_len".to_string(), func_id);

        // lift_map_len(*const LiftMap) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_map_len", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_len: {}", e))?;
        self.runtime_funcs.insert("lift_map_len".to_string(), func_id);

        // lift_range_new(i64, i64) -> *mut LiftRange
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_range_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_range_new: {}", e))?;
        self.runtime_funcs.insert("lift_range_new".to_string(), func_id);

        // lift_range_start(*const LiftRange) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_range_start", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_range_start: {}", e))?;
        self.runtime_funcs.insert("lift_range_start".to_string(), func_id);

        // lift_range_end(*const LiftRange) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_range_end", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_range_end: {}", e))?;
        self.runtime_funcs.insert("lift_range_end".to_string(), func_id);

        // lift_output_range(*const LiftRange)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_output_range", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_range: {}", e))?;
        self.runtime_funcs.insert("lift_output_range".to_string(), func_id);

        // ==================== String Method Declarations ====================

        // lift_str_upper(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_upper", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_upper: {}", e))?;
        self.runtime_funcs.insert("lift_str_upper".to_string(), func_id);

        // lift_str_lower(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_lower", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_lower: {}", e))?;
        self.runtime_funcs.insert("lift_str_lower".to_string(), func_id);

        // lift_str_substring(*const c_char, i64, i64) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_substring", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_substring: {}", e))?;
        self.runtime_funcs.insert("lift_str_substring".to_string(), func_id);

        // lift_str_contains(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_str_contains", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_contains: {}", e))?;
        self.runtime_funcs.insert("lift_str_contains".to_string(), func_id);

        // lift_str_trim(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_trim", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_trim: {}", e))?;
        self.runtime_funcs.insert("lift_str_trim".to_string(), func_id);

        // lift_str_split(*const c_char, *const c_char) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_split", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_split: {}", e))?;
        self.runtime_funcs.insert("lift_str_split".to_string(), func_id);

        // lift_str_replace(*const c_char, *const c_char, *const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_str_replace", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_replace: {}", e))?;
        self.runtime_funcs.insert("lift_str_replace".to_string(), func_id);

        // lift_str_starts_with(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_str_starts_with", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_starts_with: {}", e))?;
        self.runtime_funcs.insert("lift_str_starts_with".to_string(), func_id);

        // lift_str_ends_with(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_str_ends_with", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_ends_with: {}", e))?;
        self.runtime_funcs.insert("lift_str_ends_with".to_string(), func_id);

        // lift_str_is_empty(*const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_str_is_empty", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_is_empty: {}", e))?;
        self.runtime_funcs.insert("lift_str_is_empty".to_string(), func_id);

        // ==================== List Method Declarations ====================

        // lift_list_first(*const LiftList) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_list_first", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_first: {}", e))?;
        self.runtime_funcs.insert("lift_list_first".to_string(), func_id);

        // lift_list_last(*const LiftList) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module
            .declare_function("lift_list_last", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_last: {}", e))?;
        self.runtime_funcs.insert("lift_list_last".to_string(), func_id);

        // lift_list_contains(*const LiftList, i64) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_list_contains", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_contains: {}", e))?;
        self.runtime_funcs.insert("lift_list_contains".to_string(), func_id);

        // lift_list_slice(*const LiftList, i64, i64) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_list_slice", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_slice: {}", e))?;
        self.runtime_funcs.insert("lift_list_slice".to_string(), func_id);

        // lift_list_reverse(*const LiftList) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_list_reverse", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_reverse: {}", e))?;
        self.runtime_funcs.insert("lift_list_reverse".to_string(), func_id);

        // lift_list_join(*const LiftList, *const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_list_join", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_join: {}", e))?;
        self.runtime_funcs.insert("lift_list_join".to_string(), func_id);

        // lift_list_is_empty(*const LiftList) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_list_is_empty", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_is_empty: {}", e))?;
        self.runtime_funcs.insert("lift_list_is_empty".to_string(), func_id);

        // ==================== Map Method Declarations ====================

        // lift_map_keys(*const LiftMap) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_map_keys", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_keys: {}", e))?;
        self.runtime_funcs.insert("lift_map_keys".to_string(), func_id);

        // lift_map_values(*const LiftMap) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self.module
            .declare_function("lift_map_values", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_values: {}", e))?;
        self.runtime_funcs.insert("lift_map_values".to_string(), func_id);

        // lift_map_contains_key(*const LiftMap, i64) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_map_contains_key", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_contains_key: {}", e))?;
        self.runtime_funcs.insert("lift_map_contains_key".to_string(), func_id);

        // lift_map_is_empty(*const LiftMap) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self.module
            .declare_function("lift_map_is_empty", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_is_empty: {}", e))?;
        self.runtime_funcs.insert("lift_map_is_empty".to_string(), func_id);

        Ok(())
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

            // Declare runtime functions in this function's scope
            let mut runtime_refs = HashMap::new();
            for (name, func_id) in &self.runtime_funcs {
                let func_ref = self.module.declare_func_in_func(*func_id, &mut builder.func);
                runtime_refs.insert(name.clone(), func_ref);
            }

            // Declare user functions in this function's scope
            let mut user_func_refs = HashMap::new();
            for (name, func_id) in &self.function_refs {
                let func_ref = self.module.declare_func_in_func(*func_id, &mut builder.func);
                user_func_refs.insert(name.clone(), func_ref);
            }

            // Compile the program expression with user function support
            let result = Self::compile_expr_static(
                &mut builder,
                expr,
                symbols,
                &runtime_refs,
                &user_func_refs,
                &mut self.variables
            )?;

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

    /// Helper: Convert Lift DataType to Cranelift Type
    fn data_type_to_cranelift_type(dt: &crate::syntax::DataType, pointer_type: Type) -> Type {
        use crate::syntax::DataType;

        match dt {
            DataType::Int | DataType::Bool => types::I64,
            DataType::Flt => types::F64,
            DataType::Str => pointer_type,
            DataType::List { .. } => pointer_type,
            DataType::Map { .. } => pointer_type,
            DataType::Range(_) => pointer_type,
            DataType::Unsolved => types::I64,  // Fallback
            DataType::TypeRef(_) => pointer_type,  // User-defined types treated as pointers for now
            DataType::Optional(_) => pointer_type,  // Optionals treated as pointers
            DataType::Set(_) => pointer_type,
            DataType::Enum(_) => types::I64,  // Enums as integers
            DataType::Struct(_) => pointer_type,  // Structs as pointers
        }
    }

    /// Collect all function definitions from an expression tree
    fn collect_function_definitions<'e>(&self, expr: &'e Expr, functions: &mut Vec<(&'e str, &'e Expr)>) {
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
    fn compile_user_function(
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

    /// Compile a Lift expression and return the Cranelift value
    /// Returns None for Unit expressions
    fn compile_expr_static(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        match expr {
            // Literals
            Expr::Literal(lit) => Self::compile_literal_with_runtime(builder, lit, runtime_funcs),
            Expr::RuntimeData(lit) => Self::compile_literal_with_runtime(builder, lit, runtime_funcs),

            // Binary operations
            Expr::BinaryExpr { left, op, right } => {
                Self::compile_binary_expr(builder, left, op, right, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Unary operations
            Expr::UnaryExpr { op, expr: inner } => {
                Self::compile_unary_expr(builder, op, inner, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Output
            Expr::Output { data } => {
                Self::compile_output(builder, data, symbols, runtime_funcs, user_func_refs, variables)?;
                Ok(None) // output returns Unit
            }

            // Program and Block
            Expr::Program { body, .. } => {
                Self::compile_block_body(builder, body, symbols, runtime_funcs, user_func_refs, variables)
            }
            Expr::Block { body, .. } => {
                Self::compile_block_body(builder, body, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Control flow
            Expr::If { cond, then, final_else } => {
                Self::compile_if_expr(builder, cond, then, final_else, symbols, runtime_funcs, user_func_refs, variables)
            }

            Expr::While { cond, body } => {
                Self::compile_while_expr(builder, cond, body, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Variables
            Expr::Let { var_name, value, data_type, .. } => {
                Self::compile_let(builder, var_name, value, data_type, symbols, runtime_funcs, user_func_refs, variables)
            }

            Expr::Variable { name, .. } => {
                Self::compile_variable(builder, name, variables)
            }

            Expr::Assign { name, value, .. } => {
                Self::compile_assign(builder, name, value, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Collections
            Expr::ListLiteral { data_type, data } => {
                Self::compile_list_literal(builder, data_type, data, symbols, runtime_funcs, user_func_refs, variables)
            }

            Expr::MapLiteral { key_type, value_type, data } => {
                Self::compile_map_literal(builder, key_type, value_type, data, symbols, runtime_funcs, user_func_refs, variables)
            }

            Expr::Index { expr, index } => {
                Self::compile_index(builder, expr, index, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Built-in functions
            Expr::Len { expr } => {
                Self::compile_len(builder, expr, symbols, runtime_funcs, user_func_refs, variables)
            }

            Expr::MethodCall { receiver, method_name, args, .. } => {
                Self::compile_method_call(builder, receiver, method_name, args, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Range
            Expr::Range(start, end) => {
                Self::compile_range(builder, start, end, runtime_funcs)
            }

            // Unit
            Expr::Unit => Ok(None),

            // Function calls
            Expr::Call { fn_name, args, index, .. } => {
                Self::compile_function_call(builder, fn_name, args, index, symbols, runtime_funcs, user_func_refs, variables)
            }

            // Function definitions (handled in preprocessing, so return Unit here)
            Expr::DefineFunction { .. } => Ok(None),

            // Type definitions (compile-time only, return Unit)
            Expr::DefineType { .. } => Ok(None),

            _ => Err(format!("Compilation not yet implemented for: {:?}", expr)),
        }
    }


    /// Compile a function call expression
    fn compile_function_call(
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
                let val = builder.ins().iconst(types::I64, if *b { 1 } else { 0 });
                Ok(Some(val))
            }
            LiteralData::Str(_s) => {
                // String literals need access to runtime functions
                // For now, we'll return an error and handle them in compile_literal_with_runtime
                Err("String literals require runtime function access - use compile_literal_with_runtime".to_string())
            }
        }
    }

    /// Compile a literal value with access to runtime functions (for strings)
    fn compile_literal_with_runtime(
        builder: &mut FunctionBuilder,
        lit: &LiteralData,
        runtime_funcs: &HashMap<String, FuncRef>,
    ) -> Result<Option<Value>, String> {
        match lit {
            LiteralData::Str(s) => {
                // Create a stack slot big enough for the string + null terminator
                let byte_len = s.len() + 1; // +1 for null terminator
                let slot = builder.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    byte_len as u32,
                    0,
                ));

                // Store each byte of the string in the stack slot
                for (i, byte) in s.bytes().enumerate() {
                    let byte_val = builder.ins().iconst(types::I8, byte as i64);
                    builder.ins().stack_store(byte_val, slot, i as i32);
                }
                // Store null terminator
                let null_byte = builder.ins().iconst(types::I8, 0);
                builder.ins().stack_store(null_byte, slot, s.len() as i32);

                // Get pointer to the stack slot
                let str_ptr = builder.ins().stack_addr(types::I64, slot, 0);

                // Call lift_str_new to create a heap-allocated string
                let func_ref = runtime_funcs.get("lift_str_new")
                    .ok_or_else(|| "Runtime function lift_str_new not found".to_string())?;
                let inst = builder.ins().call(*func_ref, &[str_ptr]);
                let result = builder.inst_results(inst)[0];

                Ok(Some(result))
            }
            // For non-string literals, use the simpler version
            _ => Self::compile_literal(builder, lit),
        }
    }

    /// Compile a binary expression
    fn compile_binary_expr(
        builder: &mut FunctionBuilder,
        left: &Expr,
        op: &Operator,
        right: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic_analysis::determine_type_with_symbols;
        use crate::syntax::DataType;

        // Check the type of the left operand to determine operation type
        // We must check the original left (before any reordering) to get the correct type
        let left_type_raw = determine_type_with_symbols(left, symbols, 0);
        let left_type = left_type_raw.map(|t| Self::resolve_type_alias(&t, symbols));

        // Also check right operand type for better type inference
        let right_type_raw = determine_type_with_symbols(right, symbols, 0);
        let right_type = right_type_raw.map(|t| Self::resolve_type_alias(&t, symbols));

        // Determine if this is a string or float operation
        let is_string_op = matches!(left_type, Some(DataType::Str)) || matches!(right_type, Some(DataType::Str));
        let is_float_op = matches!(left_type, Some(DataType::Flt)) || matches!(right_type, Some(DataType::Flt));

        if is_string_op {
            let left_val = Self::compile_expr_static(builder, left, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or("String operation requires non-Unit left operand")?;
            let right_val = Self::compile_expr_static(builder, right, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or("String operation requires non-Unit right operand")?;

            match op {
                Operator::Add => {
                    // String concatenation
                    let func_ref = runtime_funcs.get("lift_str_concat")
                        .ok_or_else(|| "Runtime function lift_str_concat not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result = builder.inst_results(inst)[0];
                    return Ok(Some(result));
                }
                Operator::Eq => {
                    // String equality
                    let func_ref = runtime_funcs.get("lift_str_eq")
                        .ok_or_else(|| "Runtime function lift_str_eq not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result_i8 = builder.inst_results(inst)[0];
                    // Extend I8 to I64
                    let result_i64 = builder.ins().uextend(types::I64, result_i8);
                    return Ok(Some(result_i64));
                }
                Operator::Neq => {
                    // String inequality (not equal)
                    let func_ref = runtime_funcs.get("lift_str_eq")
                        .ok_or_else(|| "Runtime function lift_str_eq not found".to_string())?;
                    let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
                    let result_i8 = builder.inst_results(inst)[0];
                    // Extend I8 to I64
                    let eq_i64 = builder.ins().uextend(types::I64, result_i8);
                    // Negate with XOR 1 (0 becomes 1, 1 becomes 0)
                    let one = builder.ins().iconst(types::I64, 1);
                    let neq = builder.ins().bxor(eq_i64, one);
                    return Ok(Some(neq));
                }
                _ => return Err(format!("Operator {:?} not supported for strings", op)),
            }
        }

        // Handle float operations
        if is_float_op {
            let left_val = Self::compile_expr_static(builder, left, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or("Float operation requires non-Unit left operand")?;
            let right_val = Self::compile_expr_static(builder, right, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or("Float operation requires non-Unit right operand")?;

            let result = match op {
                Operator::Add => builder.ins().fadd(left_val, right_val),
                Operator::Sub => builder.ins().fsub(left_val, right_val),
                Operator::Mul => builder.ins().fmul(left_val, right_val),
                Operator::Div => builder.ins().fdiv(left_val, right_val),
                Operator::Gt => {
                    let cmp = builder.ins().fcmp(FloatCC::GreaterThan, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Lt => {
                    let cmp = builder.ins().fcmp(FloatCC::LessThan, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Gte => {
                    let cmp = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Lte => {
                    let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Eq => {
                    let cmp = builder.ins().fcmp(FloatCC::Equal, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                Operator::Neq => {
                    let cmp = builder.ins().fcmp(FloatCC::NotEqual, left_val, right_val);
                    builder.ins().uextend(types::I64, cmp)
                }
                _ => return Err(format!("Operator {:?} not yet implemented for floats", op)),
            };
            return Ok(Some(result));
        }

        // Handle Range operator
        if matches!(op, Operator::Range) {
            let left_val = Self::compile_expr_static(builder, left, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or("Range operation requires non-Unit left operand")?;
            let right_val = Self::compile_expr_static(builder, right, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or("Range operation requires non-Unit right operand")?;

            // Call lift_range_new(start, end)
            let func_ref = runtime_funcs.get("lift_range_new")
                .ok_or_else(|| "Runtime function lift_range_new not found".to_string())?;
            let inst = builder.ins().call(*func_ref, &[left_val, right_val]);
            let range_ptr = builder.inst_results(inst)[0];
            return Ok(Some(range_ptr));
        }

        // For integer operations, compile operands and perform integer operations
        let left_val = Self::compile_expr_static(builder, left, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("Binary operation requires non-Unit left operand")?;
        let right_val = Self::compile_expr_static(builder, right, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("Binary operation requires non-Unit right operand")?;

        let result = match op {
            Operator::Add => builder.ins().iadd(left_val, right_val),
            Operator::Sub => builder.ins().isub(left_val, right_val),
            Operator::Mul => builder.ins().imul(left_val, right_val),
            Operator::Div => builder.ins().sdiv(left_val, right_val),
            Operator::Gt => {
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Lt => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThan, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Gte => {
                let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Lte => {
                let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Eq => {
                let cmp = builder.ins().icmp(IntCC::Equal, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::Neq => {
                let cmp = builder.ins().icmp(IntCC::NotEqual, left_val, right_val);
                builder.ins().uextend(types::I64, cmp)
            }
            Operator::And => {
                // Logical AND: both operands must be non-zero
                // Convert each operand to boolean (0 or 1), then AND them
                let zero = builder.ins().iconst(types::I64, 0);
                let left_bool = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                let right_bool = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                let result_bool = builder.ins().band(left_bool, right_bool);
                builder.ins().uextend(types::I64, result_bool)
            }
            Operator::Or => {
                // Logical OR: at least one operand must be non-zero
                // Convert each operand to boolean (0 or 1), then OR them
                let zero = builder.ins().iconst(types::I64, 0);
                let left_bool = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                let right_bool = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                let result_bool = builder.ins().bor(left_bool, right_bool);
                builder.ins().uextend(types::I64, result_bool)
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
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        let val = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("Unary operation requires non-Unit operand")?;

        let result = match op {
            Operator::Sub => {
                // Negate: 0 - val
                let zero = builder.ins().iconst(types::I64, 0);
                builder.ins().isub(zero, val)
            }
            Operator::Not => {
                // Boolean not: val == 0
                let zero = builder.ins().iconst(types::I64, 0);
                let cmp = builder.ins().icmp(IntCC::Equal, val, zero);
                builder.ins().uextend(types::I64, cmp)
            }
            _ => return Err(format!("Unary operator {:?} not yet implemented", op)),
        };

        Ok(Some(result))
    }

    /// Helper: Resolve TypeRef to underlying type
    fn resolve_type_alias(data_type: &crate::syntax::DataType, symbols: &SymbolTable) -> crate::syntax::DataType {
        use crate::syntax::DataType;

        let mut resolved = data_type.clone();
        let mut visited = std::collections::HashSet::new();

        while let DataType::TypeRef(name) = &resolved {
            // Prevent infinite loops
            if !visited.insert(name.clone()) {
                break;
            }

            // Look up the type in all scopes (start from the deepest scope)
            let mut found = None;
            for scope_idx in (0..symbols.scope_count()).rev() {
                if let Some(underlying_type) = symbols.lookup_type(name, scope_idx) {
                    found = Some(underlying_type);
                    break;
                }
            }

            if let Some(underlying_type) = found {
                resolved = underlying_type;
            } else {
                // Type not found, leave as TypeRef
                break;
            }
        }

        resolved
    }

    /// Compile an output statement
    fn compile_output(
        builder: &mut FunctionBuilder,
        data: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<(), String> {
        use crate::semantic_analysis::determine_type_with_symbols;
        use crate::syntax::DataType;

        for expr in data {
            // Determine the type of the expression
            let expr_type_raw = determine_type_with_symbols(expr, symbols, 0)
                .ok_or_else(|| format!("Cannot determine type for output expression"))?;

            // Resolve TypeRef to underlying type
            let expr_type = Self::resolve_type_alias(&expr_type_raw, symbols);

            // Compile the expression to get the value
            let val = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, user_func_refs, variables)?
                .ok_or_else(|| "Output requires non-Unit expression".to_string())?;

            // Determine which output function to call based on type
            let (func_name, needs_conversion) = match expr_type {
                DataType::Int => ("lift_output_int", false),
                DataType::Flt => ("lift_output_float", false),
                DataType::Bool => ("lift_output_bool", true), // Need to convert I64 to I8
                DataType::Str => ("lift_output_str", false),
                DataType::Range(_) => ("lift_output_range", false),
                DataType::List { .. } => ("lift_output_list", false),
                DataType::Map { .. } => ("lift_output_map", false),
                _ => return Err(format!("Output not yet supported for type: {:?}", expr_type)),
            };

            // Get the function reference
            let func_ref = runtime_funcs.get(func_name)
                .ok_or_else(|| format!("Runtime function not found: {}", func_name))?;

            // Convert value if needed
            let call_val = if needs_conversion {
                builder.ins().ireduce(types::I8, val)
            } else {
                val
            };

            // Call the function
            builder.ins().call(*func_ref, &[call_val]);
        }

        // Print newline after all output items (to match interpreter behavior)
        let newline_func = runtime_funcs.get("lift_output_newline")
            .ok_or_else(|| "Runtime function not found: lift_output_newline".to_string())?;
        builder.ins().call(*newline_func, &[]);

        Ok(())
    }

    /// Compile a block body (sequence of expressions)
    fn compile_block_body(
        builder: &mut FunctionBuilder,
        body: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        let mut last_value = None;

        for expr in body {
            last_value = Self::compile_expr_static(builder, expr, symbols, runtime_funcs, user_func_refs, variables)?;
        }

        Ok(last_value)
    }

    /// Compile an if/else expression
    fn compile_if_expr(
        builder: &mut FunctionBuilder,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Evaluate the condition
        let cond_val = Self::compile_expr_static(builder, cond, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("If condition must produce a value")?;

        // Create blocks for the then branch, else branch, and merge point
        let then_block = builder.create_block();
        let else_block = builder.create_block();
        let merge_block = builder.create_block();

        // Check if this if expression produces a value
        let produces_value = !matches!(then_expr, Expr::Unit) || !matches!(else_expr, Expr::Unit);

        // Create a stack slot to store the result if needed
        let result_slot = if produces_value {
            Some(builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                8, // 8 bytes for I64
                0,
            )))
        } else {
            None
        };

        // Branch based on condition
        // In Cranelift, brif branches if value is non-zero
        builder.ins().brif(cond_val, then_block, &[], else_block, &[]);

        // Compile the then branch
        builder.switch_to_block(then_block);
        builder.seal_block(then_block);
        let then_val = Self::compile_expr_static(builder, then_expr, symbols, runtime_funcs, user_func_refs, variables)?;

        if produces_value {
            let then_result = then_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().stack_store(then_result, result_slot.unwrap(), 0);
        }
        builder.ins().jump(merge_block, &[]);

        // Compile the else branch
        builder.switch_to_block(else_block);
        builder.seal_block(else_block);
        let else_val = Self::compile_expr_static(builder, else_expr, symbols, runtime_funcs, user_func_refs, variables)?;

        if produces_value {
            let else_result = else_val.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
            builder.ins().stack_store(else_result, result_slot.unwrap(), 0);
        }
        builder.ins().jump(merge_block, &[]);

        // Switch to merge block and load the result
        builder.switch_to_block(merge_block);
        builder.seal_block(merge_block);

        if produces_value {
            let result = builder.ins().stack_load(types::I64, result_slot.unwrap(), 0);
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Compile a while loop
    fn compile_while_expr(
        builder: &mut FunctionBuilder,
        cond: &Expr,
        body: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Create blocks for loop header, body, and exit
        let loop_header = builder.create_block();
        let loop_body = builder.create_block();
        let loop_exit = builder.create_block();

        // Jump to the loop header
        builder.ins().jump(loop_header, &[]);

        // Loop header: evaluate condition and branch
        builder.switch_to_block(loop_header);
        let cond_val = Self::compile_expr_static(builder, cond, symbols, runtime_funcs, user_func_refs, variables)?
            .ok_or("While condition must produce a value")?;

        builder.ins().brif(cond_val, loop_body, &[], loop_exit, &[]);
        builder.seal_block(loop_header);

        // Loop body: execute body and jump back to header
        builder.switch_to_block(loop_body);
        Self::compile_expr_static(builder, body, symbols, runtime_funcs, user_func_refs, variables)?;
        builder.ins().jump(loop_header, &[]);
        builder.seal_block(loop_body);

        // Exit block
        builder.switch_to_block(loop_exit);
        builder.seal_block(loop_exit);

        // While loops return Unit
        Ok(None)
    }

    /// Compile a let binding (variable declaration)
    fn compile_let(
        builder: &mut FunctionBuilder,
        var_name: &str,
        value: &Expr,
        data_type: &crate::syntax::DataType,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic_analysis::determine_type_with_symbols;
        use crate::syntax::DataType;

        // Compile the value expression
        let val = Self::compile_expr_static(builder, value, symbols, runtime_funcs, user_func_refs, variables)?
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
        let slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            8,
            0,
        ));

        // Store the value in the stack slot
        builder.ins().stack_store(val, slot, 0);

        // Remember this variable's stack slot and type
        variables.insert(var_name.to_string(), VarInfo {
            slot,
            cranelift_type,
        });

        // Let expressions return Unit
        Ok(None)
    }

    /// Compile a variable reference (reading a variable)
    fn compile_variable(
        builder: &mut FunctionBuilder,
        name: &str,
        variables: &HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Look up the variable's stack slot and type
        let var_info = variables
            .get(name)
            .ok_or_else(|| format!("Undefined variable: {}", name))?;

        // Load the value from the stack slot with the correct type
        let val = builder.ins().stack_load(var_info.cranelift_type, var_info.slot, 0);
        Ok(Some(val))
    }

    /// Compile an assignment expression (mutating a variable)
    fn compile_assign(
        builder: &mut FunctionBuilder,
        name: &str,
        value: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        // Compile the new value
        let val = Self::compile_expr_static(builder, value, symbols, runtime_funcs, user_func_refs, variables)?
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

    /// Compile a list literal (supports all types)
    fn compile_list_literal(
        builder: &mut FunctionBuilder,
        data_type: &crate::syntax::DataType,
        data: &[Expr],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::syntax::DataType;
        use crate::semantic_analysis::determine_type_with_symbols;

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

    /// Compile a map literal (supports scalar keys and all value types)
    fn compile_map_literal(
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
        use crate::semantic_analysis::determine_type_with_symbols;

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

    /// Compile an index expression (list[i] or map[key])
    fn compile_index(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        index: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic_analysis::determine_type_with_symbols;
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

    /// Compile len() built-in function
    fn compile_len(
        builder: &mut FunctionBuilder,
        expr: &Expr,
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic_analysis::determine_type_with_symbols;
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

    /// Compile a range expression (start..end)
    fn compile_range(
        builder: &mut FunctionBuilder,
        start: &LiteralData,
        end: &LiteralData,
        runtime_funcs: &HashMap<String, FuncRef>,
    ) -> Result<Option<Value>, String> {
        // Extract integer values from start and end
        let start_val = match start {
            LiteralData::Int(i) => builder.ins().iconst(types::I64, *i),
            _ => return Err("Range start must be an integer".to_string()),
        };

        let end_val = match end {
            LiteralData::Int(i) => builder.ins().iconst(types::I64, *i),
            _ => return Err("Range end must be an integer".to_string()),
        };

        // Call lift_range_new(start, end) to create the range
        let func_ref = runtime_funcs.get("lift_range_new")
            .ok_or_else(|| "Runtime function lift_range_new not found".to_string())?;
        let inst = builder.ins().call(*func_ref, &[start_val, end_val]);
        let range_ptr = builder.inst_results(inst)[0];

        Ok(Some(range_ptr))
    }

    /// Compile method calls
    fn compile_method_call(
        builder: &mut FunctionBuilder,
        receiver: &Expr,
        method_name: &str,
        args: &[crate::syntax::KeywordArg],
        symbols: &SymbolTable,
        runtime_funcs: &HashMap<String, FuncRef>,
        user_func_refs: &HashMap<String, FuncRef>,
        variables: &mut HashMap<String, VarInfo>,
    ) -> Result<Option<Value>, String> {
        use crate::semantic_analysis::determine_type_with_symbols;
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

#[cfg(test)]
mod tests {
    use super::*;

    // Tests will be added as we implement more features
}
