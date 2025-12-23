// Output runtime functions for Lift

use std::os::raw::c_char;

use super::refcount::RefCounted;
use super::list::RcList;
use super::map::{RcMap, MapKey};
use super::structs::{RcStruct, format_struct_inline};

// Type constants for element identification
pub const TYPE_INT: i8 = 0;
pub const TYPE_FLT: i8 = 1;
pub const TYPE_BOOL: i8 = 2;
pub const TYPE_STR: i8 = 3;
pub const TYPE_LIST: i8 = 4;
pub const TYPE_MAP: i8 = 5;
pub const TYPE_RANGE: i8 = 6;
pub const TYPE_STRUCT: i8 = 7;

#[no_mangle]
pub unsafe extern "C" fn lift_output_int(value: i64) {
    print!("{} ", value);
}

#[no_mangle]
pub unsafe extern "C" fn lift_output_float(value: f64) {
    print!("{} ", value);
}

#[no_mangle]
pub unsafe extern "C" fn lift_output_bool(value: i8) {
    print!("{} ", if value != 0 { "true" } else { "false" });
}

#[no_mangle]
pub unsafe extern "C" fn lift_output_str(ptr: *const c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(ptr);
        if let Ok(s) = c_str.to_str() {
            // Lift strings include quotes - output them as-is to match interpreter
            print!("{} ", s);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_output_newline() {
    println!();
}

/// Format a list inline (without trailing space) for nested collections
unsafe fn format_list_inline(ptr: *const RcList) {
    if ptr.is_null() {
        print!("[]");
        return;
    }
    if let Some(list) = RefCounted::get(ptr) {
        print!("[");
        for (i, &val) in list.elements.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            format_value_inline(val, list.elem_type);
        }
        print!("]");
    } else {
        print!("[]");
    }
}

/// Format a map inline (without trailing space) for nested collections
unsafe fn format_map_inline(ptr: *const RcMap) {
    if ptr.is_null() {
        print!("{{}}");
        return;
    }
    if let Some(map) = RefCounted::get(ptr) {
        print!("{{");
        let mut keys: Vec<_> = map.entries.keys().collect();
        keys.sort();

        for (i, key) in keys.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            let val = map.entries[key];

            // Format key
            match key {
                MapKey::Int(v) => print!("{}", v),
                MapKey::Bool(b) => print!("{}", if *b { "true" } else { "false" }),
                MapKey::Str(s) => print!("{}", s),
            };
            print!(":");

            // Format value
            format_value_inline(val, map.value_type);
        }
        print!("}}");
    } else {
        print!("{{}}");
    }
}

/// Format a value inline (without trailing space) based on its type
pub unsafe fn format_value_inline(val: i64, type_tag: i8) {
    match type_tag {
        TYPE_INT => print!("{}", val),
        TYPE_FLT => {
            let f = f64::from_bits(val as u64);
            print!("{}", f);
        }
        TYPE_BOOL => print!("{}", if val != 0 { "true" } else { "false" }),
        TYPE_STR => {
            let str_ptr = val as *const c_char;
            if !str_ptr.is_null() {
                if let Ok(s) = std::ffi::CStr::from_ptr(str_ptr).to_str() {
                    print!("{}", s);
                }
            }
        }
        TYPE_LIST => {
            let nested_ptr = val as *const RcList;
            if !nested_ptr.is_null() {
                format_list_inline(nested_ptr);
            }
        }
        TYPE_MAP => {
            let map_ptr = val as *const RcMap;
            if !map_ptr.is_null() {
                format_map_inline(map_ptr);
            }
        }
        TYPE_STRUCT => {
            let struct_ptr = val as *const RcStruct;
            if !struct_ptr.is_null() {
                format_struct_inline(struct_ptr);
            }
        }
        _ => print!("{}", val),
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_output_list(ptr: *const RcList) {
    if ptr.is_null() {
        print!("[] ");
        return;
    }
    unsafe {
        if let Some(list) = RefCounted::get(ptr) {
            print!("[");
            for (i, &val) in list.elements.iter().enumerate() {
                if i > 0 {
                    print!(",");
                }
                // Format element based on its type
                match list.elem_type {
                    TYPE_INT => print!("{}", val),
                    TYPE_FLT => {
                        let f = f64::from_bits(val as u64);
                        print!("{}", f);
                    }
                    TYPE_BOOL => print!("{}", if val != 0 { "true" } else { "false" }),
                    TYPE_STR => {
                        // val is a pointer to a C string
                        let str_ptr = val as *const c_char;
                        if !str_ptr.is_null() {
                            let c_str = std::ffi::CStr::from_ptr(str_ptr);
                            if let Ok(s) = c_str.to_str() {
                                print!("{}", s); // Strings already have quotes
                            }
                        }
                    }
                    TYPE_LIST => {
                        // val is a pointer to a nested RcList - recursively format
                        let nested_ptr = val as *const RcList;
                        if !nested_ptr.is_null() {
                            format_list_inline(nested_ptr);
                        }
                    }
                    TYPE_MAP => {
                        // val is a pointer to a RcMap - recursively format
                        let map_ptr = val as *const RcMap;
                        if !map_ptr.is_null() {
                            format_map_inline(map_ptr);
                        }
                    }
                    _ => print!("{}", val), // Fallback for other types
                }
            }
            print!("] ");
        } else {
            print!("[] ");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_output_map(ptr: *const RcMap) {
    if ptr.is_null() {
        print!("{{}} ");
        return;
    }
    unsafe {
        if let Some(map) = RefCounted::get(ptr) {
            print!("{{");
            let mut keys: Vec<_> = map.entries.keys().collect();

            // Sort keys (MapKey implements Ord via derived traits)
            keys.sort();

            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    print!(",");
                }
                let val = map.entries[key];

                // Format key based on its type
                match key {
                    MapKey::Int(v) => print!("{}", v),
                    MapKey::Bool(b) => print!("{}", if *b { "true" } else { "false" }),
                    MapKey::Str(s) => print!("{}", s),
                };
                print!(":");

                // Format value (handles nested collections)
                format_value_inline(val, map.value_type);
            }
            print!("}} ");
        } else {
            print!("{{}} ");
        }
    }
}

