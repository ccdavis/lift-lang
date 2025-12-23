// List runtime functions for Lift


use super::refcount::RefCounted;
use super::string::LiftString;
use super::output::TYPE_INT;

/// Runtime representation of a list
#[repr(C)]
pub struct LiftList {
    pub elements: Vec<i64>,
    pub elem_type: i8, // Type tag for elements (TYPE_INT, TYPE_STR, etc.)
}

/// Type alias for reference-counted list
pub type RcList = RefCounted<LiftList>;

/// Create a new list with given capacity and element type
#[no_mangle]
pub unsafe extern "C" fn lift_list_new(capacity: i64, elem_type: i8) -> *mut RcList {
    let cap = capacity.max(0) as usize;
    let list = LiftList {
        elements: Vec::with_capacity(cap),
        elem_type,
    };
    RefCounted::new(list)
}

/// Increment reference count for a list (used when copying pointers)
#[no_mangle]
pub unsafe extern "C" fn lift_list_retain(list: *mut RcList) {
    RefCounted::retain(list);
}

/// Decrement reference count for a list and free if it reaches zero
#[no_mangle]
pub unsafe extern "C" fn lift_list_release(list: *mut RcList) {
    if RefCounted::release(list) {
        // List was freed - recursively release nested collections
        // Note: This happens automatically via Drop, but we need to manually
        // release nested ref-counted types
    }
}

/// Set an element in the list at given index
#[no_mangle]
pub unsafe extern "C" fn lift_list_set(list: *mut RcList, index: i64, value: i64) {
    if list.is_null() || index < 0 {
        return;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get_mut(list) {
            let idx = index as usize;
            // Resize if needed
            if idx >= list_ref.elements.len() {
                list_ref.elements.resize(idx + 1, 0);
            }
            list_ref.elements[idx] = value;
        }
    }
}

/// Get an element from the list at given index
#[no_mangle]
pub unsafe extern "C" fn lift_list_get(list: *const RcList, index: i64) -> i64 {
    if list.is_null() || index < 0 {
        return 0;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            let idx = index as usize;
            if idx < list_ref.elements.len() {
                list_ref.elements[idx]
            } else {
                0
            }
        } else {
            0
        }
    }
}

/// Get the length of a list
#[no_mangle]
pub unsafe extern "C" fn lift_list_len(list: *const RcList) -> i64 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            list_ref.elements.len() as i64
        } else {
            0
        }
    }
}

/// Push an element to the end of the list (like Vec::push)
/// The list will grow dynamically as needed
#[no_mangle]
pub unsafe extern "C" fn lift_list_push(list: *mut RcList, value: i64) {
    if list.is_null() {
        return;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get_mut(list) {
            list_ref.elements.push(value);
        }
    }
}

/// Reserve capacity for additional elements (like Vec::reserve)
/// This pre-allocates memory without changing the length
#[no_mangle]
pub unsafe extern "C" fn lift_list_reserve(list: *mut RcList, additional: i64) {
    if list.is_null() || additional <= 0 {
        return;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get_mut(list) {
            list_ref.elements.reserve(additional as usize);
        }
    }
}

/// Concatenate two lists, returning a new list
/// The original lists are not modified
#[no_mangle]
pub unsafe extern "C" fn lift_list_concat(
    list1: *const RcList,
    list2: *const RcList,
) -> *mut RcList {
    // Handle null cases
    if list1.is_null() && list2.is_null() {
        return lift_list_new(0, TYPE_INT);
    }

    unsafe {
        let (elem_type, new_elements) = if list1.is_null() {
            if let Some(l2) = RefCounted::get(list2) {
                (l2.elem_type, l2.elements.clone())
            } else {
                return lift_list_new(0, TYPE_INT);
            }
        } else if list2.is_null() {
            if let Some(l1) = RefCounted::get(list1) {
                (l1.elem_type, l1.elements.clone())
            } else {
                return lift_list_new(0, TYPE_INT);
            }
        } else {
            let l1 = RefCounted::get(list1);
            let l2 = RefCounted::get(list2);
            match (l1, l2) {
                (Some(l1_ref), Some(l2_ref)) => {
                    let mut combined = Vec::with_capacity(l1_ref.elements.len() + l2_ref.elements.len());
                    combined.extend_from_slice(&l1_ref.elements);
                    combined.extend_from_slice(&l2_ref.elements);
                    (l1_ref.elem_type, combined)
                }
                (Some(l1_ref), None) => (l1_ref.elem_type, l1_ref.elements.clone()),
                (None, Some(l2_ref)) => (l2_ref.elem_type, l2_ref.elements.clone()),
                (None, None) => return lift_list_new(0, TYPE_INT),
            }
        };

        let list = LiftList {
            elements: new_elements,
            elem_type,
        };
        RefCounted::new(list)
    }
}

/// Free a list (deprecated - use lift_list_release instead)
#[no_mangle]
pub unsafe extern "C" fn lift_list_free(list: *mut RcList) {
    lift_list_release(list);
}


// ==================== List Methods ====================

#[no_mangle]
pub unsafe extern "C" fn lift_list_first(list: *const RcList) -> i64 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            if list_ref.elements.is_empty() {
                panic!("Cannot get first element of empty list");
            }
            list_ref.elements[0]
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_list_last(list: *const RcList) -> i64 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            if list_ref.elements.is_empty() {
                panic!("Cannot get last element of empty list");
            }
            list_ref.elements[list_ref.elements.len() - 1]
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_list_contains(list: *const RcList, item: i64) -> i8 {
    if list.is_null() {
        return 0;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            if list_ref.elements.contains(&item) {
                1
            } else {
                0
            }
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_list_slice(list: *const RcList, start: i64, end: i64) -> *mut RcList {
    if list.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            let start_idx = start.max(0) as usize;
            let end_idx = end.min(list_ref.elements.len() as i64) as usize;

            let sliced: Vec<i64> = if start_idx <= end_idx {
                list_ref.elements[start_idx..end_idx].to_vec()
            } else {
                Vec::new()
            };

            let new_list = LiftList {
                elements: sliced,
                elem_type: list_ref.elem_type, // Preserve element type from original list
            };
            RefCounted::new(new_list)
        } else {
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_list_reverse(list: *const RcList) -> *mut RcList {
    if list.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            let mut reversed = list_ref.elements.clone();
            reversed.reverse();

            let new_list = LiftList {
                elements: reversed,
                elem_type: list_ref.elem_type, // Preserve element type from original list
            };
            RefCounted::new(new_list)
        } else {
            std::ptr::null_mut()
        }
    }
}

/// Join a list of strings with a separator, returning result in dest pointer
/// dest: pointer to uninitialized LiftString (32 bytes)
/// list: pointer to RcList containing LiftString pointers
/// separator: pointer to LiftString separator
#[no_mangle]
pub unsafe extern "C" fn lift_list_join(
    dest: *mut LiftString,
    list: *const RcList,
    separator: *const LiftString,
) {
    if dest.is_null() || list.is_null() || separator.is_null() {
        if !dest.is_null() {
            std::ptr::write(dest, LiftString::from_str("''"));
        }
        return;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            // Get separator string content (trim quotes)
            let sep_bytes = (*separator).as_bytes();
            let sep_str = std::str::from_utf8(sep_bytes).unwrap_or("");
            let sep_trimmed = sep_str.trim_matches('\'');

            // Convert i64 elements (which are LiftString pointers) to strings
            let strings: Vec<String> = list_ref
                .elements
                .iter()
                .map(|&elem| {
                    let str_ptr = elem as *const LiftString;
                    if !str_ptr.is_null() {
                        let bytes = (*str_ptr).as_bytes();
                        if let Ok(s) = std::str::from_utf8(bytes) {
                            s.trim_matches('\'').to_string()
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                })
                .collect();

            let joined = strings.join(sep_trimmed);
            let result = format!("'{}'", joined);
            std::ptr::write(dest, LiftString::from_str(&result));
        } else {
            std::ptr::write(dest, LiftString::from_str("''"));
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn lift_list_is_empty(list: *const RcList) -> i8 {
    if list.is_null() {
        return 1;
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            if list_ref.elements.is_empty() {
                1
            } else {
                0
            }
        } else {
            1
        }
    }
}

