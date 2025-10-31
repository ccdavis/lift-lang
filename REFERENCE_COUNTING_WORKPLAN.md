# Reference Counting Implementation Work Plan

## Project Status: Phase 1 Complete ✅

This document tracks the implementation of automatic memory management via reference counting for the Lift language compiler.

---

## Phase 1: Infrastructure (COMPLETED)

### What Was Done

**1. Core RefCounted<T> Infrastructure** (src/runtime.rs:15-93)
- ✅ Generic reference-counted wrapper with atomic counter
- ✅ `RefCounted::new()` - Create with count = 1
- ✅ `RefCounted::retain()` - Increment count (thread-safe)
- ✅ `RefCounted::release()` - Decrement count, free at zero
- ✅ `RefCounted::get() / get_mut()` - Safe data access
- ✅ Memory ordering guarantees (Release/Acquire semantics)

**2. Type Aliases for Collections**
```rust
pub type RcList = RefCounted<LiftList>;
pub type RcMap = RefCounted<LiftMap>;
pub type RcStruct = RefCounted<LiftStruct>;
pub type RcRange = RefCounted<LiftRange>;
```

**3. Runtime Functions Updated**

All collection creation functions now return RefCounted pointers:
- `lift_list_new() -> *mut RcList`
- `lift_map_new() -> *mut RcMap`
- `lift_struct_new() -> *mut RcStruct`
- `lift_range_new() -> *mut RcRange`

New retain/release functions for each type:
- `lift_list_retain() / lift_list_release()`
- `lift_map_retain() / lift_map_release()`
- `lift_struct_retain() / lift_struct_release()`
- `lift_range_retain() / lift_range_release()`

All accessor functions (get, set, len, etc.) updated to work with RefCounted wrappers.

**4. Backward Compatibility**
- `lift_*_free()` functions now call `lift_*_release()`
- Binary compatibility maintained
- All existing tests pass (52/52 compiler tests, 78/79 integration tests)

### Test Results
```
✅ All 52 compiler unit tests passing
✅ 78/79 integration tests passing
   (1 pre-existing failure: test_lt_file_struct_comparison)
✅ No regressions introduced
✅ Runtime compiles without errors
```

---

## Phase 2: Code Generation Integration (TODO)

### Goal
Automatically insert retain/release calls in generated code to prevent memory leaks.

### Tasks

#### 2.1 Declare Runtime Functions in Cranelift
**File**: `src/cranelift/runtime.rs`

Add function declarations for retain/release (similar to existing declarations):

```rust
// lift_list_retain(*mut RcList)
let mut sig = self.module.make_signature();
sig.params.push(AbiParam::new(pointer_type));
let func_id = self.module.declare_function(
    "lift_list_retain",
    cranelift_module::Linkage::Import,
    &sig
)?;
self.runtime_funcs.insert("lift_list_retain".to_string(), func_id);

// lift_list_release(*mut RcList)
let mut sig = self.module.make_signature();
sig.params.push(AbiParam::new(pointer_type));
let func_id = self.module.declare_function(
    "lift_list_release",
    cranelift_module::Linkage::Import,
    &sig
)?;
self.runtime_funcs.insert("lift_list_release".to_string(), func_id);
```

Repeat for: `lift_map_*`, `lift_struct_*`, `lift_range_*`

**Estimated Effort**: 1-2 hours

#### 2.2 Add Allocation Tracking to CodeGenerator
**File**: `src/cranelift/mod.rs` (CodeGenerator struct)

Add field to track allocations per scope:

```rust
pub struct CodeGenerator<'a, M: Module> {
    // ... existing fields ...

    /// Track heap allocations per scope for cleanup
    /// Maps scope depth to list of (pointer_value, type_name)
    scope_allocations: Vec<Vec<(Value, String)>>,
}
```

**Estimated Effort**: 30 minutes

#### 2.3 Record Allocations During Compilation
**Files**: `src/cranelift/collections.rs`, `src/cranelift/structs.rs`, `src/cranelift/ranges.rs`

When compiling collection literals, record the allocation:

```rust
// In compile_list_literal(), after calling lift_list_new:
let list_ptr = /* result of lift_list_new call */;
self.record_allocation(list_ptr, "list");

// Helper method in CodeGenerator:
fn record_allocation(&mut self, ptr: Value, type_name: &str) {
    if let Some(current_scope) = self.scope_allocations.last_mut() {
        current_scope.push((ptr, type_name.to_string()));
    }
}
```

**Estimated Effort**: 2-3 hours

#### 2.4 Implement Scope Entry/Exit
**File**: `src/cranelift/blocks.rs` (or wherever blocks are compiled)

```rust
fn enter_scope(&mut self) {
    self.scope_allocations.push(Vec::new());
}

fn exit_scope(&mut self) {
    if let Some(allocations) = self.scope_allocations.pop() {
        for (ptr, type_name) in allocations {
            self.emit_release_call(ptr, &type_name);
        }
    }
}

fn emit_release_call(&mut self, ptr: Value, type_name: &str) {
    let func_name = format!("lift_{}_release", type_name);
    if let Some(&func_id) = self.runtime_funcs.get(&func_name) {
        let func_ref = self.module.declare_func_in_func(func_id, self.builder.func);
        self.builder.ins().call(func_ref, &[ptr]);
    }
}
```

Call `enter_scope()` / `exit_scope()` when compiling:
- Block expressions
- Function bodies
- If/else branches
- While loop bodies

**Estimated Effort**: 3-4 hours

#### 2.5 Handle Special Cases

**Return Statements**
- Don't release returned values (caller takes ownership)
- Release all other allocations before return

**Function Parameters**
- Parameters received as pointers: DON'T release (caller owns)
- Parameters marked `cpy`: Release at function exit (we own the copy)

**Variable Assignments**
- When reassigning mutable variables, release old value before assigning new

**Estimated Effort**: 2-3 hours

---

## Phase 3: Testing & Validation (TODO)

### 3.1 Unit Tests
Create tests for reference counting behavior:
- `test_refcount_basic` - Create, retain, release
- `test_refcount_nested` - Nested collections properly cleaned
- `test_refcount_scope` - Allocations released at scope exit
- `test_refcount_function_call` - Ownership across function boundaries

**File**: `src/compiler.rs` (add to existing tests module)

**Estimated Effort**: 2-3 hours

### 3.2 Integration Tests
Test real programs for memory leaks:
- String concatenation in loops (currently leaks)
- List/map method chaining (currently leaks intermediate results)
- Recursive functions with allocations

**Tool**: Valgrind or similar memory profiler

**Estimated Effort**: 3-4 hours

### 3.3 Benchmark Performance Impact
Compare before/after:
- Execution time (expect ~5-10% overhead from refcount operations)
- Memory usage (should see dramatic reduction in long-running programs)

**Estimated Effort**: 1-2 hours

---

## Phase 4: String Management (FUTURE)

### Current State
Strings use C-string convention (`*mut c_char`) without reference counting.

### Proposal
Option A: Wrap CString in RefCounted
```rust
pub type RcString = RefCounted<CString>;
```

Option B: String interning
- Maintain global string table
- Return pointers into table
- Reference count entire table

**Complexity**: Medium
**Estimated Effort**: 4-6 hours
**Benefits**: Eliminates string concatenation leaks

---

## Known Limitations & Future Work

### Current Limitations
1. **No automatic cleanup yet** - Infrastructure in place but not used by codegen
2. **String leaks persist** - Strings not reference counted
3. **No circular reference detection** - Would require weak references
4. **Method return values leak** - Intermediate results not tracked

### Future Enhancements
1. **Weak References**: For circular data structures
2. **Copy-on-Write**: Optimize immutable collections
3. **Escape Analysis**: Detect stack-allocatable objects
4. **Arena Allocator**: Bulk-free for short-lived objects

---

## Development Workflow

### To Continue This Work

1. **Checkout this branch**:
   ```bash
   git checkout reference-count
   ```

2. **Build and test**:
   ```bash
   cargo build
   cargo test test_compile  # Run compiler tests
   ```

3. **Start with Phase 2.1**: Declare runtime functions in Cranelift

4. **Incremental approach**:
   - Implement one scope type at a time (e.g., start with function bodies)
   - Test after each change
   - Commit frequently

### Testing Strategy

After implementing cleanup in a specific scope type:

1. **Create minimal test case**:
   ```rust
   #[test]
   fn test_cleanup_function_scope() {
       let code = r#"
           function test(): Int {
               let list = [1, 2, 3];
               42
           }
           test()
       "#;
       assert!(compile_and_run(code).is_ok());
   }
   ```

2. **Verify with memory profiler**:
   ```bash
   valgrind --leak-check=full ./target/debug/lift-lang --compile test.lt
   ```

3. **Run full test suite**:
   ```bash
   cargo test
   ```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                     Lift Source Code                        │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
              ┌──────────────┐
              │    Parser    │
              │  (LALRPOP)   │
              └──────┬───────┘
                     │
                     ▼
              ┌──────────────┐
              │ Type Checker │
              └──────┬───────┘
                     │
                     ▼
         ┌───────────────────────┐
         │   Code Generator      │
         │                       │
         │  [NEW] Track Allocs   │◄──┐
         │  ├─ enter_scope()     │   │
         │  ├─ record_allocation │   │
         │  └─ exit_scope()      │   │
         └───────┬───────────────┘   │
                 │                   │
                 ▼                   │
         ┌──────────────┐            │
         │  Cranelift   │            │
         │   IR + JIT   │            │
         └──────┬───────┘            │
                │                    │
                ▼                    │
         ┌──────────────┐            │
         │   Native     │            │
         │  x86-64 Code │            │
         └──────┬───────┘            │
                │                    │
                ▼                    │
         ┌─────────────────────┐    │
         │  Runtime Library    │    │
         │                     │    │
         │  [NEW] RefCounted   │────┘
         │  ├─ retain()        │
         │  ├─ release()       │
         │  └─ get()/get_mut() │
         └─────────────────────┘
```

---

## Success Criteria

Reference counting will be considered complete when:

1. ✅ **Infrastructure implemented** (DONE)
2. ☐ **No memory leaks in collection operations**
   - List/map creation and manipulation
   - Struct creation and field access
   - Range literals
3. ☐ **Scope-based cleanup working**
   - Function scope
   - Block scope
   - Loop scope
   - If/else branches
4. ☐ **Performance acceptable**
   - <10% overhead vs. current (leaky) implementation
   - Memory usage scales properly
5. ☐ **All tests passing**
   - 52/52 compiler unit tests
   - 79/79 integration tests (after fixing pre-existing failure)
   - New refcount-specific tests

---

## Resources & References

### Related Code Files
- **Runtime**: `src/runtime.rs` (RefCounted infrastructure)
- **Code Generation**: `src/cranelift/*.rs` (IR generation)
- **Collections**: `src/cranelift/collections.rs` (list/map compilation)
- **Structs**: `src/cranelift/structs.rs` (struct compilation)
- **Tests**: `src/compiler.rs` (unit tests)

### Documentation
- Cranelift docs: https://docs.rs/cranelift
- Rust atomics: https://doc.rust-lang.org/std/sync/atomic/

### Similar Implementations
- Swift ARC: https://docs.swift.org/swift-book/LanguageGuide/AutomaticReferenceCounting.html
- Python reference counting: https://docs.python.org/3/c-api/refcounting.html

---

**Last Updated**: 2025-10-31
**Branch**: `reference-count`
**Status**: Phase 1 Complete, Ready for Phase 2
