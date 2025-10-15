# Lift Language Feature Analysis & Recommendations

## Executive Summary

Analysis of Lift language reveals several high-value, easy-to-implement features that would significantly improve usability. The top recommendations are:
1. **Built-in utility functions** (EASIEST, HIGH VALUE)
2. **For loops with ranges/lists** (MEDIUM, VERY HIGH VALUE)
3. **List/Map manipulation functions** (EASY, HIGH VALUE)
4. **Break/Continue statements** (MEDIUM, HIGH VALUE)
5. **Better output formatting** (EASIEST, HIGH VALUE)

---

## Current Language State

### ✅ Fully Implemented Features

**Core Language**:
- Variables with mutability control (`let`, `let var`)
- Functions with named parameters and `cpy` modifier
- Static typing with type inference
- Type aliases (`type Age = Int`)
- Assignment expressions (`:=`)
- All primitive types: Int, Flt, Str, Bool, Unit
- Collections: List, Map, Range
- Operators: arithmetic, comparison, logical, range (`..`)
- Control flow: if/else/else if, while loops
- Comments (single `//` and multi-line `/* */`)
- List/Map literals and indexing
- Built-in functions: `output()`, `len()`

**Type System**:
- Full type checking implemented
- Type inference from literals and expressions
- Compile-time mutability checking
- Parameter immutability by default

### ⚠️ Partially Implemented Features

1. **Match Expressions**
   - AST node exists (`Expr::Match`)
   - No grammar implementation
   - No type checking (returns `Unsolved`)
   - No interpreter implementation
   - **Status**: Skeleton only

2. **Return Statements**
   - AST node exists (`Expr::Return`)
   - Type checking exists
   - Symbol resolution exists
   - **Missing**: Interpreter control flow handling
   - **Status**: ~70% complete

3. **User-Defined Types**
   - Type aliases work (`type Age = Int`)
   - Range constraints syntax parsed but not enforced
   - Struct/Enum syntax parsed but not functional
   - **Status**: Aliases only, ~30% complete

### ❌ Missing Features

**Control Flow**:
- No `for` loops (critical gap)
- No `break` statement
- No `continue` statement
- While loops don't return values

**Standard Library**:
- Only 2 functions: `output()`, `len()`
- No type conversion functions
- No string manipulation
- No collection manipulation
- No math functions
- No I/O beyond `output()`

**Language Features**:
- No module/import system
- No automatic type coercion (Int ↔ Flt)
- No method syntax (obj.method())
- No operator overloading
- No generics (though type system could support)

---

## Recommended Additions (By Priority)

### 🟢 TIER 1: Easiest & Highest Value

#### 1. Built-in Utility Functions ⭐⭐⭐⭐⭐
**Difficulty**: Very Easy | **Value**: Very High | **Time**: 1-2 hours

Add these built-in functions following the `len()` pattern:

**Type Conversion**:
```lift
str(value: Any): Str        // Convert any value to string
int(value: Str): Int        // Parse string to int
flt(value: Str): Flt        // Parse string to float
```

**String Functions**:
```lift
substring(s: Str, start: Int, end: Int): Str
split(s: Str, delimiter: Str): List of Str
replace(s: Str, old: Str, new: Str): Str
upper(s: Str): Str
lower(s: Str): Str
trim(s: Str): Str
```

**Implementation**:
- Add new `Expr` variants or handle in `Len` pattern
- Add grammar rules
- Add interpreter cases
- Very straightforward, low risk

#### 2. Better Output Function ⭐⭐⭐⭐⭐
**Difficulty**: Very Easy | **Value**: High | **Time**: 30 minutes

```lift
print(value: Any)           // No quotes, no formatting
println(value: Any)         // With newline
```

**Current issue**: `output()` adds quotes around strings and spaces between items
**Fix**: Add `print()` and `println()` that don't add formatting

#### 3. Collection Manipulation Functions ⭐⭐⭐⭐
**Difficulty**: Easy | **Value**: Very High | **Time**: 2-3 hours

**List Functions**:
```lift
append(list: List of T, item: T): List of T
prepend(list: List of T, item: T): List of T
concat(list1: List of T, list2: List of T): List of T
slice(list: List of T, start: Int, end: Int): List of T
contains(list: List of T, item: T): Bool
first(list: List of T): T
last(list: List of T): T
```

**Map Functions**:
```lift
keys(map: Map of K to V): List of K
values(map: Map of K to V): List of V
has_key(map: Map of K to V, key: K): Bool
remove(map: Map of K to V, key: K): Map of K to V
```

**Implementation**: Similar to `len()`, straightforward pattern

---

### 🟡 TIER 2: Medium Difficulty, High Value

#### 4. For Loops ⭐⭐⭐⭐⭐
**Difficulty**: Medium | **Value**: Very High | **Time**: 3-4 hours

Most critical missing feature. Two variants needed:

**Range-based**:
```lift
for i in 1..10 {
    output(i)
}

for i in range {
    output(i)
}
```

**Collection-based**:
```lift
for item in mylist {
    output(item)
}

for key in keys(mymap) {
    output(key)
}
```

**Implementation**:
1. Add `For` variant to `Expr` enum
2. Add grammar rules (similar to `while`)
3. Add semantic analysis (create loop variable in new scope)
4. Add interpreter (iterate and evaluate body)

**Challenges**:
- Scoping for loop variable
- Iterator protocol (or just handle Range/List explicitly)

#### 5. Break and Continue Statements ⭐⭐⭐⭐
**Difficulty**: Medium | **Value**: High | **Time**: 2-3 hours

```lift
while condition {
    if done { break }
    if skip { continue }
    // ...
}

for i in 1..100 {
    if i > 50 { break }
    if i < 10 { continue }
    output(i)
}
```

**Implementation**:
1. Add `Break` and `Continue` to `Expr` enum
2. Add grammar rules
3. Modify interpreter to use Result/control flow
4. Propagate through while/for loops

**Challenges**:
- Need control flow mechanism (exceptions or Result type)
- Must track loop depth (no break outside loop)

#### 6. Complete Return Statement ⭐⭐⭐
**Difficulty**: Medium | **Value**: Medium-High | **Time**: 2-3 hours

Currently exists but not functional in interpreter.

**Implementation**:
1. Use same control flow mechanism as break/continue
2. Handle in function call evaluation
3. Early exit from nested blocks

---

### 🔵 TIER 3: Lower Priority / More Complex

#### 7. Range Enhancements ⭐⭐⭐
**Difficulty**: Easy | **Value**: Medium | **Time**: 1-2 hours

```lift
let r = 1..10 step 2        // Step ranges
let r = 10..1 step -1       // Reverse ranges
```

**Implementation**:
- Add `step` to Range type
- Update display, iteration

#### 8. Type Coercion ⭐⭐
**Difficulty**: Medium | **Value**: Medium | **Time**: 3-4 hours

Automatic Int ↔ Flt conversion in operations:
```lift
let x: Int = 5;
let y: Flt = 3.14;
let z = x + y;  // Currently error, could auto-convert to Flt
```

#### 9. Match Expressions ⭐⭐⭐
**Difficulty**: Hard | **Value**: Medium-High | **Time**: 6-8 hours

Complete the partially implemented match:
```lift
match value {
    0 => 'zero',
    1 => 'one',
    n => 'many'
}
```

**Implementation**:
- Complete grammar
- Pattern matching logic
- Exhaustiveness checking
- Type checking

#### 10. Struct Types ⭐⭐
**Difficulty**: Hard | **Value**: Medium | **Time**: 8-10 hours

Complete user-defined struct types:
```lift
type Person = struct {
    name: Str,
    age: Int
}

let p = Person { name: 'Alice', age: 30 }
```

---

## Implementation Roadmap

### Phase 1: Quick Wins (1 week)
1. Add print()/println() functions
2. Add basic utility functions (str, int, flt, substring, etc.)
3. Add list/map manipulation functions
4. Add range enhancements

**Impact**: Immediately usable for practical programs

### Phase 2: Control Flow (1 week)
1. Implement for loops (range and collection)
2. Implement break/continue
3. Complete return statement

**Impact**: Language becomes genuinely practical

### Phase 3: Advanced Features (2+ weeks)
1. Type coercion
2. Match expressions
3. Struct types
4. Module system (if needed)

---

## Detailed Implementation Guide: For Loops

Since for loops are highest value, here's a detailed plan:

### Syntax Design
```lift
// Range syntax
for i in 1..10 {
    output(i)
}

// List syntax
for item in [1, 2, 3] {
    output(item)
}

// Variable syntax
for x in mylist {
    output(x)
}
```

### AST Addition
```rust
Expr::For {
    var_name: String,
    var_index: (usize, usize),
    iterable: Box<Expr>,  // Range, List, or Variable
    body: Box<Expr>,
    environment: usize,
}
```

### Grammar
```lalrpop
ExprFor: Expr = "for" <v:ident> "in" <iter:ExprLogicOr> <body:ExprBlock> =>
    Expr::For {
        var_name: v,
        var_index: (0,0),
        iterable: Box::new(iter),
        body: Box::new(body),
        environment: 0
    }.into();
```

### Semantic Analysis
```rust
Expr::For { var_name, iterable, body, environment, var_index } => {
    // Check iterable is Range, List, or resolves to one
    add_symbols(iterable, symbols, _current_scope_id)?;

    // Create new scope for loop
    let loop_scope = symbols.create_scope(Some(_current_scope_id));
    *environment = loop_scope;

    // Determine loop variable type from iterable
    let iter_type = typecheck(iterable, symbols, _current_scope_id)?;
    let var_type = match iter_type {
        DataType::Range => DataType::Int,
        DataType::List { element_type } => *element_type,
        _ => return Err(...)
    };

    // Add loop variable to scope
    let loop_var = Expr::Let {
        var_name: var_name.clone(),
        data_type: var_type,
        value: Box::new(Expr::Unit),
        index: (0, 0),
        mutable: false,  // Loop vars are immutable
    };
    let idx = symbols.add_symbol(var_name, loop_var, loop_scope)?;
    *var_index = (loop_scope, idx);

    // Check body
    add_symbols(body, symbols, loop_scope)?;
}
```

### Interpreter
```rust
Expr::For { var_name, var_index, iterable, body, environment } => {
    let iter_value = iterable.interpret(symbols, current_scope)?;

    match iter_value {
        Expr::Range(start, end) => {
            // Extract int values
            let start_val = ...; // extract from LiteralData
            let end_val = ...;

            for i in start_val..=end_val {
                // Set loop var value
                let val = Expr::Literal(LiteralData::Int(i));
                symbols.update_runtime_value(val, var_index);

                // Execute body
                body.interpret(symbols, environment)?;
            }
        }
        Expr::RuntimeList { data, .. } => {
            for item in data {
                symbols.update_runtime_value(item, var_index);
                body.interpret(symbols, environment)?;
            }
        }
        _ => return Err(...)
    }

    Ok(Expr::Unit)
}
```

---

## Metrics

### Lines of Code Estimates
- Built-in functions: ~200 LOC
- For loops: ~300 LOC
- Break/Continue: ~250 LOC
- Collection functions: ~400 LOC
- Match expressions: ~800 LOC

### Risk Assessment
- **Low Risk**: Built-in functions, print(), collection functions
- **Medium Risk**: For loops, break/continue, return completion
- **High Risk**: Match expressions, struct types, type coercion

### Value/Effort Matrix

```
High Value │  Built-ins    For Loops
           │  Print()      Break/Cont
           │  Collections
           │
Medium Val │  Return       Match
           │  Range++      Structs
           │
Low Value  │  Type Coerce
           │
           └────────────────────────
              Easy    Medium    Hard
                    Effort
```

---

## Conclusion

**Immediate Recommendations** (1-2 days each):
1. ✅ Add print/println functions (30 min)
2. ✅ Add str/int/flt conversion functions (1 hour)
3. ✅ Add string manipulation functions (2 hours)
4. ✅ Add list/map manipulation functions (3 hours)

**Next Priority** (3-4 days):
5. ✅ Implement for loops (1 day)
6. ✅ Implement break/continue (1 day)
7. ✅ Complete return statement (1 day)

This would take Lift from a toy language to a genuinely usable scripting language in about 1 week of focused development.
