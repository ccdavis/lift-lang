# User-Defined Methods on Collection Type Aliases

## Summary

User-defined methods **fully work** on collection type aliases (List and Map). You can define custom methods on type aliases like `type Errors = List of Str` and they work correctly.

## What Works

### 1. Custom Methods on List Type Aliases

```lift
type Errors = List of Str;

function Errors.has_errors(): Bool {
    not self.is_empty()
};

function Errors.count(): Int {
    len(self)
};

let errors: Errors = ['Error 1', 'Error 2'];
output(errors.has_errors());  // true
output(errors.count());        // 2
```

### 2. Custom Methods on Map Type Aliases

```lift
type AgeMap = Map of Str to Int;

function AgeMap.has_person(name: Str): Bool {
    self.contains_key(key: name)
};

let ages: AgeMap = #{'Alice': 25, 'Bob': 30};
output(ages.has_person(name: 'Alice'));  // true
```

### 3. Built-in Methods Work on Type Aliases

All built-in methods (first, last, slice, reverse, join, keys, values, etc.) work correctly on type aliases:

```lift
type Names = List of Str;

let names: Names = ['Alice', 'Bob', 'Carol'];
output(names.first());     // 'Alice'
output(names.reverse());   // ['Carol', 'Bob', 'Alice']
```

### 4. Built-in Functions Work on Type Aliases

Functions like `len()` work on type aliases:

```lift
type Errors = List of Str;

let errors: Errors = ['Error 1', 'Error 2'];
output(len(errors));  // 2
```

## Known Limitation: Method Chaining

**Direct method chaining loses type alias information** when calling a built-in method followed by a custom method.

### Problem Example

```lift
type Names = List of Str;

function Names.format_list(): Str {
    self.join(separator: ', ')
};

let names: Names = ['Alice', 'Bob', 'Carol'];

// This FAILS because slice() returns 'List of Str', not 'Names'
output(names.slice(start: 0, end: 2).format_list());
// Error: Method 'format_list' not found for type 'List'
```

### Workaround

Assign the result to a typed variable:

```lift
let subset: Names = names.slice(start: 0, end: 2);
output(subset.format_list());  // Works!
```

### Why This Happens

Built-in methods are generic and return the underlying type (`List of Str`) rather than preserving the type alias (`Names`). This is a fundamental design limitation that would require:
- Generic type parameters / Self type support
- More sophisticated type inference
- Methods parameterized on the actual type

For now, the workaround of using intermediate typed variables is the recommended approach.

## Implementation Fixes

Three fixes were needed to support collection type aliases:

1. **len() function** - Updated to resolve TypeRef to underlying type
2. **not operator** - Updated to handle RuntimeData values
3. **Display for RuntimeData** - Already fixed for primitive types

## Test Files

- `tests/test_collection_type_methods.lt` - Comprehensive test with List and Map
- `tests/test_len_with_typealias.lt` - Tests len() on type aliases
- `tests/test_builtin_on_typealias.lt` - Tests built-in methods on type aliases
- `tests/test_chaining_collection.lt` - Shows workaround for chaining limitation
