# Cranelift Compiler - Implementation Checklist

**Use this checklist when resuming work. Check off items as you complete them.**

---

## ✅ COMPLETED - Phases 1-3

- [x] Phase 1: Logical Operators (And/Or)
  - [x] Add And/Or to compile_binary_expr
  - [x] Add 7 compiler tests
  - [x] Validate with test_logical_operators.lt

- [x] Phase 2: Float Arithmetic
  - [x] Add VarInfo struct for type tracking
  - [x] Implement float arithmetic (fadd, fsub, fmul, fdiv)
  - [x] Implement float comparisons (fcmp)
  - [x] Add 5 compiler tests
  - [x] Validate with test_float.lt

- [x] Phase 3: Range Operator
  - [x] Add range runtime functions to runtime.rs
  - [x] Declare range functions in codegen.rs
  - [x] Implement range compilation
  - [x] Register JIT symbols in compiler.rs
  - [x] Add 2 compiler tests
  - [x] Validate with test_ranges.lt

---

## 🚧 TODO - Phase 4: User-Defined Functions

### 4.1 Design & Setup (15 minutes)
- [ ] Read `COMPILER_COMPLETION_HANDOFF.md` Section 4
- [ ] Document calling convention decision in codegen.rs comments
- [ ] Add `HashMap<String, FuncId> function_refs` to CodeGenerator struct

### 4.2 Function Definition (2 hours)
- [ ] Create `compile_define_function` in src/codegen.rs
  - [ ] Extract Function from Expr::Lambda
  - [ ] Build Cranelift signature from params + return type
  - [ ] Declare function in module
  - [ ] Store FuncId in function_refs
  - [ ] Create FunctionBuilder for new function
  - [ ] Map parameters to function params (not stack vars)
  - [ ] Compile lambda body expression
  - [ ] Handle return value
  - [ ] Finalize and define function

- [ ] Update Expr::DefineFunction case in compile_expr_static
  ```rust
  Expr::DefineFunction { fn_name, value, .. } => {
      Self::compile_define_function(&mut self.module, fn_name, value, symbols, &mut self.function_refs)?;
      Ok(None)
  }
  ```

### 4.3 Function Calls (1.5 hours)
- [ ] Create `compile_function_call` in src/codegen.rs
  - [ ] Look up function in function_refs
  - [ ] Import function reference
  - [ ] Get function signature from symbols
  - [ ] Evaluate arguments in parameter order
  - [ ] Call function with builder.ins().call()
  - [ ] Handle return value (Some or None for Unit)

- [ ] Update Expr::Call case in compile_expr_static
  ```rust
  Expr::Call { fn_name, args, .. } => {
      Self::compile_function_call(builder, fn_name, args, symbols,
                                  runtime_funcs, variables, function_refs)
  }
  ```

### 4.4 Parameter Handling (30 minutes)
- [ ] Add support for regular params (immutable)
- [ ] Add support for `cpy` params (mutable, pass-by-value)
  - [ ] Allocate stack slot for cpy params
  - [ ] Copy param value to stack
  - [ ] Allow := assignment on cpy params

### 4.5 Testing (1 hour)
- [ ] Add test: `test_compile_simple_function` (no params, returns literal)
- [ ] Add test: `test_compile_function_with_int_params` (basic arithmetic)
- [ ] Add test: `test_compile_function_arithmetic` (x + y)
- [ ] Add test: `test_compile_nested_function_calls` (call from call)
- [ ] Add test: `test_compile_recursive_factorial` (recursion test)
- [ ] Add test: `test_compile_cpy_parameter` (mutable params)
- [ ] Create tests/test_functions_compiled.lt
- [ ] Validate: Run existing function test files

### 4.6 Verify Phase 4 Complete
- [ ] All 6 function tests pass
- [ ] Recursive functions work (factorial, fibonacci)
- [ ] cpy parameters behave correctly
- [ ] tests/test_recursion_depth.lt passes

---

## 🚧 TODO - Phase 5: Built-in Methods

### 5.1 String Methods Runtime (1 hour)
- [ ] Add to src/runtime.rs (after Range functions):
  - [ ] `lift_str_upper(s) -> *c_char`
  - [ ] `lift_str_lower(s) -> *c_char`
  - [ ] `lift_str_substring(s, start, end) -> *c_char`
  - [ ] `lift_str_contains(s, needle) -> i8`
  - [ ] `lift_str_trim(s) -> *c_char`
  - [ ] `lift_str_split(s, delim) -> *LiftList`
  - [ ] `lift_str_replace(s, old, new) -> *c_char`
  - [ ] `lift_str_starts_with(s, prefix) -> i8`
  - [ ] `lift_str_ends_with(s, suffix) -> i8`
  - [ ] `lift_str_is_empty(s) -> i8`

### 5.2 List Methods Runtime (45 minutes)
- [ ] Add to src/runtime.rs:
  - [ ] `lift_list_first(list) -> i64`
  - [ ] `lift_list_last(list) -> i64`
  - [ ] `lift_list_contains(list, item) -> i8`
  - [ ] `lift_list_slice(list, start, end) -> *LiftList`
  - [ ] `lift_list_reverse(list) -> *LiftList`
  - [ ] `lift_list_join(list, sep) -> *c_char`
  - [ ] `lift_list_is_empty(list) -> i8`

### 5.3 Map Methods Runtime (30 minutes)
- [ ] Add to src/runtime.rs:
  - [ ] `lift_map_keys(map) -> *LiftList`
  - [ ] `lift_map_values(map) -> *LiftList`
  - [ ] `lift_map_contains_key(map, key) -> i8`
  - [ ] `lift_map_is_empty(map) -> i8`

### 5.4 Declare Runtime Methods (30 minutes)
- [ ] Add all 21 method declarations to declare_runtime_functions() in src/codegen.rs
  - [ ] 10 string method declarations
  - [ ] 7 list method declarations
  - [ ] 4 map method declarations

### 5.5 Register JIT Symbols (15 minutes)
- [ ] Add all 21 symbols to JITCompiler::new() in src/compiler.rs
  - [ ] String method symbols
  - [ ] List method symbols
  - [ ] Map method symbols

### 5.6 Method Call Compilation (1.5 hours)
- [ ] Implement compile_method_call in src/codegen.rs
  - [ ] Determine receiver type
  - [ ] Look up BuiltinMethod from type + method name
  - [ ] Compile receiver expression
  - [ ] Compile method arguments
  - [ ] Map BuiltinMethod to runtime function name (21 cases)
  - [ ] Call runtime function
  - [ ] Handle return value (extend i8 to i64 for booleans)

### 5.7 Testing (1 hour)
- [ ] Add test: `test_compile_str_upper`
- [ ] Add test: `test_compile_str_substring`
- [ ] Add test: `test_compile_str_split`
- [ ] Add test: `test_compile_list_first`
- [ ] Add test: `test_compile_list_slice`
- [ ] Add test: `test_compile_map_keys`
- [ ] Add test: `test_compile_map_contains_key`
- [ ] Validate: Run tests/test_string_methods.lt
- [ ] Validate: Run tests/test_list_methods.lt
- [ ] Validate: Run tests/test_map_methods.lt

### 5.8 Verify Phase 5 Complete
- [ ] All 7 method tests pass
- [ ] String methods test file passes
- [ ] List methods test file passes
- [ ] Map methods test file passes
- [ ] Method chaining works: `'hello'.upper().replace(...)`

---

## 🚧 TODO - Phase 6: Integration & Documentation

### 6.1 Integration Testing (30 minutes)
- [ ] Create scripts/validate_compiler.sh
  - [ ] Loop through all .lt files
  - [ ] Run each through interpreter
  - [ ] Run each through compiler
  - [ ] Compare outputs
  - [ ] Report failures
- [ ] Run integration script
- [ ] Fix any failing tests
- [ ] Verify 100% pass rate

### 6.2 Documentation Updates (20 minutes)
- [ ] Update CLAUDE.md
  - [ ] Add "Compiler Status" section
  - [ ] List all supported features
  - [ ] Document usage (--compile flag)
  - [ ] Note limitations (no closures, no for loops)
- [ ] Update README.md
  - [ ] Add compiler feature to features list
  - [ ] Add compiler usage examples

### 6.3 Performance Benchmarks (Optional, 1 hour)
- [ ] Create benches/compiler_vs_interpreter.rs
- [ ] Benchmark: Fibonacci(30)
- [ ] Benchmark: List operations (1000 elements)
- [ ] Benchmark: Nested loops
- [ ] Benchmark: Function calls (recursion)
- [ ] Document results

### 6.4 Verify Phase 6 Complete
- [ ] Integration script shows 100% pass
- [ ] Documentation updated
- [ ] All interpreter tests work in compiler mode
- [ ] (Optional) Benchmarks created

---

## 🎯 Final Verification

### All Phases Complete When:
- [ ] All 20+ compiler tests pass
- [ ] All .lt test files work in both modes
- [ ] Functions compile and call correctly
- [ ] Methods compile and work correctly
- [ ] Documentation reflects compiler support
- [ ] No regression in existing features

### Commands to Verify:
```bash
# All tests pass
cargo test

# Integration tests pass
./scripts/validate_compiler.sh

# Manual smoke test
cargo run -- --compile tests/test_functions_compiled.lt
cargo run -- --compile tests/test_string_methods.lt
```

---

## 📝 Notes While Working

### Issues Encountered:
<!-- Add any issues you find while implementing -->

### Solutions Applied:
<!-- Document solutions for future reference -->

### Performance Observations:
<!-- Note any performance differences -->

---

## 🔄 Progress Tracking

**Started**: ___________
**Phase 4 Complete**: ___________
**Phase 5 Complete**: ___________
**Phase 6 Complete**: ___________
**100% Done**: ___________

**Estimated Total Time**: 8-10 hours
**Actual Time**: ___________

---

## 📚 Quick Reference

**Key Files**:
- Compilation logic: `src/codegen.rs`
- JIT setup: `src/compiler.rs`
- Runtime functions: `src/runtime.rs`
- Tests: `src/compiler.rs` (bottom section)

**Test Commands**:
```bash
cargo test test_compile                    # All compiler tests
cargo test test_compile_simple_function    # Specific test
cargo run -- --compile tests/file.lt       # Run compiled version
RUST_BACKTRACE=1 cargo test test_name     # Debug test
```

**Documentation**:
- Detailed guide: `COMPILER_COMPLETION_HANDOFF.md`
- Status overview: `COMPILER_STATUS.md`
- This checklist: `COMPILER_TODO_CHECKLIST.md`

---

**Last Updated**: 2025-10-06
**Next Action**: Phase 4, Step 4.1 - Design & Setup
