// Struct runtime functions for Lift

use std::collections::HashMap;
use std::os::raw::c_char;

use super::refcount::RefCounted;
use super::string::lift_str_eq;
use super::list::RcList;
use super::map::RcMap;
use super::range::RcRange;
use super::output::{TYPE_INT, TYPE_FLT, TYPE_BOOL, TYPE_STR, TYPE_LIST, TYPE_MAP, TYPE_RANGE, TYPE_STRUCT, format_value_inline};

/// Field value with type information
#[derive(Debug, Clone)]
pub struct StructFieldValue {
    pub type_tag: i8,
    pub value: i64,
}

/// Runtime representation of a struct
#[repr(C)]
pub struct LiftStruct {
    pub type_name: String,
    pub fields: HashMap<String, StructFieldValue>,
}

/// Type alias for reference-counted struct
pub type RcStruct = RefCounted<LiftStruct>;


/// Create a new struct with given type name and field capacity
#[no_mangle]
pub unsafe extern "C" fn lift_struct_new(
    type_name: *const c_char,
    field_count: i64,
) -> *mut RcStruct {
    if type_name.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let c_str = std::ffi::CStr::from_ptr(type_name);
        if let Ok(name_str) = c_str.to_str() {
            let cap = field_count.max(0) as usize;
            let lift_struct = LiftStruct {
                type_name: name_str.to_string(),
                fields: HashMap::with_capacity(cap),
            };
            return RefCounted::new(lift_struct);
        }
    }
    std::ptr::null_mut()
}

/// Increment reference count for a struct
#[no_mangle]
pub unsafe extern "C" fn lift_struct_retain(s: *mut RcStruct) {
    RefCounted::retain(s);
}

/// Decrement reference count for a struct and free if it reaches zero
#[no_mangle]
pub unsafe extern "C" fn lift_struct_release(s: *mut RcStruct) {
    RefCounted::release(s);
}

/// Set a field value in a struct
#[no_mangle]
pub unsafe extern "C" fn lift_struct_set_field(
    s: *mut RcStruct,
    field_name: *const c_char,
    type_tag: i8,
    value: i64,
) {
    if s.is_null() || field_name.is_null() {
        return;
    }

    unsafe {
        if let Some(struct_ref) = RefCounted::get_mut(s) {
            let c_str = std::ffi::CStr::from_ptr(field_name);
            if let Ok(name_str) = c_str.to_str() {
                struct_ref
                    .fields
                    .insert(name_str.to_string(), StructFieldValue { type_tag, value });
            }
        }
    }
}

/// Get a field value from a struct
#[no_mangle]
pub unsafe extern "C" fn lift_struct_get_field(
    s: *const RcStruct,
    field_name: *const c_char,
) -> i64 {
    if s.is_null() || field_name.is_null() {
        return 0;
    }

    unsafe {
        if let Some(struct_ref) = RefCounted::get(s) {
            let c_str = std::ffi::CStr::from_ptr(field_name);
            if let Ok(name_str) = c_str.to_str() {
                if let Some(field_value) = struct_ref.fields.get(name_str) {
                    return field_value.value;
                }
            }
        }
    }
    0
}

/// Get the type tag of a field in a struct
#[no_mangle]
pub unsafe extern "C" fn lift_struct_get_field_type(
    s: *const RcStruct,
    field_name: *const c_char,
) -> i8 {
    if s.is_null() || field_name.is_null() {
        return -1;
    }

    unsafe {
        if let Some(struct_ref) = RefCounted::get(s) {
            let c_str = std::ffi::CStr::from_ptr(field_name);
            if let Ok(name_str) = c_str.to_str() {
                if let Some(field_value) = struct_ref.fields.get(name_str) {
                    return field_value.type_tag;
                }
            }
        }
    }
    -1
}

/// Free a struct (deprecated - use lift_struct_release instead)
#[no_mangle]
pub unsafe extern "C" fn lift_struct_free(s: *mut RcStruct) {
    lift_struct_release(s);
}

/// Format a struct inline (without trailing space) for nested collections
pub unsafe fn format_struct_inline(ptr: *const RcStruct) {
    if ptr.is_null() {
        print!("{{}}");
        return;
    }
    if let Some(s) = RefCounted::get(ptr) {
        print!("{} {{ ", s.type_name);

        // Sort fields by name for consistent output
        let mut field_names: Vec<&String> = s.fields.keys().collect();
        field_names.sort();

        for (i, field_name) in field_names.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            let field_value = &s.fields[*field_name];
            print!("{}: ", field_name);
            format_value_inline(field_value.value, field_value.type_tag);
        }
        print!(" }}");
    } else {
        print!("{{}}");
    }
}

/// Output a struct (pretty-print with trailing space)
#[no_mangle]
pub unsafe extern "C" fn lift_output_struct(s: *const RcStruct) {
    if s.is_null() {
        print!("{{}} ");
        return;
    }
    unsafe {
        format_struct_inline(s);
        print!(" ");
    }
}

/// Helper function to recursively compare two values for equality
unsafe fn compare_values_for_equality(val1: i64, type_tag1: i8, val2: i64, type_tag2: i8) -> bool {
    // Different types are not equal
    if type_tag1 != type_tag2 {
        return false;
    }

    match type_tag1 {
        TYPE_INT => val1 == val2,
        TYPE_FLT => {
            let f1 = f64::from_bits(val1 as u64);
            let f2 = f64::from_bits(val2 as u64);
            f1 == f2
        }
        TYPE_BOOL => val1 == val2,
        TYPE_STR => {
            let ptr1 = val1 as *const c_char;
            let ptr2 = val2 as *const c_char;
            lift_str_eq(ptr1, ptr2) != 0
        }
        TYPE_STRUCT => {
            let s1 = val1 as *const RcStruct;
            let s2 = val2 as *const RcStruct;
            lift_struct_eq(s1, s2) != 0
        }
        TYPE_LIST => {
            let list1 = val1 as *const RcList;
            let list2 = val2 as *const RcList;
            if list1.is_null() && list2.is_null() {
                return true;
            }
            if list1.is_null() || list2.is_null() {
                return false;
            }
            if let (Some(l1), Some(l2)) = (RefCounted::get(list1), RefCounted::get(list2)) {
                if l1.elem_type != l2.elem_type || l1.elements.len() != l2.elements.len() {
                    return false;
                }
                for i in 0..l1.elements.len() {
                    if !compare_values_for_equality(
                        l1.elements[i],
                        l1.elem_type,
                        l2.elements[i],
                        l2.elem_type,
                    ) {
                        return false;
                    }
                }
                true
            } else {
                false
            }
        }
        TYPE_MAP => {
            let map1 = val1 as *const RcMap;
            let map2 = val2 as *const RcMap;
            if map1.is_null() && map2.is_null() {
                return true;
            }
            if map1.is_null() || map2.is_null() {
                return false;
            }
            if let (Some(m1), Some(m2)) = (RefCounted::get(map1), RefCounted::get(map2)) {
                if m1.key_type != m2.key_type
                    || m1.value_type != m2.value_type
                    || m1.entries.len() != m2.entries.len()
                {
                    return false;
                }
                for (key, val1) in &m1.entries {
                    match m2.entries.get(key) {
                        Some(val2) => {
                            if !compare_values_for_equality(
                                *val1,
                                m1.value_type,
                                *val2,
                                m2.value_type,
                            ) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            } else {
                false
            }
        }
        TYPE_RANGE => {
            let r1 = val1 as *const RcRange;
            let r2 = val2 as *const RcRange;
            if r1.is_null() && r2.is_null() {
                return true;
            }
            if r1.is_null() || r2.is_null() {
                return false;
            }
            if let (Some(range1), Some(range2)) = (RefCounted::get(r1), RefCounted::get(r2)) {
                range1.start == range2.start && range1.end == range2.end
            } else {
                false
            }
        }
        _ => val1 == val2, // Fallback for unknown types
    }
}

/// Compare two structs for structural equality
#[no_mangle]
pub unsafe extern "C" fn lift_struct_eq(s1: *const RcStruct, s2: *const RcStruct) -> i8 {
    if s1.is_null() && s2.is_null() {
        return 1;
    }
    if s1.is_null() || s2.is_null() {
        return 0;
    }

    unsafe {
        if let (Some(struct1), Some(struct2)) = (RefCounted::get(s1), RefCounted::get(s2)) {
            // Check type names match
            if struct1.type_name != struct2.type_name {
                return 0;
            }

            // Check same number of fields
            if struct1.fields.len() != struct2.fields.len() {
                return 0;
            }

            // Check all fields match
            for (field_name, field_value1) in &struct1.fields {
                match struct2.fields.get(field_name) {
                    Some(field_value2) => {
                        if !compare_values_for_equality(
                            field_value1.value,
                            field_value1.type_tag,
                            field_value2.value,
                            field_value2.type_tag,
                        ) {
                            return 0;
                        }
                    }
                    None => return 0, // Field not found in second struct
                }
            }

            1 // All fields match
        } else {
            0
        }
    }
}
