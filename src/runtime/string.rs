// String runtime functions for Lift
// Includes LiftString with Small String Optimization (SSO)

use std::ffi::CString;
use std::os::raw::c_char;

use super::refcount::RefCounted;
use super::list::{LiftList, RcList};
use super::output::TYPE_STR;

/// Maximum number of bytes that can be stored inline in a small string
const SMALL_STRING_CAPACITY: usize = 23;

/// A string type with Small String Optimization
/// - Small strings (≤ 23 bytes): stored inline on the stack
/// - Large strings (> 23 bytes): stored on the heap with reference counting
/// - Total size: exactly 32 bytes
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LiftString {
    /// Multi-purpose buffer:
    /// - For small strings: holds the actual string bytes (up to 23 bytes)
    /// - For large strings: first 8 bytes hold *mut RefCounted<Vec<u8>>
    pub data: [u8; 24],

    /// String length in bytes:
    /// - If len <= 23: this is a small string, data is inline
    /// - If len > 23: this is a large string, data[0..8] contains heap pointer
    pub len: u64,
}

impl LiftString {
    /// Create a new LiftString from a Rust &str
    pub fn from_str(s: &str) -> Self {
        let len = s.len();

        if len <= SMALL_STRING_CAPACITY {
            // Small string - store inline
            let mut data = [0u8; 24];
            data[..len].copy_from_slice(s.as_bytes());
            LiftString {
                data,
                len: len as u64,
            }
        } else {
            // Large string - allocate on heap with reference counting
            let vec = s.as_bytes().to_vec();
            let rc_ptr = RefCounted::new(vec);

            let mut data = [0u8; 24];
            // Store the pointer in the first 8 bytes
            let ptr_bytes = (rc_ptr as usize).to_ne_bytes();
            data[..8].copy_from_slice(&ptr_bytes);

            LiftString {
                data,
                len: len as u64,
            }
        }
    }

    /// Check if this is a small (inline) string
    #[inline]
    pub fn is_small(&self) -> bool {
        self.len <= SMALL_STRING_CAPACITY as u64
    }

    /// Get the string as a byte slice
    pub unsafe fn as_bytes(&self) -> &[u8] {
        if self.is_small() {
            // Small string - return slice of inline data
            &self.data[..self.len as usize]
        } else {
            // Large string - dereference heap pointer
            let ptr_bytes: [u8; 8] = self.data[..8].try_into().unwrap();
            let ptr = usize::from_ne_bytes(ptr_bytes) as *mut RefCounted<Vec<u8>>;

            if let Some(vec) = RefCounted::get(ptr) {
                vec.as_slice()
            } else {
                &[]
            }
        }
    }

    /// Convert to a C string for output/printing
    pub unsafe fn to_cstring(&self) -> CString {
        let bytes = self.as_bytes();
        CString::new(bytes).unwrap_or_else(|_| CString::new("").unwrap())
    }

    /// Retain (increment refcount) for large strings
    pub unsafe fn retain(&self) {
        if !self.is_small() {
            let ptr_bytes: [u8; 8] = self.data[..8].try_into().unwrap();
            let ptr = usize::from_ne_bytes(ptr_bytes) as *mut RefCounted<Vec<u8>>;
            RefCounted::retain(ptr);
        }
        // Small strings don't need refcounting
    }

    /// Release (decrement refcount) for large strings
    pub unsafe fn release(&self) {
        if !self.is_small() {
            let ptr_bytes: [u8; 8] = self.data[..8].try_into().unwrap();
            let ptr = usize::from_ne_bytes(ptr_bytes) as *mut RefCounted<Vec<u8>>;
            RefCounted::release(ptr);
        }
        // Small strings don't need cleanup
    }

    /// Concatenate two strings, returning a new string
    /// Handles Lift string literals which include quotes
    pub unsafe fn concat(&self, other: &LiftString) -> LiftString {
        let self_bytes = self.as_bytes();
        let other_bytes = other.as_bytes();

        // Lift strings include quotes - need to trim and re-add
        // 'Hello' + ' World' should become 'Hello World', not 'Hello'' World'
        let self_trimmed = if self_bytes.len() >= 2
            && self_bytes[0] == b'\''
            && self_bytes[self_bytes.len() - 1] == b'\''
        {
            &self_bytes[1..self_bytes.len() - 1]
        } else {
            self_bytes
        };

        let other_trimmed = if other_bytes.len() >= 2
            && other_bytes[0] == b'\''
            && other_bytes[other_bytes.len() - 1] == b'\''
        {
            &other_bytes[1..other_bytes.len() - 1]
        } else {
            other_bytes
        };

        // Total length: 1 (') + trimmed1 + trimmed2 + 1 (')
        let total_len = 1 + self_trimmed.len() + other_trimmed.len() + 1;

        if total_len <= SMALL_STRING_CAPACITY {
            // Result fits in small string
            let mut data = [0u8; 24];
            let mut pos = 0;

            // Opening quote
            data[pos] = b'\'';
            pos += 1;

            // First string (trimmed)
            data[pos..pos + self_trimmed.len()].copy_from_slice(self_trimmed);
            pos += self_trimmed.len();

            // Second string (trimmed)
            data[pos..pos + other_trimmed.len()].copy_from_slice(other_trimmed);
            pos += other_trimmed.len();

            // Closing quote
            data[pos] = b'\'';

            LiftString {
                data,
                len: total_len as u64,
            }
        } else {
            // Result needs large string
            let mut vec = Vec::with_capacity(total_len);

            // Opening quote
            vec.push(b'\'');

            // First string (trimmed)
            vec.extend_from_slice(self_trimmed);

            // Second string (trimmed)
            vec.extend_from_slice(other_trimmed);

            // Closing quote
            vec.push(b'\'');

            let rc_ptr = RefCounted::new(vec);
            let mut data = [0u8; 24];
            let ptr_bytes = (rc_ptr as usize).to_ne_bytes();
            data[..8].copy_from_slice(&ptr_bytes);

            LiftString {
                data,
                len: total_len as u64,
            }
        }
    }
}

// ============================================================================
// LiftString Runtime Functions (C-callable)
// ============================================================================

/// Create a new LiftString from a C string literal
#[no_mangle]
pub unsafe extern "C" fn lift_string_new(ptr: *const c_char) -> LiftString {
    if ptr.is_null() {
        return LiftString::from_str("");
    }

    let c_str = std::ffi::CStr::from_ptr(ptr);
    if let Ok(s) = c_str.to_str() {
        LiftString::from_str(s)
    } else {
        LiftString::from_str("")
    }
}

/// Clone a LiftString (copy small, retain large)
#[no_mangle]
pub unsafe extern "C" fn lift_string_clone(s: LiftString) -> LiftString {
    if !s.is_small() {
        // Large string - increment refcount
        s.retain();
    }
    // Return a copy (small strings are copied, large strings share the pointer)
    s
}

/// Increment reference count for large strings (no-op for small)
#[no_mangle]
pub unsafe extern "C" fn lift_string_retain(s: LiftString) {
    s.retain();
}

/// Decrement reference count for large strings (no-op for small)
#[no_mangle]
pub unsafe extern "C" fn lift_string_release(s: LiftString) {
    s.release();
}

/// Concatenate two strings
#[no_mangle]
pub unsafe extern "C" fn lift_string_concat(a: LiftString, b: LiftString) -> LiftString {
    a.concat(&b)
}

/// Convert LiftString to C string for output/printing
/// Returns a pointer that must be freed with free_cstr()
#[no_mangle]
pub unsafe extern "C" fn lift_string_to_cstr(s: LiftString) -> *mut c_char {
    let cstring = s.to_cstring();
    cstring.into_raw()
}

/// Free a C string created by lift_string_to_cstr
#[no_mangle]
pub unsafe extern "C" fn lift_string_free_cstr(ptr: *mut c_char) {
    if !ptr.is_null() {
        let _ = CString::from_raw(ptr);
    }
}

/// Output a LiftString
#[no_mangle]
pub unsafe extern "C" fn lift_output_lift_string(s: LiftString) {
    let bytes = s.as_bytes();
    if let Ok(str_val) = std::str::from_utf8(bytes) {
        print!("{} ", str_val);
    }
}

// ============================================================================
// Stack-friendly LiftString Functions (for easier Cranelift integration)
// ============================================================================

/// Initialize a LiftString from a C string (for stack-allocated LiftString)
/// dest: pointer to uninitialized LiftString (32 bytes)
/// src: C string to copy from
#[no_mangle]
pub unsafe extern "C" fn lift_string_init_from_cstr(dest: *mut LiftString, src: *const c_char) {
    if dest.is_null() {
        return;
    }

    let lift_str = if src.is_null() {
        LiftString::from_str("")
    } else {
        let c_str = std::ffi::CStr::from_ptr(src);
        if let Ok(s) = c_str.to_str() {
            LiftString::from_str(s)
        } else {
            LiftString::from_str("")
        }
    };

    // Write to destination
    std::ptr::write(dest, lift_str);
}

/// Concatenate two LiftStrings, writing result to dest
#[no_mangle]
pub unsafe extern "C" fn lift_string_concat_to(
    dest: *mut LiftString,
    a: *const LiftString,
    b: *const LiftString,
) {
    if dest.is_null() || a.is_null() || b.is_null() {
        return;
    }

    let a_ref = &*a;
    let b_ref = &*b;
    let result = a_ref.concat(b_ref);

    std::ptr::write(dest, result);
}

/// Copy a LiftString (for assignment)
#[no_mangle]
pub unsafe extern "C" fn lift_string_copy(dest: *mut LiftString, src: *const LiftString) {
    if dest.is_null() || src.is_null() {
        return;
    }

    let src_ref = &*src;
    let copied = if !src_ref.is_small() {
        src_ref.retain();
        *src_ref
    } else {
        *src_ref
    };

    std::ptr::write(dest, copied);
}

/// Release a LiftString (call before it goes out of scope)
#[no_mangle]
pub unsafe extern "C" fn lift_string_drop(s: *mut LiftString) {
    if !s.is_null() {
        let s_ref = &*s;
        s_ref.release();
    }
}

/// Output a LiftString from a pointer
#[no_mangle]
pub unsafe extern "C" fn lift_output_lift_string_ptr(s: *const LiftString) {
    if s.is_null() {
        return;
    }
    let s_ref = &*s;
    let bytes = s_ref.as_bytes();
    if let Ok(str_val) = std::str::from_utf8(bytes) {
        print!("{} ", str_val);
    }
}

// ============================================================================
// LiftString Method Functions
// ============================================================================

/// Helper: Get string content without quotes from LiftString
unsafe fn get_unquoted_str(s: &LiftString) -> &str {
    let bytes = s.as_bytes();
    let trimmed = if bytes.len() >= 2 && bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'' {
        &bytes[1..bytes.len() - 1]
    } else {
        bytes
    };
    std::str::from_utf8(trimmed).unwrap_or("")
}

/// Helper: Create a LiftString with quotes around content
unsafe fn create_quoted_string(content: &str) -> LiftString {
    let with_quotes = format!("'{}'", content);
    LiftString::from_str(&with_quotes)
}

/// String length (of content, not including quotes)
#[no_mangle]
pub unsafe extern "C" fn lift_string_len(s: *const LiftString) -> i64 {
    if s.is_null() {
        return 0;
    }
    let s_ref = &*s;
    let unquoted = get_unquoted_str(s_ref);
    unquoted.len() as i64
}

/// String equality
#[no_mangle]
pub unsafe extern "C" fn lift_string_eq(a: *const LiftString, b: *const LiftString) -> i8 {
    if a.is_null() || b.is_null() {
        return 0;
    }
    let a_ref = &*a;
    let b_ref = &*b;
    let a_str = get_unquoted_str(a_ref);
    let b_str = get_unquoted_str(b_ref);
    if a_str == b_str {
        1
    } else {
        0
    }
}

/// Convert to uppercase
#[no_mangle]
pub unsafe extern "C" fn lift_string_upper(dest: *mut LiftString, src: *const LiftString) {
    if dest.is_null() || src.is_null() {
        return;
    }
    let src_ref = &*src;
    let unquoted = get_unquoted_str(src_ref);
    let upper = unquoted.to_uppercase();
    let result = create_quoted_string(&upper);
    std::ptr::write(dest, result);
}

/// Convert to lowercase
#[no_mangle]
pub unsafe extern "C" fn lift_string_lower(dest: *mut LiftString, src: *const LiftString) {
    if dest.is_null() || src.is_null() {
        return;
    }
    let src_ref = &*src;
    let unquoted = get_unquoted_str(src_ref);
    let lower = unquoted.to_lowercase();
    let result = create_quoted_string(&lower);
    std::ptr::write(dest, result);
}

/// Substring
#[no_mangle]
pub unsafe extern "C" fn lift_string_substring(
    dest: *mut LiftString,
    src: *const LiftString,
    start: i64,
    end: i64,
) {
    if dest.is_null() || src.is_null() {
        return;
    }
    let src_ref = &*src;
    let unquoted = get_unquoted_str(src_ref);
    let start_idx = start.max(0) as usize;
    let end_idx = end.min(unquoted.len() as i64) as usize;

    let substring = if start_idx <= end_idx && end_idx <= unquoted.len() {
        &unquoted[start_idx..end_idx]
    } else {
        ""
    };

    let result = create_quoted_string(substring);
    std::ptr::write(dest, result);
}

/// Check if string contains substring
#[no_mangle]
pub unsafe extern "C" fn lift_string_contains(
    s: *const LiftString,
    needle: *const LiftString,
) -> i8 {
    if s.is_null() || needle.is_null() {
        return 0;
    }
    let s_ref = &*s;
    let needle_ref = &*needle;
    let s_str = get_unquoted_str(s_ref);
    let needle_str = get_unquoted_str(needle_ref);

    if s_str.contains(needle_str) {
        1
    } else {
        0
    }
}

/// Trim whitespace
#[no_mangle]
pub unsafe extern "C" fn lift_string_trim(dest: *mut LiftString, src: *const LiftString) {
    if dest.is_null() || src.is_null() {
        return;
    }
    let src_ref = &*src;
    let unquoted = get_unquoted_str(src_ref);
    let trimmed = unquoted.trim();
    let result = create_quoted_string(trimmed);
    std::ptr::write(dest, result);
}

/// Replace occurrences
#[no_mangle]
pub unsafe extern "C" fn lift_string_replace(
    dest: *mut LiftString,
    src: *const LiftString,
    old: *const LiftString,
    new: *const LiftString,
) {
    if dest.is_null() || src.is_null() || old.is_null() || new.is_null() {
        return;
    }
    let src_ref = &*src;
    let old_ref = &*old;
    let new_ref = &*new;

    let src_str = get_unquoted_str(src_ref);
    let old_str = get_unquoted_str(old_ref);
    let new_str = get_unquoted_str(new_ref);

    let replaced = src_str.replace(old_str, new_str);
    let result = create_quoted_string(&replaced);
    std::ptr::write(dest, result);
}

/// Check if string starts with prefix
#[no_mangle]
pub unsafe extern "C" fn lift_string_starts_with(
    s: *const LiftString,
    prefix: *const LiftString,
) -> i8 {
    if s.is_null() || prefix.is_null() {
        return 0;
    }
    let s_ref = &*s;
    let prefix_ref = &*prefix;
    let s_str = get_unquoted_str(s_ref);
    let prefix_str = get_unquoted_str(prefix_ref);

    if s_str.starts_with(prefix_str) {
        1
    } else {
        0
    }
}

/// Check if string ends with suffix
#[no_mangle]
pub unsafe extern "C" fn lift_string_ends_with(
    s: *const LiftString,
    suffix: *const LiftString,
) -> i8 {
    if s.is_null() || suffix.is_null() {
        return 0;
    }
    let s_ref = &*s;
    let suffix_ref = &*suffix;
    let s_str = get_unquoted_str(s_ref);
    let suffix_str = get_unquoted_str(suffix_ref);

    if s_str.ends_with(suffix_str) {
        1
    } else {
        0
    }
}

/// Check if string is empty
#[no_mangle]
pub unsafe extern "C" fn lift_string_is_empty(s: *const LiftString) -> i8 {
    if s.is_null() {
        return 1;
    }
    let s_ref = &*s;
    let unquoted = get_unquoted_str(s_ref);

    if unquoted.is_empty() {
        1
    } else {
        0
    }
}

/// Split string by delimiter (returns list of strings)
/// Note: This still needs updating when we integrate LiftString into lists
#[no_mangle]
pub unsafe extern "C" fn lift_string_split(
    s: *const LiftString,
    delimiter: *const LiftString,
) -> *mut RcList {
    if s.is_null() || delimiter.is_null() {
        return std::ptr::null_mut();
    }
    let s_ref = &*s;
    let delim_ref = &*delimiter;
    let s_str = get_unquoted_str(s_ref);
    let delim_str = get_unquoted_str(delim_ref);

    // Split and convert parts to C strings (for now - will update when lists support LiftString)
    let parts: Vec<i64> = s_str
        .split(delim_str)
        .map(|part| {
            let formatted = format!("'{}'", part);
            let c_string = CString::new(formatted).unwrap();
            c_string.into_raw() as i64
        })
        .collect();

    let list = LiftList {
        elements: parts,
        elem_type: TYPE_STR,
    };
    RefCounted::new(list)
}

// Type tags for collection elements
// These correspond to Lift DataType variants


// ==================== String Methods ====================

#[no_mangle]
pub unsafe extern "C" fn lift_str_upper(s: *const c_char) -> *mut c_char {
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
pub unsafe extern "C" fn lift_str_lower(s: *const c_char) -> *mut c_char {
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
pub unsafe extern "C" fn lift_str_substring(s: *const c_char, start: i64, end: i64) -> *mut c_char {
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
pub unsafe extern "C" fn lift_str_contains(s: *const c_char, needle: *const c_char) -> i8 {
    if s.is_null() || needle.is_null() {
        return 0;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_needle = std::ffi::CStr::from_ptr(needle);
        if let (Ok(rust_str), Ok(rust_needle)) = (c_str.to_str(), c_needle.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let needle_trimmed = rust_needle.trim_matches('\'');
            return if trimmed.contains(needle_trimmed) {
                1
            } else {
                0
            };
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn lift_str_trim(s: *const c_char) -> *mut c_char {
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
pub unsafe extern "C" fn lift_str_split(s: *const c_char, delimiter: *const c_char) -> *mut RcList {
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

            let list = LiftList {
                elements: parts,
                elem_type: TYPE_STR,
            };
            return RefCounted::new(list);
        }
    }
    std::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn lift_str_replace(
    s: *const c_char,
    old: *const c_char,
    new: *const c_char,
) -> *mut c_char {
    if s.is_null() || old.is_null() || new.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_old = std::ffi::CStr::from_ptr(old);
        let c_new = std::ffi::CStr::from_ptr(new);
        if let (Ok(rust_str), Ok(rust_old), Ok(rust_new)) =
            (c_str.to_str(), c_old.to_str(), c_new.to_str())
        {
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
pub unsafe extern "C" fn lift_str_starts_with(s: *const c_char, prefix: *const c_char) -> i8 {
    if s.is_null() || prefix.is_null() {
        return 0;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_prefix = std::ffi::CStr::from_ptr(prefix);
        if let (Ok(rust_str), Ok(rust_prefix)) = (c_str.to_str(), c_prefix.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let prefix_trimmed = rust_prefix.trim_matches('\'');
            return if trimmed.starts_with(prefix_trimmed) {
                1
            } else {
                0
            };
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn lift_str_ends_with(s: *const c_char, suffix: *const c_char) -> i8 {
    if s.is_null() || suffix.is_null() {
        return 0;
    }
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s);
        let c_suffix = std::ffi::CStr::from_ptr(suffix);
        if let (Ok(rust_str), Ok(rust_suffix)) = (c_str.to_str(), c_suffix.to_str()) {
            let trimmed = rust_str.trim_matches('\'');
            let suffix_trimmed = rust_suffix.trim_matches('\'');
            return if trimmed.ends_with(suffix_trimmed) {
                1
            } else {
                0
            };
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn lift_str_is_empty(s: *const c_char) -> i8 {
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


// ============================================================================
// Legacy String Functions (C-style strings)
// ============================================================================

pub unsafe extern "C" fn lift_str_new(ptr: *const c_char) -> *mut c_char {
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
pub unsafe extern "C" fn lift_str_concat(s1: *const c_char, s2: *const c_char) -> *mut c_char {
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
pub unsafe extern "C" fn lift_str_len(ptr: *const c_char) -> i64 {
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
pub unsafe extern "C" fn lift_str_free(ptr: *mut c_char) {
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
pub unsafe extern "C" fn lift_str_eq(s1: *const c_char, s2: *const c_char) -> i8 {
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
