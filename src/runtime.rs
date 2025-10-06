// Runtime library for Lift compiler
// These functions are called from JIT-compiled code to handle heap-allocated types

use std::ffi::CString;
use std::os::raw::c_char;

// ============================================================================
// Output Functions
// ============================================================================

#[no_mangle]
pub extern "C" fn lift_output_int(value: i64) {
    println!("{}", value);
}

#[no_mangle]
pub extern "C" fn lift_output_float(value: f64) {
    println!("{}", value);
}

#[no_mangle]
pub extern "C" fn lift_output_bool(value: i8) {
    println!("{}", if value != 0 { "true" } else { "false" });
}

#[no_mangle]
pub extern "C" fn lift_output_str(ptr: *const c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(ptr);
        if let Ok(s) = c_str.to_str() {
            // Remove quotes if present (Lift strings include quotes)
            let trimmed = s.trim_matches('\'');
            println!("{}", trimmed);
        }
    }
}

// ============================================================================
// String Functions
// ============================================================================

#[no_mangle]
pub extern "C" fn lift_str_new(ptr: *const c_char) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(ptr);
        if let Ok(s) = c_str.to_str() {
            if let Ok(new_cstr) = CString::new(s) {
                return new_cstr.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_concat(s1: *const c_char, s2: *const c_char) -> *mut c_char {
    if s1.is_null() || s2.is_null() {
        return std::ptr::null_mut();
    }

    unsafe {
        let str1 = std::ffi::CStr::from_ptr(s1);
        let str2 = std::ffi::CStr::from_ptr(s2);

        if let (Ok(s1), Ok(s2)) = (str1.to_str(), str2.to_str()) {
            // Remove quotes from both strings
            let s1_trimmed = s1.trim_matches('\'');
            let s2_trimmed = s2.trim_matches('\'');

            // Concatenate and add quotes back
            let result = format!("'{}{}'", s1_trimmed, s2_trimmed);

            if let Ok(c_result) = CString::new(result) {
                return c_result.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_len(ptr: *const c_char) -> i64 {
    if ptr.is_null() {
        return 0;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(ptr);
        if let Ok(s) = c_str.to_str() {
            // Don't count the quotes
            let trimmed = s.trim_matches('\'');
            return trimmed.len() as i64;
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn lift_str_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}

// ============================================================================
// Comparison Functions
// ============================================================================

#[no_mangle]
pub extern "C" fn lift_str_eq(s1: *const c_char, s2: *const c_char) -> i8 {
    if s1.is_null() && s2.is_null() {
        return 1;
    }
    if s1.is_null() || s2.is_null() {
        return 0;
    }

    unsafe {
        let str1 = std::ffi::CStr::from_ptr(s1);
        let str2 = std::ffi::CStr::from_ptr(s2);

        if str1 == str2 {
            1
        } else {
            0
        }
    }
}

// ============================================================================
// Helper Functions for Testing
// ============================================================================

/// Create a Lift string from a Rust string (for testing)
pub fn make_lift_string(s: &str) -> *mut c_char {
    let with_quotes = format!("'{}'", s);
    if let Ok(c_str) = CString::new(with_quotes) {
        c_str.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Free a Lift string (for testing)
pub fn free_lift_string(ptr: *mut c_char) {
    lift_str_free(ptr);
}

// ============================================================================
// List Functions (for integers)
// ============================================================================

/// Runtime representation of a list
#[repr(C)]
pub struct LiftList {
    data: *mut i64,
    len: usize,
    capacity: usize,
}

/// Create a new list with given capacity
#[no_mangle]
pub extern "C" fn lift_list_new(capacity: i64) -> *mut LiftList {
    let cap = capacity.max(0) as usize;
    let list = Box::new(LiftList {
        data: if cap > 0 {
            unsafe {
                let layout = std::alloc::Layout::array::<i64>(cap).unwrap();
                std::alloc::alloc(layout) as *mut i64
            }
        } else {
            std::ptr::null_mut()
        },
        len: 0,
        capacity: cap,
    });
    Box::into_raw(list)
}

/// Set an element in the list at given index
#[no_mangle]
pub extern "C" fn lift_list_set(list: *mut LiftList, index: i64, value: i64) {
    if list.is_null() || index < 0 {
        return;
    }
    unsafe {
        let list_ref = &mut *list;
        let idx = index as usize;
        if idx < list_ref.capacity {
            *list_ref.data.add(idx) = value;
            if idx >= list_ref.len {
                list_ref.len = idx + 1;
            }
        }
    }
}

/// Get an element from the list at given index
#[no_mangle]
pub extern "C" fn lift_list_get(list: *const LiftList, index: i64) -> i64 {
    if list.is_null() || index < 0 {
        return 0;
    }
    unsafe {
        let list_ref = &*list;
        let idx = index as usize;
        if idx < list_ref.len {
            *list_ref.data.add(idx)
        } else {
            0
        }
    }
}

/// Get the length of a list
#[no_mangle]
pub extern "C" fn lift_list_len(list: *const LiftList) -> i64 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        let list_ref = &*list;
        list_ref.len as i64
    }
}

/// Free a list
#[no_mangle]
pub extern "C" fn lift_list_free(list: *mut LiftList) {
    if list.is_null() {
        return;
    }
    unsafe {
        let list_box = Box::from_raw(list);
        if !list_box.data.is_null() && list_box.capacity > 0 {
            let layout = std::alloc::Layout::array::<i64>(list_box.capacity).unwrap();
            std::alloc::dealloc(list_box.data as *mut u8, layout);
        }
    }
}

// ============================================================================
// Map Functions (integer keys to integer values)
// ============================================================================

/// Runtime representation of a map (simple array-based for now)
#[repr(C)]
pub struct LiftMap {
    keys: *mut i64,
    values: *mut i64,
    len: usize,
    capacity: usize,
}

/// Create a new map with given capacity
#[no_mangle]
pub extern "C" fn lift_map_new(capacity: i64) -> *mut LiftMap {
    let cap = capacity.max(0) as usize;
    let map = Box::new(LiftMap {
        keys: if cap > 0 {
            unsafe {
                let layout = std::alloc::Layout::array::<i64>(cap).unwrap();
                std::alloc::alloc(layout) as *mut i64
            }
        } else {
            std::ptr::null_mut()
        },
        values: if cap > 0 {
            unsafe {
                let layout = std::alloc::Layout::array::<i64>(cap).unwrap();
                std::alloc::alloc(layout) as *mut i64
            }
        } else {
            std::ptr::null_mut()
        },
        len: 0,
        capacity: cap,
    });
    Box::into_raw(map)
}

/// Set a key-value pair in the map
#[no_mangle]
pub extern "C" fn lift_map_set(map: *mut LiftMap, key: i64, value: i64) {
    if map.is_null() {
        return;
    }
    unsafe {
        let map_ref = &mut *map;

        // Check if key already exists
        for i in 0..map_ref.len {
            if *map_ref.keys.add(i) == key {
                *map_ref.values.add(i) = value;
                return;
            }
        }

        // Add new key-value pair if capacity allows
        if map_ref.len < map_ref.capacity {
            *map_ref.keys.add(map_ref.len) = key;
            *map_ref.values.add(map_ref.len) = value;
            map_ref.len += 1;
        }
    }
}

/// Get a value from the map by key
#[no_mangle]
pub extern "C" fn lift_map_get(map: *const LiftMap, key: i64) -> i64 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        let map_ref = &*map;
        for i in 0..map_ref.len {
            if *map_ref.keys.add(i) == key {
                return *map_ref.values.add(i);
            }
        }
        0 // Key not found
    }
}

/// Free a map
#[no_mangle]
pub extern "C" fn lift_map_free(map: *mut LiftMap) {
    if map.is_null() {
        return;
    }
    unsafe {
        let map_box = Box::from_raw(map);
        if !map_box.keys.is_null() && map_box.capacity > 0 {
            let layout = std::alloc::Layout::array::<i64>(map_box.capacity).unwrap();
            std::alloc::dealloc(map_box.keys as *mut u8, layout);
            std::alloc::dealloc(map_box.values as *mut u8, layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_concat() {
        unsafe {
            let s1 = make_lift_string("Hello");
            let s2 = make_lift_string(" World");

            let result = lift_str_concat(s1, s2);
            assert!(!result.is_null());

            let c_str = std::ffi::CStr::from_ptr(result);
            assert_eq!(c_str.to_str().unwrap(), "'Hello World'");

            lift_str_free(s1 as *mut c_char);
            lift_str_free(s2 as *mut c_char);
            lift_str_free(result);
        }
    }

    #[test]
    fn test_string_length() {
        unsafe {
            let s = make_lift_string("Hello");
            let len = lift_str_len(s);
            assert_eq!(len, 5);
            lift_str_free(s as *mut c_char);
        }
    }

    #[test]
    fn test_string_equality() {
        unsafe {
            let s1 = make_lift_string("test");
            let s2 = make_lift_string("test");
            let s3 = make_lift_string("other");

            assert_eq!(lift_str_eq(s1, s2), 1);
            assert_eq!(lift_str_eq(s1, s3), 0);

            lift_str_free(s1 as *mut c_char);
            lift_str_free(s2 as *mut c_char);
            lift_str_free(s3 as *mut c_char);
        }
    }
}
