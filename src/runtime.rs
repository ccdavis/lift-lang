// Runtime library for Lift compiler
// These functions are called from JIT-compiled code to handle heap-allocated types

use std::ffi::CString;
use std::os::raw::c_char;

// Type tags for collection elements
// These correspond to Lift DataType variants
pub const TYPE_INT: i8 = 0;
pub const TYPE_FLT: i8 = 1;
pub const TYPE_BOOL: i8 = 2;
pub const TYPE_STR: i8 = 3;
pub const TYPE_LIST: i8 = 4;
pub const TYPE_MAP: i8 = 5;
pub const TYPE_RANGE: i8 = 6;

// ============================================================================
// Output Functions
// ============================================================================

#[no_mangle]
pub extern "C" fn lift_output_int(value: i64) {
    print!("{} ", value);
}

#[no_mangle]
pub extern "C" fn lift_output_float(value: f64) {
    print!("{} ", value);
}

#[no_mangle]
pub extern "C" fn lift_output_bool(value: i8) {
    print!("{} ", if value != 0 { "true" } else { "false" });
}

#[no_mangle]
pub extern "C" fn lift_output_str(ptr: *const c_char) {
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
pub extern "C" fn lift_output_newline() {
    println!();
}

/// Format a list inline (without trailing space) for nested collections
unsafe fn format_list_inline(ptr: *const LiftList) {
    if ptr.is_null() {
        print!("[]");
        return;
    }
    let list = &*ptr;
    print!("[");
    for (i, &val) in list.elements.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        format_value_inline(val, list.elem_type);
    }
    print!("]");
}

/// Format a map inline (without trailing space) for nested collections
unsafe fn format_map_inline(ptr: *const LiftMap) {
    if ptr.is_null() {
        print!("{{}}");
        return;
    }
    let map = &*ptr;
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
}

/// Format a value inline (without trailing space) based on its type
unsafe fn format_value_inline(val: i64, type_tag: i8) {
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
            let nested_ptr = val as *const LiftList;
            if !nested_ptr.is_null() {
                format_list_inline(nested_ptr);
            }
        }
        TYPE_MAP => {
            let map_ptr = val as *const LiftMap;
            if !map_ptr.is_null() {
                format_map_inline(map_ptr);
            }
        }
        _ => print!("{}", val),
    }
}

#[no_mangle]
pub extern "C" fn lift_output_list(ptr: *const LiftList) {
    if ptr.is_null() {
        print!("[] ");
        return;
    }
    unsafe {
        let list = &*ptr;
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
                            print!("{}", s);  // Strings already have quotes
                        }
                    }
                }
                TYPE_LIST => {
                    // val is a pointer to a nested LiftList - recursively format
                    let nested_ptr = val as *const LiftList;
                    if !nested_ptr.is_null() {
                        format_list_inline(nested_ptr);
                    }
                }
                TYPE_MAP => {
                    // val is a pointer to a LiftMap - recursively format
                    let map_ptr = val as *const LiftMap;
                    if !map_ptr.is_null() {
                        format_map_inline(map_ptr);
                    }
                }
                _ => print!("{}", val),  // Fallback for other types
            }
        }
        print!("] ");
    }
}

/// Helper function to format a value based on its type tag
unsafe fn format_value_by_type(val: i64, type_tag: i8) -> String {
    match type_tag {
        TYPE_INT => format!("{}", val),
        TYPE_FLT => {
            let f = f64::from_bits(val as u64);
            format!("{}", f)
        }
        TYPE_BOOL => format!("{}", if val != 0 { "true" } else { "false" }),
        TYPE_STR => {
            // val is a pointer to a C string
            let str_ptr = val as *const c_char;
            if !str_ptr.is_null() {
                let c_str = std::ffi::CStr::from_ptr(str_ptr);
                if let Ok(s) = c_str.to_str() {
                    return s.to_string();  // Strings already have quotes
                }
            }
            format!("{}", val)  // Fallback
        }
        _ => format!("{}", val),  // Fallback for other types
    }
}

#[no_mangle]
pub extern "C" fn lift_output_map(ptr: *const LiftMap) {
    if ptr.is_null() {
        print!("{{}} ");
        return;
    }
    unsafe {
        let map = &*ptr;
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
    pub elements: Vec<i64>,
    pub elem_type: i8,  // Type tag for elements (TYPE_INT, TYPE_STR, etc.)
}

/// Create a new list with given capacity and element type
#[no_mangle]
pub extern "C" fn lift_list_new(capacity: i64, elem_type: i8) -> *mut LiftList {
    let cap = capacity.max(0) as usize;
    let list = Box::new(LiftList {
        elements: Vec::with_capacity(cap),
        elem_type,
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
        // Resize if needed
        if idx >= list_ref.elements.len() {
            list_ref.elements.resize(idx + 1, 0);
        }
        list_ref.elements[idx] = value;
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
        if idx < list_ref.elements.len() {
            list_ref.elements[idx]
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
        list_ref.elements.len() as i64
    }
}

/// Free a list
#[no_mangle]
pub extern "C" fn lift_list_free(list: *mut LiftList) {
    if list.is_null() {
        return;
    }
    unsafe {
        let _list_box = Box::from_raw(list);
        // Vec will be automatically dropped
    }
}

// ============================================================================
// Map Functions (integer keys to integer values)
// ============================================================================

use std::collections::HashMap;

/// Map key that properly handles different types with correct equality/hashing
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum MapKey {
    Int(i64),
    Bool(bool),
    Str(String),  // Actual string content, not pointer address
}

impl MapKey {
    /// Convert an i64 (from FFI) to a MapKey based on type tag
    unsafe fn from_i64(val: i64, type_tag: i8) -> Option<Self> {
        match type_tag {
            TYPE_INT => Some(MapKey::Int(val)),
            TYPE_BOOL => Some(MapKey::Bool(val != 0)),
            TYPE_STR => {
                let ptr = val as *const c_char;
                if ptr.is_null() {
                    return None;
                }
                let c_str = std::ffi::CStr::from_ptr(ptr);
                if let Ok(s) = c_str.to_str() {
                    Some(MapKey::Str(s.to_string()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Convert MapKey back to i64 for FFI (used when iterating keys)
    fn to_i64(&self) -> i64 {
        match self {
            MapKey::Int(v) => *v,
            MapKey::Bool(b) => if *b { 1 } else { 0 },
            MapKey::Str(s) => {
                // Need to allocate a new C string
                let with_quotes = s.clone(); // String already has quotes
                if let Ok(c_str) = CString::new(with_quotes) {
                    c_str.into_raw() as i64
                } else {
                    0
                }
            }
        }
    }
}

/// Runtime representation of a map
#[repr(C)]
pub struct LiftMap {
    pub entries: HashMap<MapKey, i64>,
    pub key_type: i8,    // Type tag for keys (TYPE_INT, TYPE_STR, etc.)
    pub value_type: i8,  // Type tag for values
}

/// Create a new map with given capacity, key type, and value type
#[no_mangle]
pub extern "C" fn lift_map_new(capacity: i64, key_type: i8, value_type: i8) -> *mut LiftMap {
    let cap = capacity.max(0) as usize;
    let map = Box::new(LiftMap {
        entries: HashMap::with_capacity(cap),
        key_type,
        value_type,
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
        if let Some(map_key) = MapKey::from_i64(key, map_ref.key_type) {
            map_ref.entries.insert(map_key, value);
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
        if let Some(map_key) = MapKey::from_i64(key, map_ref.key_type) {
            map_ref.entries.get(&map_key).copied().unwrap_or(0)
        } else {
            0
        }
    }
}

/// Get the number of entries in a map
#[no_mangle]
pub extern "C" fn lift_map_len(map: *const LiftMap) -> i64 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        let map_ref = &*map;
        map_ref.entries.len() as i64
    }
}

/// Free a map
#[no_mangle]
pub extern "C" fn lift_map_free(map: *mut LiftMap) {
    if map.is_null() {
        return;
    }
    unsafe {
        let _map_box = Box::from_raw(map);
        // HashMap will be automatically dropped
    }
}

// ============================================================================
// Range Functions
// ============================================================================

/// Runtime representation of a range
#[repr(C)]
pub struct LiftRange {
    start: i64,
    end: i64,
}

/// Create a new range
#[no_mangle]
pub extern "C" fn lift_range_new(start: i64, end: i64) -> *mut LiftRange {
    let range = Box::new(LiftRange { start, end });
    Box::into_raw(range)
}

/// Get the start of a range
#[no_mangle]
pub extern "C" fn lift_range_start(range: *const LiftRange) -> i64 {
    if range.is_null() {
        return 0;
    }
    unsafe {
        let range_ref = &*range;
        range_ref.start
    }
}

/// Get the end of a range
#[no_mangle]
pub extern "C" fn lift_range_end(range: *const LiftRange) -> i64 {
    if range.is_null() {
        return 0;
    }
    unsafe {
        let range_ref = &*range;
        range_ref.end
    }
}

/// Free a range
#[no_mangle]
pub extern "C" fn lift_range_free(range: *mut LiftRange) {
    if !range.is_null() {
        unsafe {
            let _ = Box::from_raw(range);
        }
    }
}

/// Output a range
#[no_mangle]
pub extern "C" fn lift_output_range(range: *const LiftRange) {
    if range.is_null() {
        print!("null ");
        return;
    }
    unsafe {
        let range_ref = &*range;
        print!("{}..{} ", range_ref.start, range_ref.end);
    }
}

// ==================== String Methods ====================

#[no_mangle]
pub extern "C" fn lift_str_upper(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            let trimmed = rust_str.trim_matches('\'');
            let upper = trimmed.to_uppercase();
            let result = format!("'{}'", upper);
            if let Ok(c_result) = CString::new(result) {
                return c_result.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_lower(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            let trimmed = rust_str.trim_matches('\'');
            let lower = trimmed.to_lowercase();
            let result = format!("'{}'", lower);
            if let Ok(c_result) = CString::new(result) {
                return c_result.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_substring(s: *const c_char, start: i64, end: i64) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            let trimmed = rust_str.trim_matches('\'');
            let start_idx = start.max(0) as usize;
            let end_idx = end.min(trimmed.len() as i64) as usize;
            if start_idx <= end_idx && end_idx <= trimmed.len() {
                let substring = &trimmed[start_idx..end_idx];
                let result = format!("'{}'", substring);
                if let Ok(c_result) = CString::new(result) {
                    return c_result.into_raw();
                }
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_contains(s: *const c_char, needle: *const c_char) -> i8 {
    if s.is_null() || needle.is_null() {
        return 0;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_needle = std::ffi::CStr::from_ptr(needle);
        if let (Ok(rust_str), Ok(rust_needle)) = (c_str.to_str(), c_needle.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let needle_trimmed = rust_needle.trim_matches('\'');
            return if trimmed.contains(needle_trimmed) { 1 } else { 0 };
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn lift_str_trim(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            let trimmed_quotes = rust_str.trim_matches('\'');
            let trimmed = trimmed_quotes.trim();
            let result = format!("'{}'", trimmed);
            if let Ok(c_result) = CString::new(result) {
                return c_result.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_split(s: *const c_char, delimiter: *const c_char) -> *mut LiftList {
    if s.is_null() || delimiter.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_delim = std::ffi::CStr::from_ptr(delimiter);
        if let (Ok(rust_str), Ok(rust_delim)) = (c_str.to_str(), c_delim.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let delim_trimmed = rust_delim.trim_matches('\'');
            let parts: Vec<i64> = trimmed
                .split(delim_trimmed)
                .map(|part| {
                    let formatted = format!("'{}'", part);
                    let c_string = CString::new(formatted).unwrap();
                    c_string.into_raw() as i64
                })
                .collect();

            let list = Box::new(LiftList {
                elements: parts,
                elem_type: TYPE_STR,
            });
            return Box::into_raw(list);
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_replace(s: *const c_char, old: *const c_char, new: *const c_char) -> *mut c_char {
    if s.is_null() || old.is_null() || new.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_old = std::ffi::CStr::from_ptr(old);
        let c_new = std::ffi::CStr::from_ptr(new);
        if let (Ok(rust_str), Ok(rust_old), Ok(rust_new)) = (c_str.to_str(), c_old.to_str(), c_new.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let old_trimmed = rust_old.trim_matches('\'');
            let new_trimmed = rust_new.trim_matches('\'');
            let replaced = trimmed.replace(old_trimmed, new_trimmed);
            let result = format!("'{}'", replaced);
            if let Ok(c_result) = CString::new(result) {
                return c_result.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_str_starts_with(s: *const c_char, prefix: *const c_char) -> i8 {
    if s.is_null() || prefix.is_null() {
        return 0;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_prefix = std::ffi::CStr::from_ptr(prefix);
        if let (Ok(rust_str), Ok(rust_prefix)) = (c_str.to_str(), c_prefix.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let prefix_trimmed = rust_prefix.trim_matches('\'');
            return if trimmed.starts_with(prefix_trimmed) { 1 } else { 0 };
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn lift_str_ends_with(s: *const c_char, suffix: *const c_char) -> i8 {
    if s.is_null() || suffix.is_null() {
        return 0;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_suffix = std::ffi::CStr::from_ptr(suffix);
        if let (Ok(rust_str), Ok(rust_suffix)) = (c_str.to_str(), c_suffix.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let suffix_trimmed = rust_suffix.trim_matches('\'');
            return if trimmed.ends_with(suffix_trimmed) { 1 } else { 0 };
        }
    }
    0
}

#[no_mangle]
pub extern "C" fn lift_str_is_empty(s: *const c_char) -> i8 {
    if s.is_null() {
        return 1;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            let trimmed = rust_str.trim_matches('\'');
            return if trimmed.is_empty() { 1 } else { 0 };
        }
    }
    1
}

// ==================== List Methods ====================

#[no_mangle]
pub extern "C" fn lift_list_first(list: *const LiftList) -> i64 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        let list_ref = &*list;
        if list_ref.elements.is_empty() {
            panic!("Cannot get first element of empty list");
        }
        list_ref.elements[0]
    }
}

#[no_mangle]
pub extern "C" fn lift_list_last(list: *const LiftList) -> i64 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        let list_ref = &*list;
        if list_ref.elements.is_empty() {
            panic!("Cannot get last element of empty list");
        }
        list_ref.elements[list_ref.elements.len() - 1]
    }
}

#[no_mangle]
pub extern "C" fn lift_list_contains(list: *const LiftList, item: i64) -> i8 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        let list_ref = &*list;
        if list_ref.elements.contains(&item) { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn lift_list_slice(list: *const LiftList, start: i64, end: i64) -> *mut LiftList {
    if list.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let list_ref = &*list;
        let start_idx = start.max(0) as usize;
        let end_idx = end.min(list_ref.elements.len() as i64) as usize;

        let sliced: Vec<i64> = if start_idx <= end_idx {
            list_ref.elements[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        let new_list = Box::new(LiftList {
            elements: sliced,
            elem_type: list_ref.elem_type,  // Preserve element type from original list
        });
        Box::into_raw(new_list)
    }
}

#[no_mangle]
pub extern "C" fn lift_list_reverse(list: *const LiftList) -> *mut LiftList {
    if list.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let list_ref = &*list;
        let mut reversed = list_ref.elements.clone();
        reversed.reverse();

        let new_list = Box::new(LiftList {
            elements: reversed,
            elem_type: list_ref.elem_type,  // Preserve element type from original list
        });
        Box::into_raw(new_list)
    }
}

#[no_mangle]
pub extern "C" fn lift_list_join(list: *const LiftList, separator: *const c_char) -> *mut c_char {
    if list.is_null() || separator.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let list_ref = &*list;
        let c_sep = std::ffi::CStr::from_ptr(separator);
        if let Ok(rust_sep) = c_sep.to_str() {
            let sep_trimmed = rust_sep.trim_matches('\'');

            // Convert i64 elements (which are string pointers) to strings
            let strings: Vec<String> = list_ref.elements.iter().map(|&elem| {
                let str_ptr = elem as *const c_char;
                if !str_ptr.is_null() {
                    if let Ok(c_str) = std::ffi::CStr::from_ptr(str_ptr).to_str() {
                        c_str.trim_matches('\'').to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }).collect();

            let joined = strings.join(sep_trimmed);
            let result = format!("'{}'", joined);
            if let Ok(c_result) = CString::new(result) {
                return c_result.into_raw();
            }
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn lift_list_is_empty(list: *const LiftList) -> i8 {
    if list.is_null() {
        return 1;
    }
    unsafe {
        let list_ref = &*list;
        if list_ref.elements.is_empty() { 1 } else { 0 }
    }
}

// ==================== Map Methods ====================

#[no_mangle]
pub extern "C" fn lift_map_keys(map: *const LiftMap) -> *mut LiftList {
    if map.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let map_ref = &*map;
        let mut keys: Vec<&MapKey> = map_ref.entries.keys().collect();
        keys.sort(); // Sort for consistency

        // Convert MapKey back to i64 for FFI
        let key_values: Vec<i64> = keys.iter().map(|k| k.to_i64()).collect();

        let list = Box::new(LiftList {
            elements: key_values,
            elem_type: map_ref.key_type,  // Keys have the map's key type
        });
        Box::into_raw(list)
    }
}

#[no_mangle]
pub extern "C" fn lift_map_values(map: *const LiftMap) -> *mut LiftList {
    if map.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let map_ref = &*map;
        let mut key_value_pairs: Vec<(&MapKey, &i64)> = map_ref.entries.iter().collect();
        key_value_pairs.sort_by_key(|&(k, _)| k); // Sort by key

        let values: Vec<i64> = key_value_pairs.iter().map(|&(_, v)| *v).collect();

        let list = Box::new(LiftList {
            elements: values,
            elem_type: map_ref.value_type,  // Values have the map's value type
        });
        Box::into_raw(list)
    }
}

#[no_mangle]
pub extern "C" fn lift_map_contains_key(map: *const LiftMap, key: i64) -> i8 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        let map_ref = &*map;
        if let Some(map_key) = MapKey::from_i64(key, map_ref.key_type) {
            if map_ref.entries.contains_key(&map_key) { 1 } else { 0 }
        } else {
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn lift_map_is_empty(map: *const LiftMap) -> i8 {
    if map.is_null() {
        return 1;
    }
    unsafe {
        let map_ref = &*map;
        if map_ref.entries.is_empty() { 1 } else { 0 }
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
