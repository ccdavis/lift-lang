// Range runtime functions for Lift

use super::refcount::RefCounted;

/// Runtime representation of a range
#[repr(C)]
pub struct LiftRange {
    pub start: i64,
    pub end: i64,
}

/// Type alias for reference-counted range
pub type RcRange = RefCounted<LiftRange>;

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

