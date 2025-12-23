# Higher-Order Functions Implementation Plan

## Goal
Add support for passing functions as values, enabling higher-order functions like `map()`, `filter()`, `reduce()`.

## Design Decisions

### Lambda as Last Argument
Higher-order functions should take the lambda as the **last** argument:
```lift
function map(list: List of Int, f: Fn(Int): Int): List of Int { ... }
```

This enables UFCS chaining:
```lift
[1, 2, 3].map(f: Lambda (x: Int): Int { x * 2 })
         .filter(f: Lambda (x: Int): Bool { x > 2 })
         .map(f: Lambda (x: Int): Int { x + 1 })
```

Future: Could add Ruby-style trailing block syntax.

### Initial Scope: No Captures in Passed Lambdas
For simplicity, lambdas passed as arguments initially won't support captures.
This avoids needing closure objects (fn ptr + environment).

## Implementation Phases

### Phase 1: Grammar & Types
- [x] Add `Fn(ParamTypes): ReturnType` syntax to grammar
- [x] Add `DataType::Fn { params, return_type }` variant
- [x] Parse function types in parameter declarations
- [x] Update type display/debug formatting (uses Debug derive)

### Phase 2: Type Checking
- [x] Infer types for lambda expressions
- [x] Check function type compatibility in calls
- [x] Validate lambda parameter/return types match expected Fn type

### Phase 3: Interpreter Support
- [x] Store lambda values in variables
- [x] Handle indirect function calls (calling a variable)
- [x] Verified with user-defined HOFs (map_print, sum_with, count_if)
- Note: Built-in map/filter returning new lists need list concatenation

### Phase 4: Compiler Support (Pending)
- [ ] Represent functions as values (function pointers)
- [ ] Generate code for indirect calls using Cranelift
- [ ] Compile user-defined HOFs and verify output matches interpreter

### Phase 5: User-Definable HOFs (Complete for Interpreter)
List concatenation added, HOFs now fully functional in interpreter.
- [x] Add list concatenation support (`+` for lists)
- [x] Add runtime `lift_list_push`, `lift_list_reserve`, `lift_list_concat`
- [x] User-defined `map`, `filter`, `reduce`, `foreach` all work
- [ ] Add as built-in methods for better ergonomics (optional)

## Progress Log

### Session 1: 2024-XX-XX
- Created implementation plan
- Status: Starting Phase 1

### Session 2: 2025-12-23
- **Phase 1 Complete**: Added `Fn(ParamTypes): ReturnType` syntax to grammar
- Added `DataType::Fn { params, return_type }` variant
- Added `DataType::Unit` for Unit return type in function signatures
- **Phase 2 Complete**: Lambda expressions now return `DataType::Fn` type
- Added type compatibility checking for Fn types
- Updated type checker to allow calls to variables with Fn type
- **Phase 3 Mostly Complete**: Implemented indirect function calls in interpreter
- Lambdas are now first-class values (don't auto-execute on creation)
- Added positional argument matching for indirect calls
- Tested with `apply_twice` function - works correctly
- Status: Phases 1-3 complete for interpreter, Phase 4 (compiler) pending

### Session 2 (continued): 2025-12-23
- Verified HOF support with comprehensive tests:
  - `map_print`: applies function to each element and prints
  - `sum_with`: reduces list using a binary function
  - `count_if`: counts elements matching a predicate
- All tests pass correctly
- Note: Full map/filter need list concatenation which is not yet implemented
- Status: Interpreter HOF support complete, compiler support and list concat pending

### Session 3: 2025-12-23
- **List Operations Added**:
  - Added `lift_list_push()` for dynamic list building
  - Added `lift_list_reserve()` for pre-allocation
  - Added `lift_list_concat()` for list concatenation
- **Type Checking**: Added list concatenation with `+` operator
- **Interpreter**: List concatenation now works in all contexts
- **Full HOF Test Passed**:
  - `map_int`: `[1,2,3,4,5]` → `[2,4,6,8,10]`
  - `filter_int`: `[1,2,3,4,5]` → `[2,4]` (even numbers)
  - `reduce_int`: `1+2+3+4+5 = 15`
  - `foreach_int`: side effects work
- **Status**: Interpreter fully supports HOFs, compiler support pending

---

## Technical Notes

### Function Type Syntax Options
```
Fn(Int): Int           // Single param
Fn(Int, Int): Bool     // Multiple params
Fn(): Unit             // No params
```

### DataType Addition
```rust
pub enum DataType {
    // ... existing variants ...
    Fn {
        params: Vec<DataType>,
        return_type: Box<DataType>,
    },
}
```

### Compiler Representation
Without captures, a lambda is just a function pointer:
- Compile lambda to a named function
- Pass function pointer as i64
- Indirect call: `call_indirect` or cast and call

### Future: Captures
If we later add captures to passed lambdas:
- Need closure struct: { fn_ptr, captured_values... }
- Pass closure pointer instead of fn pointer
- More complex calling convention
