// Map runtime functions for Lift

use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::c_char;

use super::refcount::RefCounted;
use super::list::{LiftList, RcList};
use super::output::{TYPE_INT, TYPE_BOOL, TYPE_STR};

/// Map key that properly handles different types with correct equality/hashing
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MapKey {
    Int(i64),
    Bool(bool),
    Str(String), // Actual string content, not pointer address
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
            MapKey::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
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
    pub key_type: i8,   // Type tag for keys (TYPE_INT, TYPE_STR, etc.)
    pub value_type: i8, // Type tag for values
}

/// Type alias for reference-counted map
pub type RcMap = RefCounted<LiftMap>;

/// Create a new map with given capacity, key type, and value type
#[no_mangle]
pub unsafe extern "C" fn lift_map_new(capacity: i64, key_type: i8, value_type: i8) -> *mut RcMap {
    let cap = capacity.max(0) as usize;
    let map = LiftMap {
        entries: HashMap::with_capacity(cap),
        key_type,
        value_type,
    };
    RefCounted::new(map)
}

/// Increment reference count for a map
#[no_mangle]
pub unsafe extern "C" fn lift_map_retain(map: *mut RcMap) {
    RefCounted::retain(map);
}

/// Decrement reference count for a map and free if it reaches zero
#[no_mangle]
pub unsafe extern "C" fn lift_map_release(map: *mut RcMap) {
    RefCounted::release(map);
}

/// Set a key-value pair in the map
#[no_mangle]
pub unsafe extern "C" fn lift_map_set(map: *mut RcMap, key: i64, value: i64) {
    if map.is_null() {
        return;
    }
    unsafe {
        if let Some(map_ref) = RefCounted::get_mut(map) {
            if let Some(map_key) = MapKey::from_i64(key, map_ref.key_type) {
                map_ref.entries.insert(map_key, value);
            }
        }
    }
}

/// Get a value from the map by key
#[no_mangle]
pub unsafe extern "C" fn lift_map_get(map: *const RcMap, key: i64) -> i64 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        if let Some(map_ref) = RefCounted::get(map) {
            if let Some(map_key) = MapKey::from_i64(key, map_ref.key_type) {
                map_ref.entries.get(&map_key).copied().unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        }
    }
}

/// Get the number of entries in a map
#[no_mangle]
pub unsafe extern "C" fn lift_map_len(map: *const RcMap) -> i64 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        if let Some(map_ref) = RefCounted::get(map) {
            map_ref.entries.len() as i64
        } else {
            0
        }
    }
}

/// Free a map (deprecated - use lift_map_release instead)
#[no_mangle]
pub unsafe extern "C" fn lift_map_free(map: *mut RcMap) {
    lift_map_release(map);
}


// ==================== Map Methods ====================

#[no_mangle]
pub unsafe extern "C" fn lift_map_keys(map: *const RcMap) -> *mut RcList {
    if map.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if let Some(map_ref) = RefCounted::get(map) {
            let mut keys: Vec<&MapKey> = map_ref.entries.keys().collect();
            keys.sort(); // Sort for consistency

            // Convert MapKey back to i64 for FFI
            let key_values: Vec<i64> = keys.iter().map(|k| k.to_i64()).collect();

            let list = LiftList {
                elements: key_values,
                elem_type: map_ref.key_type, // Keys have the map's key type
            };
            RefCounted::new(list)
        } else {
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_map_values(map: *const RcMap) -> *mut RcList {
    if map.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if let Some(map_ref) = RefCounted::get(map) {
            let mut key_value_pairs: Vec<(&MapKey, &i64)> = map_ref.entries.iter().collect();
            key_value_pairs.sort_by_key(|&(k, _)| k); // Sort by key

            let values: Vec<i64> = key_value_pairs.iter().map(|&(_, v)| *v).collect();

            let list = LiftList {
                elements: values,
                elem_type: map_ref.value_type, // Values have the map's value type
            };
            RefCounted::new(list)
        } else {
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_map_contains_key(map: *const RcMap, key: i64) -> i8 {
    if map.is_null() {
        return 0;
    }
    unsafe {
        if let Some(map_ref) = RefCounted::get(map) {
            if let Some(map_key) = MapKey::from_i64(key, map_ref.key_type) {
                if map_ref.entries.contains_key(&map_key) {
                    1
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_map_is_empty(map: *const RcMap) -> i8 {
    if map.is_null() {
        return 1;
    }
    unsafe {
        if let Some(map_ref) = RefCounted::get(map) {
            if map_ref.entries.is_empty() {
                1
            } else {
                0
            }
        } else {
            1
        }
    }
}
