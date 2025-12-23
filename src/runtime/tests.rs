mod tests {
    use crate::runtime::*;
    use crate::runtime::refcount::RefCounted;
    use crate::runtime::string::{LiftString, lift_str_concat, lift_str_len, lift_str_eq, lift_string_clone};
    use std::os::raw::c_char;

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
        use std::mem::align_of;

        // Verify RefCounted has same alignment as Arc's inner structure
        assert_eq!(align_of::<RefCounted<Vec<i64>>>(), align_of::<usize>());

        // Verify the fields are in the correct order
        let rc_ptr = RefCounted::new(vec![1, 2, 3]);
        unsafe {
            let strong = RefCounted::count(rc_ptr);
            let weak = RefCounted::weak_count(rc_ptr);

            assert_eq!(strong, 1, "Initial strong count should be 1");
            assert_eq!(
                weak, 0,
                "Initial weak count should be 0 (internal weak is hidden)"
            );

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
