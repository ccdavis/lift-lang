// Runtime function declarations for Cranelift code generation

use super::CodeGenerator;
use cranelift::prelude::*;
use cranelift_module::Module;

impl<'a, M: Module> CodeGenerator<'a, M> {
    /// Declare all runtime functions that will be linked from the runtime library
    pub fn declare_runtime_functions(&mut self) -> Result<(), String> {
        let pointer_type = self.module.target_config().pointer_type();

        // lift_output_int(i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_output_int", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_int: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_int".to_string(), func_id);

        // lift_output_float(f64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::F64));
        let func_id = self
            .module
            .declare_function("lift_output_float", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_float: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_float".to_string(), func_id);

        // lift_output_bool(i8)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function("lift_output_bool", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_bool: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_bool".to_string(), func_id);

        // lift_output_str(*const c_char)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_output_str", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_str: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_str".to_string(), func_id);

        // lift_output_newline()
        let sig = self.module.make_signature();
        let func_id = self
            .module
            .declare_function(
                "lift_output_newline",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_output_newline: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_newline".to_string(), func_id);

        // lift_output_list(*const LiftList)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_output_list", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_list: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_list".to_string(), func_id);

        // lift_output_map(*const LiftMap)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_output_map", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_map: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_map".to_string(), func_id);

        // lift_str_new(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_str_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_new: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_new".to_string(), func_id);

        // lift_str_concat(*const c_char, *const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_str_concat", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_concat: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_concat".to_string(), func_id);

        // lift_str_eq(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function("lift_str_eq", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_eq: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_eq".to_string(), func_id);

        // ====================================================================
        // LiftString Functions (new string type with SSO)
        // ====================================================================

        // lift_string_init_from_cstr(*mut LiftString, *const c_char)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_string_init_from_cstr",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_init_from_cstr: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_init_from_cstr".to_string(), func_id);

        // lift_string_concat_to(*mut LiftString, *const LiftString, *const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_string_concat_to",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_concat_to: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_concat_to".to_string(), func_id);

        // lift_string_copy(*mut LiftString, *const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_string_copy", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_copy: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_copy".to_string(), func_id);

        // lift_string_drop(*mut LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_string_drop", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_drop: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_drop".to_string(), func_id);

        // lift_output_lift_string_ptr(*const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_output_lift_string_ptr",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_output_lift_string_ptr: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_lift_string_ptr".to_string(), func_id);

        // LiftString method functions
        // lift_string_len(*const LiftString) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_string_len", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_len: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_len".to_string(), func_id);

        // lift_string_eq(*const LiftString, *const LiftString) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function("lift_string_eq", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_eq: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_eq".to_string(), func_id);

        // lift_string_upper(*mut LiftString, *const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_string_upper", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_upper: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_upper".to_string(), func_id);

        // lift_string_lower(*mut LiftString, *const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_string_lower", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_lower: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_lower".to_string(), func_id);

        // lift_string_substring(*mut LiftString, *const LiftString, i64, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function(
                "lift_string_substring",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_substring: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_substring".to_string(), func_id);

        // lift_string_contains(*const LiftString, *const LiftString) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_string_contains",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_contains: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_contains".to_string(), func_id);

        // lift_string_trim(*mut LiftString, *const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_string_trim", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_trim: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_trim".to_string(), func_id);

        // lift_string_replace(*mut LiftString, *const LiftString, *const LiftString, *const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_string_replace",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_replace: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_replace".to_string(), func_id);

        // lift_string_starts_with(*const LiftString, *const LiftString) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_string_starts_with",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_starts_with: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_starts_with".to_string(), func_id);

        // lift_string_ends_with(*const LiftString, *const LiftString) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_string_ends_with",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_ends_with: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_ends_with".to_string(), func_id);

        // lift_string_is_empty(*const LiftString) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_string_is_empty",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_string_is_empty: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_is_empty".to_string(), func_id);

        // lift_string_split(*const LiftString, *const LiftString) -> *mut RcList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_string_split", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_string_split: {}", e))?;
        self.runtime_funcs
            .insert("lift_string_split".to_string(), func_id);

        // lift_list_new(i64, i8) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I8));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_list_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_new: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_new".to_string(), func_id);

        // lift_list_set(*mut LiftList, i64, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_list_set", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_set: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_set".to_string(), func_id);

        // lift_list_get(*const LiftList, i64) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_list_get", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_get: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_get".to_string(), func_id);

        // lift_map_new(i64, i8, i8) -> *mut LiftMap
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I8));
        sig.params.push(AbiParam::new(types::I8));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_map_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_new: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_new".to_string(), func_id);

        // lift_map_set(*mut LiftMap, i64, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_map_set", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_set: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_set".to_string(), func_id);

        // lift_map_get(*const LiftMap, i64) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_map_get", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_get: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_get".to_string(), func_id);

        // lift_str_len(*const c_char) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_str_len", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_len: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_len".to_string(), func_id);

        // lift_list_len(*const LiftList) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_list_len", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_len: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_len".to_string(), func_id);

        // lift_list_push(*mut LiftList, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_list_push", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_push: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_push".to_string(), func_id);

        // lift_list_reserve(*mut LiftList, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_list_reserve", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_reserve: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_reserve".to_string(), func_id);

        // lift_list_concat(*const LiftList, *const LiftList) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_list_concat", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_concat: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_concat".to_string(), func_id);

        // lift_map_len(*const LiftMap) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_map_len", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_len: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_len".to_string(), func_id);

        // lift_range_new(i64, i64) -> *mut LiftRange
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_range_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_range_new: {}", e))?;
        self.runtime_funcs
            .insert("lift_range_new".to_string(), func_id);

        // lift_range_start(*const LiftRange) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_range_start", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_range_start: {}", e))?;
        self.runtime_funcs
            .insert("lift_range_start".to_string(), func_id);

        // lift_range_end(*const LiftRange) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_range_end", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_range_end: {}", e))?;
        self.runtime_funcs
            .insert("lift_range_end".to_string(), func_id);

        // lift_output_range(*const LiftRange)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_output_range", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_output_range: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_range".to_string(), func_id);

        // ==================== String Method Declarations ====================

        // lift_str_upper(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_str_upper", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_upper: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_upper".to_string(), func_id);

        // lift_str_lower(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_str_lower", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_lower: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_lower".to_string(), func_id);

        // lift_str_substring(*const c_char, i64, i64) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_str_substring",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_str_substring: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_substring".to_string(), func_id);

        // lift_str_contains(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function("lift_str_contains", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_contains: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_contains".to_string(), func_id);

        // lift_str_trim(*const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_str_trim", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_trim: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_trim".to_string(), func_id);

        // lift_str_split(*const c_char, *const c_char) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_str_split", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_split: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_split".to_string(), func_id);

        // lift_str_replace(*const c_char, *const c_char, *const c_char) -> *mut c_char
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_str_replace", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_replace: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_replace".to_string(), func_id);

        // lift_str_starts_with(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_str_starts_with",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_str_starts_with: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_starts_with".to_string(), func_id);

        // lift_str_ends_with(*const c_char, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_str_ends_with",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_str_ends_with: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_ends_with".to_string(), func_id);

        // lift_str_is_empty(*const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function("lift_str_is_empty", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_str_is_empty: {}", e))?;
        self.runtime_funcs
            .insert("lift_str_is_empty".to_string(), func_id);

        // ==================== List Method Declarations ====================

        // lift_list_first(*const LiftList) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_list_first", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_first: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_first".to_string(), func_id);

        // lift_list_last(*const LiftList) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function("lift_list_last", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_last: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_last".to_string(), func_id);

        // lift_list_contains(*const LiftList, i64) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_list_contains",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_list_contains: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_contains".to_string(), func_id);

        // lift_list_slice(*const LiftList, i64, i64) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_list_slice", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_slice: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_slice".to_string(), func_id);

        // lift_list_reverse(*const LiftList) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_list_reverse", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_reverse: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_reverse".to_string(), func_id);

        // lift_list_join(dest: *mut LiftString, list: *const RcList, separator: *const LiftString)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type)); // dest
        sig.params.push(AbiParam::new(pointer_type)); // list
        sig.params.push(AbiParam::new(pointer_type)); // separator
        let func_id = self
            .module
            .declare_function("lift_list_join", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_join: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_join".to_string(), func_id);

        // lift_list_is_empty(*const LiftList) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_list_is_empty",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_list_is_empty: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_is_empty".to_string(), func_id);

        // ==================== Map Method Declarations ====================

        // lift_map_keys(*const LiftMap) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_map_keys", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_keys: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_keys".to_string(), func_id);

        // lift_map_values(*const LiftMap) -> *mut LiftList
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_map_values", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_values: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_values".to_string(), func_id);

        // lift_map_contains_key(*const LiftMap, i64) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_map_contains_key",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_map_contains_key: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_contains_key".to_string(), func_id);

        // lift_map_is_empty(*const LiftMap) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function("lift_map_is_empty", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_is_empty: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_is_empty".to_string(), func_id);

        // ==================== Struct Function Declarations ====================

        // lift_struct_new(*const c_char, i64) -> *mut LiftStruct
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type)); // type_name
        sig.params.push(AbiParam::new(types::I64)); // field_count
        sig.returns.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_struct_new", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_struct_new: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_new".to_string(), func_id);

        // lift_struct_set_field(*mut LiftStruct, *const c_char, i8, i64)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type)); // struct_ptr
        sig.params.push(AbiParam::new(pointer_type)); // field_name
        sig.params.push(AbiParam::new(types::I8)); // type_tag
        sig.params.push(AbiParam::new(types::I64)); // value
        let func_id = self
            .module
            .declare_function(
                "lift_struct_set_field",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_struct_set_field: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_set_field".to_string(), func_id);

        // lift_struct_get_field(*const LiftStruct, *const c_char) -> i64
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type)); // struct_ptr
        sig.params.push(AbiParam::new(pointer_type)); // field_name
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self
            .module
            .declare_function(
                "lift_struct_get_field",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_struct_get_field: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_get_field".to_string(), func_id);

        // lift_struct_get_field_type(*const LiftStruct, *const c_char) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type)); // struct_ptr
        sig.params.push(AbiParam::new(pointer_type)); // field_name
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function(
                "lift_struct_get_field_type",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_struct_get_field_type: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_get_field_type".to_string(), func_id);

        // lift_struct_eq(*const LiftStruct, *const LiftStruct) -> i8
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type)); // struct1_ptr
        sig.params.push(AbiParam::new(pointer_type)); // struct2_ptr
        sig.returns.push(AbiParam::new(types::I8));
        let func_id = self
            .module
            .declare_function("lift_struct_eq", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_struct_eq: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_eq".to_string(), func_id);

        // lift_output_struct(*const LiftStruct)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_output_struct",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_output_struct: {}", e))?;
        self.runtime_funcs
            .insert("lift_output_struct".to_string(), func_id);

        // lift_struct_free(*mut LiftStruct)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_struct_free", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_struct_free: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_free".to_string(), func_id);

        // ==================== Reference Counting Declarations ====================

        // lift_list_retain(*mut RcList)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_list_retain", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_retain: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_retain".to_string(), func_id);

        // lift_list_release(*mut RcList)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_list_release", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_list_release: {}", e))?;
        self.runtime_funcs
            .insert("lift_list_release".to_string(), func_id);

        // lift_map_retain(*mut RcMap)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_map_retain", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_retain: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_retain".to_string(), func_id);

        // lift_map_release(*mut RcMap)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_map_release", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_map_release: {}", e))?;
        self.runtime_funcs
            .insert("lift_map_release".to_string(), func_id);

        // lift_struct_retain(*mut RcStruct)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_struct_retain",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_struct_retain: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_retain".to_string(), func_id);

        // lift_struct_release(*mut RcStruct)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_struct_release",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_struct_release: {}", e))?;
        self.runtime_funcs
            .insert("lift_struct_release".to_string(), func_id);

        // lift_range_retain(*mut RcRange)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function("lift_range_retain", cranelift_module::Linkage::Import, &sig)
            .map_err(|e| format!("Failed to declare lift_range_retain: {}", e))?;
        self.runtime_funcs
            .insert("lift_range_retain".to_string(), func_id);

        // lift_range_release(*mut RcRange)
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(pointer_type));
        let func_id = self
            .module
            .declare_function(
                "lift_range_release",
                cranelift_module::Linkage::Import,
                &sig,
            )
            .map_err(|e| format!("Failed to declare lift_range_release: {}", e))?;
        self.runtime_funcs
            .insert("lift_range_release".to_string(), func_id);

        Ok(())
    }
}
