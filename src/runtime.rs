// Runtime library for Lift compiler
// These functions are called from JIT-compiled code to handle heap-allocated types

use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Reference Counting Infrastructure
// ============================================================================

/// Reference-counted wrapper for heap-allocated data
/// This provides automatic memory management via reference counting
///
/// Memory layout is ABI-compatible with Rust's Arc<T> (specifically ArcInner<T>):
/// [strong_count: AtomicUsize, weak_count: AtomicUsize, data: T]
/// This allows zero-cost interop with Rust's Arc when needed.
#[repr(C)]
pub struct RefCounted<T> {
    /// Strong reference count - how many pointers currently reference this data
    /// When this reaches 0, the data T is dropped
    strong_count: AtomicUsize,

    /// Weak reference count - for future weak reference support
    /// Currently always kept at 1 (not yet implementing weak refs)
    /// When strong_count=0 and weak_count=1, the allocation is freed
    weak_count: AtomicUsize,

    /// The actual data being reference counted
    data: T,
}

impl<T> RefCounted<T> {
    /// Create a new reference-counted value with initial count of 1
    /// Compatible with Arc::new() behavior
    pub fn new(data: T) -> *mut Self {
        #[cfg(debug_assertions)]
        refcount_stats::record_alloc();

        let rc = Box::new(RefCounted {
            strong_count: AtomicUsize::new(1),
            weak_count: AtomicUsize::new(1),  // Always 1 until we implement weak refs
            data,
        });
        Box::into_raw(rc)
    }

    /// Increment the strong reference count (called when copying a pointer)
    /// Compatible with Arc::clone() behavior
    pub unsafe fn retain(ptr: *mut Self) {
        if ptr.is_null() {
            return;
        }

        #[cfg(debug_assertions)]
        refcount_stats::record_retain();

        let rc = &*ptr;
        let old_count = rc.strong_count.fetch_add(1, Ordering::Relaxed);
        // Sanity check: if count was 0, something went very wrong
        if old_count == 0 {
            panic!("Attempted to retain a freed RefCounted object");
        }
    }

    /// Decrement the strong reference count and free if it reaches zero
    /// Returns true if the object was freed
    /// Compatible with Arc::drop() behavior
    pub unsafe fn release(ptr: *mut Self) -> bool {
        if ptr.is_null() {
            return false;
        }

        #[cfg(debug_assertions)]
        refcount_stats::record_release();

        let rc = &*ptr;
        let old_count = rc.strong_count.fetch_sub(1, Ordering::Release);

        if old_count == 1 {
            // We were the last strong reference
            // Acquire ordering ensures all previous operations are visible
            std::sync::atomic::fence(Ordering::Acquire);

            #[cfg(debug_assertions)]
            refcount_stats::record_free();

            // Drop the data (but not the allocation yet, in case of weak refs)
            // Since we don't support weak refs yet, we can free immediately
            // The weak_count is always 1, so we can safely free the entire allocation
            let _box = Box::from_raw(ptr);
            // Box will be dropped, calling T's destructor and freeing memory
            return true;
        } else if old_count == 0 {
            panic!("Strong reference count underflow");
        }
        false
    }

    /// Get a reference to the data (for reading)
    pub unsafe fn get(ptr: *const Self) -> Option<&'static T> {
        if ptr.is_null() {
            return None;
        }
        let rc = &*ptr;
        Some(&rc.data)
    }

    /// Get a mutable reference to the data (for writing)
    pub unsafe fn get_mut(ptr: *mut Self) -> Option<&'static mut T> {
        if ptr.is_null() {
            return None;
        }
        let rc = &mut *ptr;
        Some(&mut rc.data)
    }

    /// Get the current strong reference count (for debugging)
    /// Compatible with Arc::strong_count()
    pub unsafe fn count(ptr: *const Self) -> usize {
        if ptr.is_null() {
            return 0;
        }
        let rc = &*ptr;
        rc.strong_count.load(Ordering::Relaxed)
    }

    /// Get the current weak reference count (for debugging)
    /// Compatible with Arc::weak_count()
    pub unsafe fn weak_count(ptr: *const Self) -> usize {
        if ptr.is_null() {
            return 0;
        }
        let rc = &*ptr;
        // Subtract 1 because Arc's weak_count doesn't include the implicit weak ref
        rc.weak_count.load(Ordering::Relaxed).saturating_sub(1)
    }

    /// Convert a RefCounted<T> pointer to an Arc<T>
    /// This is a zero-cost conversion since the memory layouts are identical
    ///
    /// # Safety
    /// - ptr must be a valid RefCounted<T> pointer created by RefCounted::new()
    /// - The caller must ensure the RefCounted is not used after this conversion
    /// - The Arc will take ownership of the reference count
    pub unsafe fn into_arc(ptr: *mut Self) -> std::sync::Arc<T>
    where
        T: Sized,
    {
        // Since RefCounted<T> has the same layout as ArcInner<T>,
        // we can transmute the pointer directly
        std::sync::Arc::from_raw(&(*ptr).data as *const T)
    }

    /// Convert an Arc<T> to a RefCounted<T> pointer
    /// This is a zero-cost conversion since the memory layouts are identical
    ///
    /// # Safety
    /// - The Arc will be consumed and its reference count transferred to RefCounted
    /// - The returned pointer must be manually freed with release() when done
    pub unsafe fn from_arc(arc: std::sync::Arc<T>) -> *mut Self
    where
        T: Sized,
    {
        let ptr = std::sync::Arc::into_raw(arc);
        // Calculate the offset back to the start of RefCounted
        // ptr points to T, we need to go back by offset_of!(RefCounted, data)
        let rc_ptr = (ptr as *const u8).offset(
            -(std::mem::offset_of!(RefCounted<T>, data) as isize)
        ) as *mut Self;
        rc_ptr
    }

    // ==================== Debug Assertions ====================

    /// Assert that a RefCounted has a specific reference count (debug builds only)
    ///
    /// This helps catch refcount bugs during development:
    /// - Use after allocation to verify refcount=1
    /// - Use before release to ensure refcount >= 1
    /// - Use to verify ownership transfer worked correctly
    ///
    /// # Safety
    /// ptr must be a valid RefCounted pointer
    #[cfg(debug_assertions)]
    pub unsafe fn assert_refcount(ptr: *const Self, expected: usize, msg: &str) {
        if ptr.is_null() {
            panic!("assert_refcount: null pointer (expected {}): {}", expected, msg);
        }
        let actual = Self::count(ptr);
        assert_eq!(actual, expected,
            "Refcount mismatch at '{}': expected {}, got {}", msg, expected, actual);
    }

    /// Assert that a RefCounted has at least a minimum reference count (debug builds only)
    ///
    /// Useful before operations that decrement the refcount:
    /// - Before release() to ensure we're not releasing a freed object
    /// - Before untrack() to verify object is still alive
    ///
    /// # Safety
    /// ptr must be a valid RefCounted pointer
    #[cfg(debug_assertions)]
    pub unsafe fn assert_refcount_at_least(ptr: *const Self, min: usize, msg: &str) {
        if ptr.is_null() {
            panic!("assert_refcount_at_least: null pointer (expected >= {}): {}", min, msg);
        }
        let actual = Self::count(ptr);
        assert!(actual >= min,
            "Refcount too low at '{}': expected at least {}, got {}", msg, min, actual);
    }

    /// No-op in release builds
    #[cfg(not(debug_assertions))]
    #[inline(always)]
    pub unsafe fn assert_refcount(_ptr: *const Self, _expected: usize, _msg: &str) {}

    /// No-op in release builds
    #[cfg(not(debug_assertions))]
    #[inline(always)]
    pub unsafe fn assert_refcount_at_least(_ptr: *const Self, _min: usize, _msg: &str) {}
}

// Type aliases for reference-counted collections
pub type RcList = RefCounted<LiftList>;
pub type RcMap = RefCounted<LiftMap>;
pub type RcStruct = RefCounted<LiftStruct>;
pub type RcRange = RefCounted<LiftRange>;

// ============================================================================
// Reference Counting Statistics (Debug Mode Only)
// ============================================================================

/// Runtime statistics for reference counting operations
/// Only compiled in debug builds for performance profiling
#[cfg(debug_assertions)]
pub mod refcount_stats {
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Total number of RefCounted allocations
    pub static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

    /// Total number of retain() calls (should be low due to elision)
    pub static RETAINS: AtomicUsize = AtomicUsize::new(0);

    /// Total number of release() calls
    pub static RELEASES: AtomicUsize = AtomicUsize::new(0);

    /// Number of objects actually freed (release with refcount=1)
    pub static FREES: AtomicUsize = AtomicUsize::new(0);

    /// Record an allocation
    #[inline]
    pub fn record_alloc() {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a retain operation
    #[inline]
    pub fn record_retain() {
        RETAINS.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a release operation
    #[inline]
    pub fn record_release() {
        RELEASES.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an actual free (when refcount reaches 0)
    #[inline]
    pub fn record_free() {
        FREES.fetch_add(1, Ordering::Relaxed);
    }

    /// Print statistics report
    pub fn report() {
        let allocs = ALLOCATIONS.load(Ordering::Relaxed);
        let retains = RETAINS.load(Ordering::Relaxed);
        let releases = RELEASES.load(Ordering::Relaxed);
        let frees = FREES.load(Ordering::Relaxed);

        eprintln!("\n=== Reference Counting Statistics ===");
        eprintln!("Allocations:  {}", allocs);
        eprintln!("Retains:      {}", retains);
        eprintln!("Releases:     {}", releases);
        eprintln!("Frees:        {}", frees);

        if allocs > 0 {
            let elision_rate = 100.0 * (1.0 - retains as f64 / allocs as f64);
            eprintln!("Elision Rate: {:.1}% (lower retains = better)", elision_rate);

            let leak_count = allocs - frees;
            if leak_count > 0 {
                eprintln!("WARNING: {} objects not freed (possible leak)", leak_count);
            } else {
                eprintln!("Memory: All objects freed correctly ✓");
            }
        }
        eprintln!("=====================================\n");
    }

    /// Reset all statistics (useful for benchmarking)
    pub fn reset() {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        RETAINS.store(0, Ordering::Relaxed);
        RELEASES.store(0, Ordering::Relaxed);
        FREES.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// Small String Optimization (SSO)
// ============================================================================

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
    data: [u8; 24],

    /// String length in bytes:
    /// - If len <= 23: this is a small string, data is inline
    /// - If len > 23: this is a large string, data[0..8] contains heap pointer
    len: u64,
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
pub unsafe extern "C" fn lift_string_init_from_cstr(
    dest: *mut LiftString,
    src: *const c_char,
) {
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
pub unsafe extern "C" fn lift_string_contains(s: *const LiftString, needle: *const LiftString) -> i8 {
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
pub const TYPE_INT: i8 = 0;
pub const TYPE_FLT: i8 = 1;
pub const TYPE_BOOL: i8 = 2;
pub const TYPE_STR: i8 = 3;
pub const TYPE_LIST: i8 = 4;
pub const TYPE_MAP: i8 = 5;
pub const TYPE_RANGE: i8 = 6;
pub const TYPE_STRUCT: i8 = 7;

// ============================================================================
// Output Functions
// ============================================================================

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

// ============================================================================
// String Functions
// ============================================================================

#[no_mangle]
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

// ============================================================================
// Helper Functions for Testing
// ============================================================================

/// Create a Lift string from a Rust string (for testing)
pub unsafe fn make_lift_string(s: &str) -> *mut c_char {
    let with_quotes = format!("'{}'", s);
    if let Ok(c_str) = CString::new(with_quotes) {
        c_str.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Free a Lift string (for testing)
pub unsafe fn free_lift_string(ptr: *mut c_char) {
    lift_str_free(ptr);
}

// ============================================================================
// List Functions (for integers)
// ============================================================================

/// Runtime representation of a list
#[repr(C)]
pub struct LiftList {
    pub elements: Vec<i64>,
    pub elem_type: i8, // Type tag for elements (TYPE_INT, TYPE_STR, etc.)
}

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

/// Free a list (deprecated - use lift_list_release instead)
#[no_mangle]
pub unsafe extern "C" fn lift_list_free(list: *mut RcList) {
    lift_list_release(list);
}

// ============================================================================
// Map Functions (integer keys to integer values)
// ============================================================================

use std::collections::HashMap;

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
pub unsafe extern "C" fn lift_range_new(start: i64, end: i64) -> *mut RcRange {
    let range = LiftRange { start, end };
    RefCounted::new(range)
}

/// Increment reference count for a range
#[no_mangle]
pub unsafe extern "C" fn lift_range_retain(range: *mut RcRange) {
    RefCounted::retain(range);
}

/// Decrement reference count for a range and free if it reaches zero
#[no_mangle]
pub unsafe extern "C" fn lift_range_release(range: *mut RcRange) {
    RefCounted::release(range);
}

/// Get the start of a range
#[no_mangle]
pub unsafe extern "C" fn lift_range_start(range: *const RcRange) -> i64 {
    if range.is_null() {
        return 0;
    }
    unsafe {
        if let Some(range_ref) = RefCounted::get(range) {
            range_ref.start
        } else {
            0
        }
    }
}

/// Get the end of a range
#[no_mangle]
pub unsafe extern "C" fn lift_range_end(range: *const RcRange) -> i64 {
    if range.is_null() {
        return 0;
    }
    unsafe {
        if let Some(range_ref) = RefCounted::get(range) {
            range_ref.end
        } else {
            0
        }
    }
}

/// Free a range (deprecated - use lift_range_release instead)
#[no_mangle]
pub unsafe extern "C" fn lift_range_free(range: *mut RcRange) {
    lift_range_release(range);
}

/// Output a range
#[no_mangle]
pub unsafe extern "C" fn lift_output_range(range: *const RcRange) {
    if range.is_null() {
        print!("null ");
        return;
    }
    unsafe {
        if let Some(range_ref) = RefCounted::get(range) {
            print!("{}..{} ", range_ref.start, range_ref.end);
        } else {
            print!("null ");
        }
    }
}

// ============================================================================
// Struct Functions
// ============================================================================

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
unsafe fn format_struct_inline(ptr: *const RcStruct) {
    if ptr.is_null() {
        print!("{{}}");
        return;
    }
    if let Some(s) = RefCounted::get(ptr) {
        print!("{} {{", s.type_name);

        // Sort fields by name for consistent output
        let mut field_names: Vec<&String> = s.fields.keys().collect();
        field_names.sort();

        for (i, field_name) in field_names.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            let field_value = &s.fields[*field_name];
            print!("{}:", field_name);
            format_value_inline(field_value.value, field_value.type_tag);
        }
        print!("}}");
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
                            if !compare_values_for_equality(*val1, m1.value_type, *val2, m2.value_type)
                            {
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
pub unsafe extern "C" fn lift_str_split(
    s: *const c_char,
    delimiter: *const c_char,
) -> *mut RcList {
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
pub unsafe extern "C" fn lift_list_slice(
    list: *const RcList,
    start: i64,
    end: i64,
) -> *mut RcList {
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

#[no_mangle]
pub unsafe extern "C" fn lift_list_join(
    list: *const RcList,
    separator: *const c_char,
) -> *mut c_char {
    if list.is_null() || separator.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        if let Some(list_ref) = RefCounted::get(list) {
            let c_sep = std::ffi::CStr::from_ptr(separator);
            if let Ok(rust_sep) = c_sep.to_str() {
                let sep_trimmed = rust_sep.trim_matches('\'');

                // Convert i64 elements (which are string pointers) to strings
                let strings: Vec<String> = list_ref
                    .elements
                    .iter()
                    .map(|&elem| {
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
                    })
                    .collect();

                let joined = strings.join(sep_trimmed);
                let result = format!("'{}'", joined);
                if let Ok(c_result) = CString::new(result) {
                    return c_result.into_raw();
                }
            }
        }
        std::ptr::null_mut()
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

            free_lift_string(s1 as *mut c_char);
            free_lift_string(s2 as *mut c_char);
            free_lift_string(result);
        }
    }

    #[test]
    fn test_string_length() {
        unsafe {
            let s = make_lift_string("Hello");
            let len = lift_str_len(s);
            assert_eq!(len, 5);
            free_lift_string(s as *mut c_char);
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

            free_lift_string(s1 as *mut c_char);
            free_lift_string(s2 as *mut c_char);
            free_lift_string(s3 as *mut c_char);
        }
    }

    // =============================================================================
    // LiftString SSO Tests
    // =============================================================================

    #[test]
    fn test_lift_string_small() {
        // Test small string (≤ 23 bytes) stored inline
        let s = LiftString::from_str("Hello");
        assert!(s.is_small());
        assert_eq!(s.len, 5);

        unsafe {
            let bytes = s.as_bytes();
            assert_eq!(bytes, b"Hello");
        }
    }

    #[test]
    fn test_lift_string_small_max() {
        // Test maximum size small string (23 bytes)
        let text = "12345678901234567890123"; // exactly 23 bytes
        let s = LiftString::from_str(text);
        assert!(s.is_small());
        assert_eq!(s.len, 23);

        unsafe {
            let bytes = s.as_bytes();
            assert_eq!(bytes, text.as_bytes());
        }
    }

    #[test]
    fn test_lift_string_large() {
        // Test large string (> 23 bytes) heap-allocated
        let text = "This is a longer string that exceeds 23 bytes";
        let s = LiftString::from_str(text);
        assert!(!s.is_small());
        assert_eq!(s.len as usize, text.len());

        unsafe {
            let bytes = s.as_bytes();
            assert_eq!(bytes, text.as_bytes());

            // Clean up
            s.release();
        }
    }

    #[test]
    fn test_lift_string_concat_small_small() {
        // Concatenate two small strings resulting in small string
        let s1 = LiftString::from_str("Hello");
        let s2 = LiftString::from_str(" Wor");

        unsafe {
            let result = s1.concat(&s2);
            assert!(result.is_small()); // 11 bytes total (includes quotes)
            assert_eq!(result.len, 11); // 'Hello Wor' with quotes
            assert_eq!(result.as_bytes(), b"'Hello Wor'");
        }
    }

    #[test]
    fn test_lift_string_concat_small_large() {
        // Concatenate small + small = large string
        let s1 = LiftString::from_str("Hello ");
        let s2 = LiftString::from_str("World! This is long");

        unsafe {
            let result = s1.concat(&s2);
            assert!(!result.is_small()); // 27 bytes total (includes quotes)
            assert_eq!(result.len, 27); // 'Hello World! This is long' with quotes
            assert_eq!(result.as_bytes(), b"'Hello World! This is long'");

            // Clean up
            result.release();
        }
    }

    #[test]
    fn test_lift_string_concat_large_large() {
        // Concatenate two large strings
        let s1 = LiftString::from_str("This is the first long string!!");
        let s2 = LiftString::from_str("This is the second long string!!");

        unsafe {
            let result = s1.concat(&s2);
            assert!(!result.is_small());
            let expected = "'This is the first long string!!This is the second long string!!'";
            assert_eq!(result.len as usize, expected.len());
            assert_eq!(result.as_bytes(), expected.as_bytes());

            // Clean up
            s1.release();
            s2.release();
            result.release();
        }
    }

    #[test]
    fn test_lift_string_retain_release() {
        // Test reference counting for large strings
        let text = "This is a long string for testing refcounting";
        let s = LiftString::from_str(text);
        assert!(!s.is_small());

        unsafe {
            // Initial refcount should be 1
            let ptr_bytes: [u8; 8] = s.data[..8].try_into().unwrap();
            let ptr = usize::from_ne_bytes(ptr_bytes) as *mut RefCounted<Vec<u8>>;
            assert_eq!(RefCounted::count(ptr), 1);

            // Retain - refcount should be 2
            s.retain();
            assert_eq!(RefCounted::count(ptr), 2);

            // Release - refcount should be 1
            s.release();
            assert_eq!(RefCounted::count(ptr), 1);

            // Final release
            s.release();
            // Don't check count after final release - memory is freed
        }
    }

    #[test]
    fn test_lift_string_clone() {
        // Test cloning small string
        let small = LiftString::from_str("Hi");
        unsafe {
            let cloned = lift_string_clone(small);
            assert!(cloned.is_small());
            assert_eq!(cloned.as_bytes(), b"Hi");
        }

        // Test cloning large string (should increment refcount)
        let large = LiftString::from_str("This is a very long string for testing");
        unsafe {
            let ptr_bytes: [u8; 8] = large.data[..8].try_into().unwrap();
            let ptr = usize::from_ne_bytes(ptr_bytes) as *mut RefCounted<Vec<u8>>;

            let before_count = RefCounted::count(ptr);
            let cloned = lift_string_clone(large);
            let after_count = RefCounted::count(ptr);

            assert_eq!(after_count, before_count + 1);
            assert_eq!(cloned.as_bytes(), large.as_bytes());

            // Clean up
            large.release();
            cloned.release();
        }
    }

    #[test]
    fn test_lift_string_size() {
        // Verify LiftString is exactly 32 bytes
        use std::mem::size_of;
        assert_eq!(size_of::<LiftString>(), 32);
    }

    // =============================================================================
    // Arc<T> Compatibility Tests
    // =============================================================================

    #[test]
    fn test_arc_compatibility_layout() {
        use std::mem::{size_of, align_of};
        use std::sync::Arc;

        // Verify RefCounted has same alignment as Arc's inner structure
        assert_eq!(align_of::<RefCounted<Vec<i64>>>(), align_of::<usize>());

        // Verify the fields are in the correct order
        let rc_ptr = RefCounted::new(vec![1, 2, 3]);
        unsafe {
            let strong = RefCounted::count(rc_ptr);
            let weak = RefCounted::weak_count(rc_ptr);

            assert_eq!(strong, 1, "Initial strong count should be 1");
            assert_eq!(weak, 0, "Initial weak count should be 0 (internal weak is hidden)");

            RefCounted::release(rc_ptr);
        }
    }

    #[test]
    fn test_refcounted_to_arc_conversion() {
        use std::sync::Arc;

        unsafe {
            // Create a RefCounted
            let rc_ptr = RefCounted::new(vec![1, 2, 3]);
            assert_eq!(RefCounted::count(rc_ptr), 1);

            // Convert to Arc
            let arc: Arc<Vec<i64>> = RefCounted::into_arc(rc_ptr);
            assert_eq!(Arc::strong_count(&arc), 1);
            assert_eq!(*arc, vec![1, 2, 3]);

            // Clone the Arc
            let arc2 = Arc::clone(&arc);
            assert_eq!(Arc::strong_count(&arc), 2);
            assert_eq!(Arc::strong_count(&arc2), 2);

            // Drop both Arcs - memory should be freed automatically
            drop(arc);
            drop(arc2);
        }
    }

    #[test]
    fn test_arc_to_refcounted_conversion() {
        use std::sync::Arc;

        unsafe {
            // Create an Arc
            let arc = Arc::new(vec![10, 20, 30]);
            assert_eq!(Arc::strong_count(&arc), 1);

            // Convert to RefCounted
            let rc_ptr = RefCounted::from_arc(arc);
            assert_eq!(RefCounted::count(rc_ptr), 1);

            // Verify data is intact
            let data = RefCounted::get(rc_ptr).unwrap();
            assert_eq!(*data, vec![10, 20, 30]);

            // Retain and release
            RefCounted::retain(rc_ptr);
            assert_eq!(RefCounted::count(rc_ptr), 2);

            RefCounted::release(rc_ptr);
            assert_eq!(RefCounted::count(rc_ptr), 1);

            // Final release should free
            let freed = RefCounted::release(rc_ptr);
            assert!(freed, "Should have freed the memory");
        }
    }

    #[test]
    fn test_arc_refcounted_roundtrip() {
        use std::sync::Arc;

        unsafe {
            // Start with RefCounted
            let rc_ptr = RefCounted::new("Hello, Arc!".to_string());
            assert_eq!(RefCounted::count(rc_ptr), 1);

            // Convert to Arc
            let arc = RefCounted::into_arc(rc_ptr);
            assert_eq!(Arc::strong_count(&arc), 1);
            assert_eq!(*arc, "Hello, Arc!");

            // Convert back to RefCounted
            let rc_ptr2 = RefCounted::from_arc(arc);
            assert_eq!(RefCounted::count(rc_ptr2), 1);

            // Verify data
            let data = RefCounted::get(rc_ptr2).unwrap();
            assert_eq!(*data, "Hello, Arc!");

            // Clean up
            RefCounted::release(rc_ptr2);
        }
    }

    #[test]
    fn test_arc_compatibility_with_collections() {
        use std::sync::Arc;

        unsafe {
            // Test with LiftList (Vec<i64>)
            let list = vec![1, 2, 3, 4, 5];
            let rc_ptr = RefCounted::new(list);

            let arc = RefCounted::into_arc(rc_ptr);
            assert_eq!(Arc::strong_count(&arc), 1);

            // Verify we can use Arc normally
            let arc2 = Arc::clone(&arc);
            assert_eq!(Arc::strong_count(&arc), 2);
            assert_eq!(arc.len(), 5);
            assert_eq!(arc[2], 3);

            drop(arc);
            drop(arc2);
        }
    }

    #[test]
    fn test_refcounted_weak_count_compatibility() {
        use std::sync::Arc;

        unsafe {
            let rc_ptr = RefCounted::new(42);

            // Convert to Arc
            let arc = RefCounted::into_arc(rc_ptr);

            // Create a weak reference
            let weak = Arc::downgrade(&arc);

            assert_eq!(Arc::strong_count(&arc), 1);
            assert_eq!(Arc::weak_count(&arc), 1);

            // Upgrade the weak ref
            let arc2 = weak.upgrade().unwrap();
            assert_eq!(Arc::strong_count(&arc), 2);

            drop(arc);
            drop(arc2);
            drop(weak);
        }
    }
}
