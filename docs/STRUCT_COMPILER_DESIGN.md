# Struct Compiler Support - Design Document

## Overview

This document outlines the design for adding struct support to the Cranelift JIT compiler. Structs will follow the same heap-allocated pattern as List and Map.

## Architecture Pattern

### Current Pattern (List/Map/Range)
1. **Runtime representation**: C-compatible struct with `#[repr(C)]`
2. **Allocation**: `Box::new()` → `Box::into_raw()` → return pointer as `i64`
3. **Runtime functions**: Exported with `#[no_mangle]` and `extern "C"`
4. **Registration**: Symbols registered in `JITCompiler::new()`
5. **Codegen**: Call runtime functions via FFI from compiled code

## Struct Design

### 1. Runtime Representation (`src/runtime.rs`)

```rust
// Add new type constant
pub const TYPE_STRUCT: i8 = 7;

// Runtime struct representation
#[repr(C)]
pub struct LiftStruct {
    pub type_name: String,
    pub fields: HashMap<String, StructFieldValue>,
}

// Field value with type information
#[derive(Debug, Clone)]
struct StructFieldValue {
    type_tag: i8,
    value: i64,
}
```

### 2. Runtime Functions

#### Core Operations
- `lift_struct_new(type_name: *const c_char, field_count: i64) -> *mut LiftStruct`
  - Create new empty struct with type name
  - Pre-allocate HashMap with capacity

- `lift_struct_set_field(s: *mut LiftStruct, name: *const c_char, type_tag: i8, val: i64)`
  - Set field value with type information
  - Called during struct literal creation

- `lift_struct_get_field(s: *const LiftStruct, name: *const c_char) -> i64`
  - Get field value by name
  - Returns 0 if field not found (runtime error handling)

- `lift_struct_get_field_type(s: *const LiftStruct, name: *const c_char) -> i8`
  - Get field type tag
  - Used for type checking/casting

#### Comparison
- `lift_struct_eq(s1: *const LiftStruct, s2: *const LiftStruct) -> i8`
  - Structural equality comparison
  - Check type names match
  - Recursively compare all fields
  - Return 1 (true) or 0 (false)

#### Output
- `lift_output_struct(s: *const LiftStruct)`
  - Pretty-print struct
  - Format: `TypeName { field1: value1, field2: value2 }`
  - Sort fields alphabetically for consistency
  - Handle nested structs recursively

#### Memory Management
- `lift_struct_free(s: *mut LiftStruct)`
  - Free struct and all owned field values
  - Drop HashMap

### 3. Codegen Changes (`src/codegen.rs`)

#### Struct Literal (`Expr::StructLiteral`)
```rust
// Pseudo-code:
1. Get type name string
2. Call lift_struct_new(type_name, field_count)
3. For each field:
   a. Compile field value expression
   b. Get field type tag
   c. Call lift_struct_set_field(struct_ptr, field_name, type_tag, value)
4. Return struct pointer (as i64)
```

#### Field Access (`Expr::FieldAccess`)
```rust
// Pseudo-code:
1. Compile expr to get struct pointer
2. Call lift_struct_get_field(struct_ptr, field_name)
3. Return field value (may need type conversion)
```

#### Field Assignment (`Expr::FieldAssign`)
```rust
// Pseudo-code:
1. Compile expr to get struct pointer (from variable)
2. Compile value expression
3. Get field type tag
4. Call lift_struct_set_field(struct_ptr, field_name, type_tag, new_value)
5. Return Unit
```

#### Struct Comparison (in binary operators)
```rust
// For Operator::Eq and Operator::Neq:
1. Compile left expr (struct pointer)
2. Compile right expr (struct pointer)
3. Call lift_struct_eq(left_ptr, right_ptr)
4. For Neq: negate result
5. Return boolean (as i8)
```

### 4. Compiler Registration (`src/compiler.rs`)

Add to `JITCompiler::new()`:
```rust
builder.symbol("lift_struct_new", runtime::lift_struct_new as *const u8);
builder.symbol("lift_struct_set_field", runtime::lift_struct_set_field as *const u8);
builder.symbol("lift_struct_get_field", runtime::lift_struct_get_field as *const u8);
builder.symbol("lift_struct_get_field_type", runtime::lift_struct_get_field_type as *const u8);
builder.symbol("lift_struct_eq", runtime::lift_struct_eq as *const u8);
builder.symbol("lift_output_struct", runtime::lift_output_struct as *const u8);
builder.symbol("lift_struct_free", runtime::lift_struct_free as *const u8);
```

### 5. Type Handling

Need to add TYPE_STRUCT handling to:
- `format_value_inline()` - for nested struct display
- Type tag conversion functions
- Collection element type handling (List of structs, Map values as structs)

## Implementation Order

1. ✅ **Phase 0**: Design document (this file)
2. **Phase 1**: Runtime struct representation
   - Add LiftStruct type
   - Add TYPE_STRUCT constant
   - Add basic struct creation/destruction

3. **Phase 2**: Runtime field operations
   - Implement set_field and get_field
   - Add field type tracking

4. **Phase 3**: Runtime output
   - Implement lift_output_struct
   - Handle nested structs
   - Sort fields for display

5. **Phase 4**: Runtime comparison
   - Implement lift_struct_eq
   - Recursive comparison for nested structs

6. **Phase 5**: Compiler registration
   - Register all runtime functions
   - Update declare_runtime_functions in codegen

7. **Phase 6**: Codegen - struct literals
   - Compile StructLiteral expression
   - Handle field initialization

8. **Phase 7**: Codegen - field access
   - Compile FieldAccess expression
   - Type conversion if needed

9. **Phase 8**: Codegen - field mutation
   - Compile FieldAssign expression
   - Handle mutable variables

10. **Phase 9**: Codegen - comparison
    - Add struct comparison to binary operators

11. **Phase 10**: Testing
    - Unit tests for each operation
    - Integration tests with .lt files
    - Test nested structs
    - Test struct methods (if needed)

## Test Strategy

### Unit Tests (in `src/compiler.rs`)
- `test_compile_struct_literal` - Basic struct creation
- `test_compile_struct_field_access` - Read field values
- `test_compile_struct_field_mutation` - Modify fields
- `test_compile_struct_comparison_equal` - Equality
- `test_compile_struct_comparison_not_equal` - Inequality
- `test_compile_nested_struct` - Struct containing struct
- `test_compile_struct_output` - Pretty-printing

### Integration Tests (`.lt` files)
- Reuse interpreter tests with `--compile` flag:
  - `tests/test_struct_definition.lt`
  - `tests/test_struct_field_access.lt`
  - `tests/test_field_mutation.lt`
  - `tests/test_struct_comparison.lt`

## Challenges and Considerations

### 1. Field Name Strings
- Need to convert Rust String to C string for FFI
- Consider string interning or caching

### 2. Type Tags for Struct Fields
- Each field needs a type tag (TYPE_INT, TYPE_FLT, etc.)
- Nested structs: field value is pointer, type tag is TYPE_STRUCT

### 3. Memory Management
- Structs are heap-allocated
- No GC - acceptable for short programs
- Fields that are pointers (strings, nested structs) need careful tracking

### 4. Method Calls on Structs
- May already work via existing UFCS mechanism
- Self parameter passes struct pointer
- Needs testing

### 5. Struct Methods Definition
- Function definitions with receiver_type
- May need codegen support for method registration

### 6. Variable Storage
- Struct variables store pointer (i64)
- Assignment copies pointer (not deep copy)
- Mutation modifies the heap object

## Success Criteria

- All interpreter struct tests pass with `--compile` flag
- Performance: struct operations run at native speed
- Memory safety: no leaks in test suite
- Compatibility: compiled and interpreted results match

## Future Enhancements (Post-Initial Implementation)

- Struct methods in compiled code
- Optimization: inline small structs on stack
- Copy-on-write for immutable struct semantics
- Better error messages for field access failures
- Struct pattern matching (when interpreter supports it)

---

**Status**: Design Complete - Ready for Implementation
**Date**: 2025-10-11
**Estimated Effort**: ~2-3 hours (10 phases)
