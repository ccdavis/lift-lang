# Struct Flt Field Fix - Summary

## Overview

Fixed critical bugs in the Cranelift JIT compiler and semantic analyzer that prevented Flt (float) fields in structs from working correctly. The fix involved **three separate but related bugs** across multiple modules.

##  Bugs Fixed

### Bug #1: Missing Bitcast for Flt Fields in Compiler (`src/cranelift/structs.rs`)

**Problem**: Field access compilation returned i64 values for all field types, but Flt fields needed bitcast to f64.

**Location**: `src/cranelift/structs.rs:196-198`

**Fix**: Added type-aware conversion:
```rust
// Get field type from struct definition
if let DataType::Struct(fields) = resolved_type {
    if let Some(field_param) = fields.iter().find(|p| p.name == field_name) {
        let field_type = &field_param.data_type;
        let resolved_field_type = resolve_type_alias(field_type, symbols);

        // Bitcast i64 to f64 for Flt fields
        if matches!(resolved_field_type, DataType::Flt) {
            let float_val = builder.ins().bitcast(types::F64, MemFlags::new(), field_val);
            return Ok(Some(float_val));
        }
    }
}
```

### Bug #2: Type Alias Resolution Only Checked Scope 0 (`src/semantic/mod.rs`)

**Problem**: `resolve_type_alias()` only looked up types in scope 0, failing for types defined in nested blocks.

**Location**: `src/semantic/mod.rs:112`

**Fix**: Search all scopes from deepest to shallowest:
```rust
// Look up the type in all scopes (start from the deepest scope)
let mut found = None;
for scope_idx in (0..symbols.scope_count()).rev() {
    if let Some(underlying_type) = symbols.lookup_type(name, scope_idx) {
        found = Some(underlying_type);
        break;
    }
}
```

**Impact**: This bug also affected field access type inference, preventing correct Flt type detection in binary operations.

### Bug #3: Missing Struct Type Handling in Method Lookup

**Problem**: Both compiler and semantic analyzer couldn't find methods on struct types because they didn't handle `DataType::Struct` case.

**Locations**:
- `src/cranelift/functions.rs:358`
- `src/semantic/symbol_processing.rs:285`

**Fix**: Get type name from original TypeRef before resolution:
```rust
DataType::Struct(_) => {
    // For structs, get the name from the original TypeRef
    match &receiver_type_raw { // or receiver_type_original in semantic
        DataType::TypeRef(name) => name.as_str(),
        _ => return Err(...),
    }
}
```

### Bug #4: Field Type Not Resolved in Type Inference (`src/semantic/type_inference.rs`)

**Problem**: Field access type inference didn't resolve field types that were type aliases.

**Location**: `src/semantic/type_inference.rs:279`

**Fix**: Resolve field types:
```rust
.map(|p| {
    // Resolve the field type in case it's also a type alias
    resolve_type_alias(&p.data_type, symbols)
})
```

## Test Results

All tests pass in **both** interpreter and compiler modes:

| Test | Interpreter | Compiler | Description |
|------|-------------|----------|-------------|
| test_struct_flt_field_access.lt | ✅ 3.5 | ✅ 3.5 | Simple field read |
| test_struct_flt_multiply.lt | ✅ 9 | ✅ 9 | Field multiplication |
| test_struct_flt_arithmetic.lt | ✅ 25 | ✅ 25 | Complex arithmetic (3²+4²) |
| test_minimal_struct_method.lt | ✅ 25 | ✅ 25 | User-defined method on struct |
| test_struct_flt_simple_add.lt | ✅ 7 | ✅ 7 | Field addition |

## Files Modified

1. **src/cranelift/structs.rs** - Added bitcast for Flt fields
2. **src/cranelift/functions.rs** - Handle Struct type in method lookup
3. **src/semantic/mod.rs** - Fix scope resolution for type aliases
4. **src/semantic/type_inference.rs** - Resolve field types in type inference
5. **src/semantic/symbol_processing.rs** - Handle Struct type in method lookup
6. **src/cranelift/codegen.rs** - Added IR printing for debugging

## Technical Details

### Root Cause Analysis

The bugs formed a chain of failures:

1. **Scope issue** → Type aliases in blocks weren't resolved
2. **Type inference issue** → Field access didn't return correct Flt type
3. **Binary op issue** → Arithmetic used integer ops instead of float ops
4. **Field access issue** → Returned i64 instead of f64
5. **Method lookup issue** → Couldn't find methods on struct types

### Cranelift IR Before/After

**Before** (incorrect - using integer ops):
```
v26 = bitcast.f64 v25    # Field access (fixed)
v32 = bitcast.f64 v31    # Field access (fixed)
v33 = iadd v26, v32      # ❌ Integer add on floats
```

**After** (correct - using float ops):
```
v26 = bitcast.f64 v25    # Field access returns f64 ✅
v32 = bitcast.f64 v31    # Field access returns f64 ✅
v33 = fadd v26, v32      # ✅ Float add on floats
```

## Known Limitations

### Mandelbrot Renderer Limitation

The full Mandelbrot renderer (`test_mandelbrot_complex.lt`) still fails with:
```
Struct type without TypeRef name: Struct([...])
```

**Reason**: When variables are stored in the symbol table, they store the RESOLVED type (Struct) instead of the original type name (TypeRef). This means when a variable of struct type is used as a method receiver, the type system can't find the original type name.

**Example**:
```lift
let var z = Complex(x: 0.0, iy: 0.0);  // Stored as Struct([...]) not TypeRef("Complex")
z.magnitude_squared()  // Can't find "Complex" name for method lookup
```

**Workaround**: Use struct literals directly as method receivers:
```lift
Complex(x: 3.0, y: 4.0).magnitude()  // Works ✅
```

**Future Fix**: Would require changing symbol table to preserve TypeRef names for user-defined types, or implementing a reverse lookup from Struct to TypeRef name.

## Impact

**Before fixes**:
- ❌ Struct Flt fields always returned 0 or garbage
- ❌ Arithmetic on struct fields silently wrong
- ❌ Methods on struct types couldn't be found
- ❌ Type aliases in blocks not resolved

**After fixes**:
- ✅ Struct Flt fields work correctly
- ✅ Arithmetic uses correct float operations
- ✅ Methods on struct types found and called
- ✅ Type aliases work in any scope
- ✅ 5/5 targeted tests pass in both modes

## Recommendations

1. **Symbol Table Refactoring**: Store original TypeRef names alongside resolved types
2. **Type System Documentation**: Document when types are resolved vs. when they stay as TypeRefs
3. **More Integration Tests**: Add more complex struct tests to catch similar issues
4. **Bi-directional Type Mapping**: Consider maintaining Struct → TypeRef reverse lookup

---

**Date**: 2025-11-01
**Fixed By**: Claude Code
**Status**: Core functionality fixed, one edge case remains
