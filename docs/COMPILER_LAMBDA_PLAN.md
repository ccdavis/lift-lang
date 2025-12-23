# Compiler Lambda & Indirect Call Support

## Goal
Enable the Cranelift JIT compiler to support:
1. Anonymous lambdas as first-class values (function pointers)
2. Indirect function calls through `Fn`-typed variables

## Current State

**Working in Interpreter:**
- Lambdas are first-class values
- Indirect calls via `Fn`-typed parameters work
- Full HOF support (map, filter, reduce, foreach)

**Compiler Limitations:**
- `Expr::Lambda` falls through to error case in `compile_expr_static`
- No mechanism to compile anonymous lambdas
- No indirect call support for `Fn`-typed variables

## Implementation Plan

### Phase 1: Lambda Collection & Naming
**Goal:** Collect all lambdas (including anonymous ones) during preprocessing and assign unique names.

- [ ] **1.1** Extend `collect_function_definitions` to find lambdas in:
  - Function call arguments (`Expr::Call { args }`)
  - Let bindings (`Expr::Let { value }`)
  - Method call arguments (`Expr::MethodCall { args }`)

- [ ] **1.2** Create lambda naming scheme:
  - Generate unique names like `__lambda_0`, `__lambda_1`, etc.
  - Store mapping: `HashMap<LambdaId, String>` where LambdaId identifies the lambda's location

- [ ] **1.3** Add `anonymous_lambdas` field to `CodeGenerator`:
  ```rust
  pub(super) anonymous_lambdas: HashMap<(usize, usize), String>,
  // Maps (scope_id, expr_index) -> generated function name
  ```

**Files to modify:**
- `src/cranelift/codegen.rs` - Add field, update `compile_program`
- `src/cranelift/functions.rs` - Extend `collect_function_definitions`

### Phase 2: Compile Anonymous Lambdas
**Goal:** Compile collected anonymous lambdas as regular functions.

- [ ] **2.1** Modify lambda collection to store the Lambda expression:
  ```rust
  anonymous_lambdas: Vec<(String, &Expr)>  // (generated_name, lambda_expr)
  ```

- [ ] **2.2** In `compile_program`, compile anonymous lambdas alongside named functions:
  ```rust
  // After collecting function_defs
  for (name, lambda_expr) in &anonymous_lambdas {
      self.compile_user_function(name, lambda_expr, symbols)?;
  }
  ```

- [ ] **2.3** Handle captures for anonymous lambdas:
  - Include them in capture analysis passes
  - Anonymous lambdas can capture variables from their definition scope

**Files to modify:**
- `src/cranelift/codegen.rs` - Compile anonymous lambdas
- `src/cranelift/functions.rs` - Include in capture analysis

### Phase 3: Lambda as Value (Function Pointer)
**Goal:** When a Lambda is used as a value, return its function address.

- [ ] **3.1** Add `Expr::Lambda` case to `compile_expr_static`:
  ```rust
  Expr::Lambda { .. } => {
      // Look up the lambda's compiled function name
      let fn_name = self.get_lambda_function_name(lambda_id)?;

      // Get function reference
      let func_ref = user_func_refs.get(&fn_name)?;

      // Get function address as i64
      let func_addr = builder.ins().func_addr(types::I64, func_ref);
      Ok(Some(func_addr))
  }
  ```

- [ ] **3.2** Need to pass lambda identification through compile chain:
  - Option A: Use lambda's `environment` field as identifier
  - Option B: Add expression location tracking
  - Option C: Store pre-computed mapping in CodeGenerator

- [ ] **3.3** Register anonymous lambda function refs:
  ```rust
  // In compile_program, after compiling anonymous lambdas
  for (name, _) in &anonymous_lambdas {
      let func_ref = self.module.declare_func_in_func(func_id, builder.func);
      user_func_refs.insert(name.clone(), func_ref);
  }
  ```

**Files to modify:**
- `src/cranelift/codegen.rs` - Add Lambda case
- `src/cranelift/functions.rs` - Lambda identification

### Phase 4: Indirect Function Calls
**Goal:** Call through a variable that holds a function pointer.

- [ ] **4.1** Detect indirect calls in `compile_function_call`:
  ```rust
  // Check if fn_name is a variable with Fn type
  if let Some(var_type) = symbols.get_symbol_type(index) {
      if let DataType::Fn { params, return_type } = var_type {
          return self.compile_indirect_call(builder, index, args, params, return_type);
      }
  }
  ```

- [ ] **4.2** Implement `compile_indirect_call`:
  ```rust
  fn compile_indirect_call(
      builder: &mut FunctionBuilder,
      fn_var_index: (usize, usize),
      args: &[KeywordArg],
      param_types: &[DataType],
      return_type: &DataType,
  ) -> Result<Option<Value>, String> {
      // 1. Load the function pointer from the variable
      let fn_ptr = load_variable(fn_var_index);

      // 2. Create signature for the call
      let sig = create_signature(param_types, return_type);
      let sig_ref = builder.import_signature(sig);

      // 3. Compile arguments (positional order)
      let arg_values = compile_args_positional(args, param_types);

      // 4. Make indirect call
      let call = builder.ins().call_indirect(sig_ref, fn_ptr, &arg_values);

      // 5. Get return value
      Ok(builder.inst_results(call).first().copied())
  }
  ```

- [ ] **4.3** Handle special cases:
  - Functions returning strings (hidden dest pointer)
  - Functions with captures (may need closure struct later)

**Files to modify:**
- `src/cranelift/functions.rs` - Add `compile_indirect_call`

### Phase 5: Testing & Verification
**Goal:** Verify compiler output matches interpreter.

- [ ] **5.1** Create compiler test for basic lambda:
  ```lift
  function apply(x: Int, f: Fn(Int): Int): Int { f(n: x) };
  apply(x: 5, f: Lambda (n: Int): Int { n * 2 })  // Should return 10
  ```

- [ ] **5.2** Create compiler test for HOFs:
  ```lift
  function map_int(list: List of Int, f: Fn(Int): Int): List of Int { ... }
  map_int(list: [1,2,3], f: Lambda (x: Int): Int { x * 2 })
  ```

- [ ] **5.3** Compare interpreter vs compiler output for all HOF tests

- [ ] **5.4** Add unit tests in `src/compiler.rs`:
  - `test_compile_lambda_as_value`
  - `test_compile_indirect_call`
  - `test_compile_hof_map`

**Files to modify:**
- `src/compiler.rs` - Add tests
- `tests/` - Add integration test files

## Technical Notes

### Cranelift Indirect Calls
```rust
// Create a signature
let mut sig = Signature::new(CallConv::SystemV);
sig.params.push(AbiParam::new(types::I64));
sig.returns.push(AbiParam::new(types::I64));

// Import the signature
let sig_ref = builder.import_signature(sig);

// Get function pointer (from variable)
let fn_ptr = builder.ins().stack_load(types::I64, var_slot, 0);

// Make indirect call
let call = builder.ins().call_indirect(sig_ref, fn_ptr, &[arg1, arg2]);
let result = builder.inst_results(call)[0];
```

### Lambda Identification Strategy
Use the lambda's `environment` field (scope ID) as part of the identifier:
```rust
// During collection
let lambda_id = format!("__lambda_{}_{}", scope_id, lambda_count);
lambda_map.insert(environment, lambda_id);

// During compilation
if let Expr::Lambda { environment, .. } = expr {
    let fn_name = lambda_map.get(environment)?;
}
```

### Capture Handling for Anonymous Lambdas
Anonymous lambdas need the same capture treatment as named functions:
1. Compute direct captures
2. Compute transitive captures
3. Pass captured values as hidden parameters

## Progress Tracking

| Phase | Status | Notes |
|-------|--------|-------|
| Phase 1: Lambda Collection | ✅ Complete | `collect_anonymous_lambdas`, `anonymous_lambdas` HashMap |
| Phase 2: Compile Lambdas | ✅ Complete | Anonymous lambdas compiled like named functions |
| Phase 3: Lambda as Value | ✅ Complete | `Expr::Lambda` returns `func_addr` |
| Phase 4: Indirect Calls | ✅ Complete | `compile_indirect_call` with `call_indirect` |
| Phase 5: Testing | ✅ Complete | HOF tests pass in both interpreter and compiler |

## Implementation Summary (Completed 2025-12-23)

Core HOF functionality is now fully implemented:
- Anonymous lambdas are collected and compiled as functions with generated names (`__lambda_0`, etc.)
- `Expr::Lambda` expressions return function pointers via Cranelift's `func_addr`
- Function calls through `Fn`-typed variables use `call_indirect`
- Arguments are passed positionally (Fn types don't carry parameter names)
- String-returning HOFs use hidden dest pointer convention

**Known Limitations:**
- Lambdas calling functions defined in parent scopes (nested functions) require additional capture handling
- Closures with mutable captures not yet supported (would need closure structs)

## Dependencies
- Phases 1-3 must be done before Phase 4
- Phase 4 requires Phase 3 (need function pointers to call)
- Phase 5 can start after Phase 4

## Estimated Complexity
- Phase 1: Medium (AST traversal changes)
- Phase 2: Low (reuse existing compile_user_function)
- Phase 3: Medium (lambda identification, func_addr)
- Phase 4: High (signature creation, call_indirect)
- Phase 5: Low (test writing)
