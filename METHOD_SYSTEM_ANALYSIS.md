# Method System Analysis - Hard-Coding Issues

**Date:** 2025-10-04
**Status:** Problem Identified - Needs Refactor

---

## 🔴 Problem Statement

The current method implementation has **hard-coded dispatch** in the interpreter. Methods are not generically associated with their receiver types - instead, the interpreter has a giant match statement that checks method names and receiver types manually.

### Current Bad Pattern

**In `src/interpreter.rs` (lines ~190-540):**

```rust
match method_name.as_str() {
    "upper" if args.is_empty() => {
        match receiver_value {
            Expr::Literal(LiteralData::Str(..)) | Expr::RuntimeData(LiteralData::Str(..)) => {
                // Hard-coded string upper logic
            }
            _ => Err(...)
        }
    }
    "contains" if args.len() == 1 => {
        match receiver_value {
            // String version - hard-coded
            Expr::Literal(LiteralData::Str(..)) => { /* string contains */ }
            // List version - hard-coded
            Expr::RuntimeList { .. } => { /* list contains */ }
            _ => Err(...)
        }
    }
    // ... 15 more hard-coded methods
}
```

---

## 🎯 What We Want

**Generic method dispatch based on receiver type:**

1. ✅ Methods registered in symbol table with their receiver type (already done!)
2. ✅ Compiler resolves methods by looking up `Type.method_name` (already done!)
3. ❌ Interpreter should dispatch based on receiver type, NOT hard-code logic
4. ❌ Users should be able to define methods on built-in types
5. ❌ Built-in methods should use same mechanism as user-defined methods

---

## 📊 Current System Architecture

### What Works Well ✅

#### 1. Symbol Table Registration (`src/symboltable.rs`)

```rust
// Built-ins are registered with full type information
self.add_builtin_method("Str", "upper", DataType::Str, DataType::Str)?;
self.add_builtin_method_with_params("Str", "contains",
    DataType::Str,
    vec![("substring", DataType::Str)],
    DataType::Bool)?;

// This creates: Str.upper, Str.contains in symbol table
```

**This is good!** Methods are associated with types in the symbol table.

#### 2. Semantic Analysis (`src/semantic_analysis.rs`)

```rust
// Method resolution builds full name: Type.method_name
let type_name = match receiver_type {
    DataType::Str => "Str",
    DataType::List { .. } => "List",
    DataType::Map { .. } => "Map",
    _ => ...
};

let full_method_name = format!("{}.{}", type_name, method_name);

// Look up in symbol table
if let Some(found_index) = symbols.find_index_reachable_from(&full_method_name, scope) {
    *fn_index = found_index;
}
```

**This is good!** Compiler resolves methods generically based on receiver type.

### What's Broken ❌

#### 3. Interpreter Dispatch (`src/interpreter.rs`)

```rust
Expr::MethodCall { receiver, method_name, fn_index, args } => {
    let receiver_value = receiver.interpret(symbols, current_scope)?;

    // ❌ HARD-CODED: Ignores fn_index, matches on method name
    match method_name.as_str() {
        "upper" => { /* hard-coded */ }
        "lower" => { /* hard-coded */ }
        "contains" => { /* hard-coded */ }
        // ... 14 more hard-coded methods

        _ => {
            // Finally tries to use fn_index
            interpret_call(symbols, current_scope, method_name, *fn_index, &all_args)
        }
    }
}
```

**This is the problem!** The interpreter:
- Ignores the symbol table lookup result (`fn_index`)
- Hard-codes every built-in method with string matching
- Can't support user-defined methods on built-in types
- Duplicates type information already in symbol table

---

## 🏗️ Proposed Solution

### Design: Built-in Method Registry

**Add a trait/enum to mark and dispatch built-in methods:**

```rust
// In src/syntax.rs or new src/builtins.rs
pub enum BuiltinMethod {
    // String methods
    StrUpper,
    StrLower,
    StrSubstring,
    StrContains,
    StrTrim,
    StrSplit,
    StrReplace,
    StrStartsWith,
    StrEndsWith,
    StrIsEmpty,

    // List methods
    ListFirst,
    ListLast,
    ListContains,
    ListSlice,
    ListReverse,
    ListJoin,
    ListIsEmpty,

    // Map methods
    MapKeys,
    MapValues,
    MapContainsKey,
    MapIsEmpty,
}

impl BuiltinMethod {
    /// Execute this built-in method with given receiver and arguments
    pub fn execute(
        &self,
        receiver: Expr,
        args: &[Expr],
    ) -> Result<Expr, Box<dyn Error>> {
        match self {
            BuiltinMethod::StrUpper => {
                // Validate receiver is string
                match receiver {
                    Expr::RuntimeData(LiteralData::Str(ref s)) => {
                        Ok(Expr::RuntimeData(LiteralData::Str(s.to_uppercase().into())))
                    }
                    _ => Err("upper() requires Str receiver".into())
                }
            }
            // ... other methods
        }
    }
}
```

### Updated Symbol Table

```rust
// In Function or Lambda
pub struct Function {
    pub params: Vec<Param>,
    pub return_type: DataType,
    pub body: Box<Expr>,
    pub receiver_type: Option<String>,
    pub builtin: Option<BuiltinMethod>,  // NEW: Mark as built-in
}
```

### Updated Interpreter

```rust
Expr::MethodCall { receiver, method_name, fn_index, args } => {
    // Evaluate receiver
    let receiver_value = receiver.interpret(symbols, current_scope)?;

    // Look up method from symbol table (use the fn_index we already have!)
    if let Some(fn_expr) = symbols.get_symbol_value(fn_index) {
        match fn_expr {
            Expr::DefineFunction { value, .. } => {
                match value.as_ref() {
                    Expr::Lambda { value: func, .. } => {
                        // Check if it's a built-in
                        if let Some(builtin) = &func.builtin {
                            // Execute built-in generically
                            builtin.execute(receiver_value, args)
                        } else {
                            // User-defined method - execute body
                            interpret_call(symbols, current_scope, method_name, *fn_index, &all_args)
                        }
                    }
                }
            }
        }
    }
}
```

---

## 🎨 Benefits of Refactor

### 1. **Generic Dispatch**
- Methods dispatched based on receiver type from symbol table
- No hard-coded method names in interpreter
- Same code path for built-in and user-defined methods

### 2. **User-Extensible**
Users can add methods to built-in types:

```lift
// User defines a new method on Str
function Str.shout(): Str {
    self.upper() + '!!!'
}

'hello'.shout()  // Works! Uses same dispatch as built-ins
```

### 3. **Maintainable**
- Adding a new built-in method: Just add to `BuiltinMethod` enum
- Built-in logic centralized in one place
- Type checking already handles everything correctly

### 4. **Consistent**
- Built-ins and user-defined methods use same mechanism
- Symbol table is source of truth
- No special cases in interpreter

---

## 📋 Refactoring Checklist

### Phase 1: Infrastructure
- [ ] Create `BuiltinMethod` enum with all current methods
- [ ] Implement `BuiltinMethod::execute()` method
- [ ] Add `builtin: Option<BuiltinMethod>` to `Function` struct
- [ ] Update `add_builtin_method` to mark functions as built-in

### Phase 2: Update Interpreter
- [ ] Remove hard-coded match statement on method names
- [ ] Use `fn_index` to look up method from symbol table
- [ ] Check if method is built-in using `builtin` field
- [ ] Dispatch to `builtin.execute()` or user function body
- [ ] Remove special-case handling

### Phase 3: Testing
- [ ] Verify all 17 built-in methods still work
- [ ] Test user-defined methods on built-in types
- [ ] Test method chaining
- [ ] Test UFCS
- [ ] Verify error messages

### Phase 4: Documentation
- [ ] Update architecture docs
- [ ] Document how to add new built-ins
- [ ] Document user extensibility

---

## 🔍 Current Issues Summary

**Hard-coded locations:**

1. **`src/interpreter.rs:189-540`** - Giant match on method names
   - Each method hard-coded with name check and receiver type check
   - Ignores symbol table information
   - Can't be extended by users

2. **Similar issue in UFCS handling** (`src/interpreter.rs:123-176`)
   - Also hard-codes method names for UFCS calls
   - Should use same registry

**Good locations (keep as-is):**

1. **`src/symboltable.rs:45-160`** - Method registration
   - Already associates methods with types correctly
   - Just needs to mark built-ins

2. **`src/semantic_analysis.rs:232-282`** - Method resolution
   - Already resolves methods by type correctly
   - Generic and extensible

---

## 🚀 Next Steps

1. **Discuss design** - Is `BuiltinMethod` enum the right approach?
2. **Prototype refactor** - Start with 2-3 methods
3. **Test thoroughly** - Ensure no regressions
4. **Complete refactor** - Move all methods to new system
5. **Document pattern** - Show how to add new built-ins

---

## 💡 Alternative Approaches

### Option A: BuiltinMethod Enum (Recommended)
- Explicit enum of all built-ins
- Type-safe and exhaustive
- Easy to add new built-ins
- Centralized logic

### Option B: Function Pointers
- Store Rust function pointer in `Function`
- More flexible but less type-safe
- Harder to serialize/debug

### Option C: Macro-based Registry
- Register built-ins with macros
- Reduces boilerplate
- More complex to understand

**Recommendation: Start with Option A (BuiltinMethod enum)**

---

## 📝 Notes

The core insight is: **We already have generic method resolution in the compiler (semantic analysis). We just need to make the interpreter use it instead of hard-coding everything.**

The symbol table has the right information. The compiler uses it correctly. Only the interpreter needs fixing.
