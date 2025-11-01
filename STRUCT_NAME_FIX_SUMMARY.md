# Struct Name Preservation Fix - Complete Summary

## Overview

Fixed the critical bug where struct types lost their type names, preventing method lookups on struct variables. The solution was to add a `name` field to the `Struct` variant of the `DataType` enum, allowing structs to carry their type name.

## Problem

When variables of struct type were stored in the symbol table, they were stored as `Struct([Param...])` without the original type name. This caused method lookup failures:

```lift
type Complex = struct (x: Flt, iy: Flt);
let var z = Complex(x: 0.0, iy: 0.0);  // Stored as Struct([...])
z.magnitude()  // ❌ Error: Can't find type name for method lookup
```

## Solution

Modified `DataType::Struct` from tuple variant to struct variant with name field:

```rust
// Before
Struct(Vec<Param>)

// After
Struct {
    name: String,
    fields: Vec<Param>,
}
```

## Changes Made

### 1. Core Type Definition (`src/syntax.rs`)

```rust
pub enum DataType {
    ...
    Struct {
        name: String,      // NEW: Type name
        fields: Vec<Param>,
    },
    ...
}
```

### 2. Grammar Update (`src/grammar.lalrpop`)

```rust
"type" <i:ident> "=" "struct" "(" <m:CommaSeparated<Param>> ")" =>
    Expr::DefineType{
        type_name: i.clone(),
        definition: DataType::Struct { name: i, fields: m },  // Store name
        index: (0,0)
    },
```

### 3. Pattern Matches (88 locations updated)

Updated all pattern matches from:
```rust
DataType::Struct(fields)          → DataType::Struct { fields, .. }
DataType::Struct(params)          → DataType::Struct { fields: params, .. }
DataType::Struct(_)               → DataType::Struct { name: _, fields: _ }
```

**Files updated**:
- `src/compile_types.rs`
- `src/cranelift/types.rs`
- `src/cranelift/expressions.rs`
- `src/cranelift/functions.rs`
- `src/cranelift/structs.rs`
- `src/semantic/mod.rs`
- `src/semantic/symbol_processing.rs`
- `src/semantic/type_inference.rs`
- `src/semantic/typecheck.rs`

### 4. Method Lookup Updates

**Compiler** (`src/cranelift/functions.rs`):
```rust
DataType::Struct { name, .. } => {
    // Structs now carry their type name
    name.as_str()
}
```

**Semantic Analyzer** (`src/semantic/symbol_processing.rs`):
```rust
DataType::Struct { name, .. } => {
    // Structs now carry their type name
    name.as_str()
}
```

### 5. Type Compatibility Updates

Added TypeRef ↔ Struct compatibility (both files):
- `src/semantic/mod.rs:types_compatible()`
- `src/semantic/typecheck.rs:types_compatible()`

```rust
// TypeRef vs Struct compatibility - TypeRef name must match Struct name
(DataType::TypeRef(ref_name), DataType::Struct { name: struct_name, .. }) =>
    ref_name == struct_name,
(DataType::Struct { name: struct_name, .. }, DataType::TypeRef(ref_name)) =>
    struct_name == ref_name,
```

This allows:
```rust
let var z: TypeRef("Complex") = ...;
z := Complex(...);  // Returns Struct { name: "Complex", ... } - now compatible!
```

## Test Results

### All Tests Pass ✅

| Test | Interpreter | Compiler | Description |
|------|:-----------:|:--------:|-------------|
| test_struct_flt_field_access.lt | ✅ 3.5 | ✅ 3.5 | Simple field read |
| test_struct_flt_multiply.lt | ✅ 9 | ✅ 9 | Field multiplication |
| test_struct_flt_arithmetic.lt | ✅ 25 | ✅ 25 | Complex arithmetic |
| test_minimal_struct_method.lt | ✅ 25 | ✅ 25 | Method on struct |
| **test_complex_methods_simple.lt** | ✅ 25,4,6,52 | ✅ 25,4,6,52 | **Methods on variables!** |

### Key Test (Struct Variable Methods)

```lift
type Complex = struct (x: Flt, iy: Flt);

function Complex.magnitude_squared(): Flt {
    self.x * self.x + self.iy * self.iy
};

let var z = Complex(x: 3.0, iy: 4.0);
z.magnitude_squared()  // ✅ Works! Returns 25
```

## Architecture Benefits

### Before Fix
- ❌ Struct types lost their names after resolution
- ❌ Method lookup required backtracking to find TypeRef
- ❌ Variables of struct type couldn't call methods
- ❌ TypeRef and Struct were incompatible types

### After Fix
- ✅ Struct types always carry their name
- ✅ Method lookup directly uses struct name
- ✅ Variables of struct type work perfectly
- ✅ TypeRef and Struct with same name are compatible
- ✅ Cleaner, more maintainable code

## Technical Details

### Lifetime Management

Careful handling of string slices required:
```rust
// Must borrow resolved_type to extend lifetime of extracted name
let type_name = match &resolved_type {  // Note the &
    DataType::Struct { name, .. } => name.as_str(),
    ...
}
```

### Type System Consistency

The fix maintains consistency across:
1. **Type Definition**: Struct created with name
2. **Type Storage**: Symbol table stores Struct with name
3. **Type Resolution**: TypeRef resolves to Struct with same name
4. **Type Compatibility**: TypeRef("X") ↔ Struct { name: "X", ... }
5. **Method Lookup**: Uses Struct.name directly

## Future Improvements

With named structs, future features become easier:
- Better error messages (include type names)
- Type reflection/introspection
- Serialization/deserialization
- Debug printing with type names
- Generic programming support

## Commit Message

```
feat: Add name field to Struct variant for method lookup

- Change DataType::Struct from tuple to struct variant with name field
- Update 88 pattern match sites across codebase
- Fix method lookup to use struct name directly
- Add TypeRef ↔ Struct type compatibility
- Enable method calls on struct variables

Fixes struct method lookup bug where variables lost their type names.
All tests pass in both interpreter and compiler modes.
```

---

**Date**: 2025-11-01
**Fixed By**: Claude Code
**Status**: ✅ Complete - All tests passing
