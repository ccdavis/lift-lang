// Runtime library for Lift compiler
// These functions are called from JIT-compiled code to handle heap-allocated types

pub mod refcount;
pub mod string;
pub mod list;
pub mod map;
pub mod range;
pub mod structs;
pub mod output;

// Re-export commonly used items at the crate level for backward compatibility
pub use refcount::{RefCounted, refcount_stats};
pub use string::LiftString;
pub use list::{LiftList, RcList};
pub use map::{LiftMap, RcMap};
pub use range::{LiftRange, RcRange};
pub use structs::{LiftStruct, RcStruct};
pub use output::{
    TYPE_INT, TYPE_FLT, TYPE_BOOL, TYPE_STR, TYPE_LIST, TYPE_MAP, TYPE_RANGE, TYPE_STRUCT,
};

// Re-export all the C-callable functions
pub use string::{
    lift_string_new, lift_string_clone, lift_string_retain, lift_string_release,
    lift_string_concat, lift_string_to_cstr, lift_string_free_cstr,
    lift_output_lift_string, lift_string_init_from_cstr, lift_string_concat_to,
    lift_string_copy, lift_string_drop, lift_output_lift_string_ptr,
    lift_string_len, lift_string_eq, lift_string_upper, lift_string_lower,
    lift_string_substring, lift_string_contains, lift_string_trim,
    lift_string_replace, lift_string_starts_with, lift_string_ends_with,
    lift_string_is_empty, lift_string_split,
    // Legacy string functions
    lift_str_new, lift_str_concat, lift_str_len, lift_str_free, lift_str_eq,
    // String methods
    lift_str_upper, lift_str_lower, lift_str_substring, lift_str_contains,
    lift_str_trim, lift_str_replace, lift_str_starts_with, lift_str_ends_with,
    lift_str_is_empty, lift_str_split,
};

pub use list::{
    lift_list_new, lift_list_retain, lift_list_release, lift_list_set,
    lift_list_get, lift_list_len, lift_list_push, lift_list_reserve,
    lift_list_concat, lift_list_free,
    // List methods
    lift_list_first, lift_list_last, lift_list_contains, lift_list_slice,
    lift_list_reverse, lift_list_join, lift_list_is_empty,
};

pub use map::{
    lift_map_new, lift_map_retain, lift_map_release, lift_map_set,
    lift_map_get, lift_map_len, lift_map_free,
    // Map methods
    lift_map_keys, lift_map_values, lift_map_contains_key, lift_map_is_empty,
};

pub use range::{
    lift_range_new, lift_range_retain, lift_range_release,
    lift_range_start, lift_range_end, lift_range_free,
    lift_output_range,
};

pub use structs::{
    lift_struct_new, lift_struct_retain, lift_struct_release,
    lift_struct_set_field, lift_struct_get_field, lift_struct_get_field_type,
    lift_struct_free, lift_struct_eq, lift_output_struct,
};

pub use output::{
    lift_output_int, lift_output_float, lift_output_bool, lift_output_str,
    lift_output_newline, lift_output_list, lift_output_map,
};

// Helper functions that were in the original file
use std::ffi::CString;
use std::os::raw::c_char;

/// Create a LiftString from a Rust &str (for testing)
pub unsafe fn make_lift_string(s: &str) -> *mut c_char {
    let c_str = CString::new(s).expect("CString creation failed");
    c_str.into_raw()
}

/// Free a LiftString created by make_lift_string
pub unsafe fn free_lift_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

#[cfg(test)]
mod tests;
