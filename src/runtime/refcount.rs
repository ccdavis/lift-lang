// Reference counting infrastructure for Lift runtime
// Provides automatic memory management via reference counting

use std::sync::atomic::{AtomicUsize, Ordering};

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
            weak_count: AtomicUsize::new(1), // Always 1 until we implement weak refs
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
        (ptr as *const u8).offset(-(std::mem::offset_of!(RefCounted<T>, data) as isize))
            as *mut Self
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
            panic!(
                "assert_refcount: null pointer (expected {}): {}",
                expected, msg
            );
        }
        let actual = Self::count(ptr);
        assert_eq!(
            actual, expected,
            "Refcount mismatch at '{}': expected {}, got {}",
            msg, expected, actual
        );
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
            panic!(
                "assert_refcount_at_least: null pointer (expected >= {}): {}",
                min, msg
            );
        }
        let actual = Self::count(ptr);
        assert!(
            actual >= min,
            "Refcount too low at '{}': expected at least {}, got {}",
            msg,
            min,
            actual
        );
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
            eprintln!(
                "Elision Rate: {:.1}% (lower retains = better)",
                elision_rate
            );

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

/// Stub module for release builds
#[cfg(not(debug_assertions))]
pub mod refcount_stats {
    #[inline(always)]
    pub fn record_alloc() {}
    #[inline(always)]
    pub fn record_retain() {}
    #[inline(always)]
    pub fn record_release() {}
    #[inline(always)]
    pub fn record_free() {}
    #[inline(always)]
    pub fn report() {}
    #[inline(always)]
    pub fn reset() {}
}
