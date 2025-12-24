# Struct Method Compilation Bug Report

## Executive Summary

Investigation into compiling a Mandelbrot renderer with struct-based complex numbers revealed a critical bug in the Cranelift JIT compiler's handling of Flt (floating-point) fields in structs. Struct methods work perfectly in the interpreter but fail in the compiler due to missing type conversions.

## Test Files Created

All test files are in `/tests/` directory:

1. **test_mandelbrot_complex.lt** - Original comprehensive Mandelbrot renderer
   - Uses `Complex` struct with `Flt` fields
   - Defines methods: `add()`, `multiply()`, `magnitude_squared()`
   - Demonstrates real-world usage of struct methods

2. **test_minimal_struct_method.lt** - Minimal reproduction of verifier error
   - Single `Point` struct with `magnitude()` method
   - Shows Cranelift verifier errors

3. **test_struct_flt_field_access.lt** - Tests simple field access
   - ✅ PASSES in both interpreter and compiler
   - Simple read of Flt field works correctly

4. **test_struct_flt_arithmetic.lt** - Tests arithmetic on Flt fields
   - ✅ PASSES in interpreter (returns 25)
   - ❌ FAILS in compiler (returns 0 instead of 25)
   - Reveals incorrect arithmetic results

5. **test_struct_flt_multiply.lt** - Tests single multiplication
   - ✅ PASSES in interpreter (returns 9)
   - ❌ FAILS in compiler (returns 0)
   - Isolates the multiplication bug

## Bug Details

### Root Cause

**Location**: `/home/ccd/lift-lang/src/cranelift/structs.rs:196-198`

```rust
// TODO: May need type conversion based on field type (e.g., bitcast for floats)
// For now, return as-is (works for Int, Bool, and pointer types)
Ok(Some(field_val))
```

The `compile_field_access()` function:
1. Calls runtime function `lift_struct_get_field()` which returns `i64`
2. Returns this value directly without type conversion
3. **BUG**: Flt (f64) fields are stored as bit patterns in i64, but need `bitcast` to f64

### Symptoms

1. **Cranelift Verifier Errors** (in methods)
   ```
   Compilation error: Failed to define function Point.magnitude: Compilation error: Verifier errors
   ```
   - Function signature declares `f64` return type
   - But arithmetic operations use `imul`/`iadd` (integer) instead of `fmul`/`fadd` (float)
   - Returns `i64` value when `f64` expected

2. **Incorrect Results** (in direct field access with arithmetic)
   ```
   Input:  3.0 * 3.0
   Expected: 9.0
   Actual:   0
   ```
   - Field access returns i64 bit pattern
   - Arithmetic treats it as integer
   - Result is garbage (often 0)

### Cranelift IR Analysis

From `test_minimal_struct_method.lt`:

```
function u0:0(i64) -> f64 system_v
...
block0(v0: i64):
    v5 = call fn40(v1, v4)   // Gets field 'x' as i64
    v10 = call fn40(v6, v9)  // Gets field 'x' as i64 again
    v11 = imul v5, v10       // ❌ Integer multiply (should be fmul)
    v16 = call fn40(v12, v15) // Gets field 'y' as i64
    v21 = call fn40(v17, v20) // Gets field 'y' as i64 again
    v22 = imul v16, v21      // ❌ Integer multiply (should be fmul)
    v23 = iadd v11, v22      // ❌ Integer add (should be fadd)
    return v23               // ❌ Returns i64 when f64 expected
}
```

## Comparison: Interpreter vs Compiler

| Feature | Interpreter | Compiler |
|---------|------------|----------|
| Struct definition | ✅ Works | ✅ Works |
| Struct creation | ✅ Works | ✅ Works |
| Flt field access (simple) | ✅ Works | ✅ Works |
| Flt field arithmetic | ✅ Works (25) | ❌ Wrong result (0) |
| Struct methods | ✅ Works | ❌ Verifier errors |

## The Fix

### Required Changes

**File**: `src/cranelift/structs.rs` function `compile_field_access()` (around line 196)

**Current Code**:
```rust
let inst = builder.ins().call(*func_ref, &[struct_val, field_name_ptr]);
let field_val = builder.inst_results(inst)[0];

// TODO: May need type conversion based on field type (e.g., bitcast for floats)
// For now, return as-is (works for Int, Bool, and pointer types)
Ok(Some(field_val))
```

**Proposed Fix**:
```rust
let inst = builder.ins().call(*func_ref, &[struct_val, field_name_ptr]);
let field_val = builder.inst_results(inst)[0];

// Type conversion based on field type
// Get the type of the field from the struct definition
let expr_type = determine_type_with_symbols(expr, symbols, 0)
    .ok_or_else(|| "Cannot determine struct type for field access".to_string())?;

let resolved_type = resolve_type_alias(&expr_type, symbols);

if let DataType::Struct(fields) = resolved_type {
    // Find the field in the struct definition
    if let Some(field_param) = fields.iter().find(|p| p.name == field_name) {
        let field_type = &field_param.data_type;

        // Bitcast i64 to f64 for Flt fields
        if matches!(field_type, DataType::Flt) {
            let float_val = builder.ins().bitcast(types::F64, field_val);
            return Ok(Some(float_val));
        }
        // i64 values can be used as-is for Int, Bool
        // Pointer types also work as-is
    }
}

Ok(Some(field_val))
```

### Additional Considerations

1. **Field Assignment**: The `compile_field_assign()` function (line 201+) may need similar fixes for storing Flt values
2. **Testing**: After fix, all test files should pass in both modes
3. **Performance**: Bitcast is a zero-cost operation at runtime (just type reinterpretation)

## Testing After Fix

Run these commands to verify the fix:

```bash
# Should all pass without errors
cargo run -- --compile tests/test_struct_flt_field_access.lt
cargo run -- --compile tests/test_struct_flt_multiply.lt
cargo run -- --compile tests/test_struct_flt_arithmetic.lt
cargo run -- --compile tests/test_minimal_struct_method.lt
cargo run -- --compile tests/test_mandelbrot_complex.lt

# Verify correct output
cargo run -- --compile tests/test_struct_flt_multiply.lt  # Should output: 9
cargo run -- --compile tests/test_struct_flt_arithmetic.lt  # Should output: 25
```

## Impact

**Severity**: HIGH
- Blocks all compiled code using Flt fields in structs
- Silently produces wrong results (doesn't always error)
- Critical for scientific/mathematical applications

**Workaround**: Use interpreter mode for struct-heavy code until fixed

## Related Files

- `src/cranelift/structs.rs` - Main bug location
- `src/runtime.rs` - Runtime functions `lift_struct_get_field()` etc.
- `src/semantic/type_inference.rs` - Type inference for field access
- `tests/test_struct_methods.lt` - Existing test that also fails

## Investigation Notes

This bug was discovered while implementing a Mandelbrot renderer using:
- `Complex` struct with `Flt` fields for real and imaginary parts
- Methods for complex arithmetic (`add`, `multiply`, `magnitude_squared`)
- Iterative rendering with nested loops

The investigation followed this path:
1. Started with comprehensive Mandelbrot renderer → Type check errors
2. Reduced to minimal method example → Verifier errors
3. Reduced to field arithmetic → Wrong results
4. Reduced to single multiplication → Isolated the bug
5. Examined Cranelift IR → Confirmed type mismatch
6. Found TODO comment → Identified exact fix location

---

**Date**: 2025-11-01
**Investigator**: Claude Code
**Status**: Bug identified, fix proposed, awaiting implementation
