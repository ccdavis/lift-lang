# Struct/Record Implementation Plan

**Status**: Partially implemented (type definition only)
**Completion**: ~25% (type system skeleton exists)
**Date**: 2025-10-11

## Executive Summary

Structs are currently **75% unimplemented** in Lift. Only the type definition skeleton exists in the AST, with a **critical bug** that discards field definitions. All runtime operations (creation, field access, type checking, interpretation, compilation) are completely missing.

## Current State

### ✅ What Exists

1. **Type System Representation** (src/syntax.rs:71)
   ```rust
   pub enum DataType {
       // ...
       Struct(Vec<Param>),
       // ...
   }
   ```

2. **Grammar Definition** (src/grammar.lalrpop)
   ```lalrpop
   "type" <i:ident> "=" "struct" "(" <m:CommaSeparated<Param>> ")" =>
       Expr::DefineType{
           type_name: i,
           definition: DataType::Struct(Vec::new()),  // ❌ BUG: should be 'm'
           index: (0,0)
       }
   ```

3. **Param Structure** (src/syntax.rs:45)
   ```rust
   pub struct Param {
       pub name: String,
       pub data_type: DataType,
       pub default: Option<Expr>,
       pub index: (usize, usize),
       pub copy: bool,
   }
   ```

### ❌ Critical Missing Components

#### 1. Grammar Bug (MUST FIX FIRST)
**Location**: src/grammar.lalrpop line with struct definition

**Current (BROKEN)**:
```lalrpop
"type" <i:ident> "=" "struct" "(" <m:CommaSeparated<Param>> ")" =>
    Expr::DefineType{
        type_name: i,
        definition: DataType::Struct(Vec::new()),  // ❌ Params lost!
        index: (0,0)
    }
```

**Should Be**:
```lalrpop
"type" <i:ident> "=" "struct" "(" <m:CommaSeparated<Param>> ")" =>
    Expr::DefineType{
        type_name: i,
        definition: DataType::Struct(m),  // ✅ Use actual params
        index: (0,0)
    }
```

#### 2. No Struct Literals
**Missing**: Cannot create struct instances

**Needed**:
```rust
// In Expr enum (src/syntax.rs):
StructLiteral {
    type_name: String,
    fields: Vec<(String, Expr)>,  // field_name -> value expression
}

RuntimeStruct {
    type_name: String,
    fields: HashMap<String, Expr>,  // or Vec<Expr> for efficiency
}
```

**Example syntax desired**:
```lift
type Person = struct (name: Str, age: Int)
let alice = Person { name: 'Alice', age: 30 }
```

#### 3. No Field Access
**Missing**: Cannot read struct fields

**Needed**:
```rust
// In Expr enum (src/syntax.rs):
FieldAccess {
    expr: Box<Expr>,
    field_name: String,
}
```

**Example syntax desired**:
```lift
let name = alice.name      // Get field value
let age = alice.age        // Get another field
```

#### 4. No Type Checking Support
**Missing**: src/semantic_analysis.rs has no struct handling

**Needed**:
- Struct literal type checking
- Field existence verification
- Field type matching
- Field access type inference

#### 5. No Interpreter Support
**Missing**: src/interpreter.rs has no struct cases

**Needed**:
- Struct literal interpretation
- Field access interpretation
- Struct value storage

#### 6. No Compiler Support
**Missing**: src/codegen.rs has no struct compilation

**Needed**:
- Struct layout in memory
- Field offset calculations
- Struct literal compilation
- Field access compilation

## Implementation Plan

### Phase 1: Fix Critical Bug (15 minutes)
**Priority**: CRITICAL - Do this before anything else

1. **Fix Grammar** (src/grammar.lalrpop)
   ```lalrpop
   "type" <i:ident> "=" "struct" "(" <m:CommaSeparated<Param>> ")" =>
       Expr::DefineType{
           type_name: i,
           definition: DataType::Struct(m),  // ✅ Pass params
           index: (0,0)
       }
   ```

2. **Add Test**
   ```lift
   type Person = struct (name: Str, age: Int)
   // Verify params are stored correctly
   ```

### Phase 2: Add Struct Literals (2-3 hours)

#### Step 1: Extend AST (src/syntax.rs)
```rust
pub enum Expr {
    // ... existing variants ...

    StructLiteral {
        type_name: String,
        fields: Vec<(String, Expr)>,
    },

    RuntimeStruct {
        type_name: String,
        fields: HashMap<String, Expr>,  // or Vec for efficiency later
    },
}
```

#### Step 2: Add Grammar Rules (src/grammar.lalrpop)
```lalrpop
// Add to expression hierarchy (probably at ExprPrimary level)
ExprStructLiteral: Expr = {
    <type_name:ident> "{" <fields:CommaSeparated<StructField>> "}" => {
        Expr::StructLiteral {
            type_name,
            fields
        }
    }
};

StructField: (String, Expr) = {
    <name:ident> ":" <value:ProgramPartExpr> => (name, value)
};
```

#### Step 3: Type Checking (src/semantic_analysis.rs)
```rust
// In typecheck() function:
Expr::StructLiteral { type_name, fields } => {
    // 1. Look up struct definition in symbol table
    let struct_def = symbols.get_type_definition(type_name)?;

    // 2. Verify it's actually a struct
    let DataType::Struct(params) = struct_def else {
        return Err(format!("{} is not a struct type", type_name));
    };

    // 3. Create a map of expected fields
    let expected_fields: HashMap<String, DataType> = params
        .iter()
        .map(|p| (p.name.clone(), p.data_type.clone()))
        .collect();

    // 4. Check all required fields are present
    for param in params {
        if !fields.iter().any(|(name, _)| name == &param.name) {
            return Err(format!("Missing field '{}' in struct literal", param.name));
        }
    }

    // 5. Check no extra fields
    for (field_name, _) in fields {
        if !expected_fields.contains_key(field_name) {
            return Err(format!("Unknown field '{}' for struct {}", field_name, type_name));
        }
    }

    // 6. Type check each field value
    for (field_name, field_expr) in fields {
        let expected_type = &expected_fields[field_name];
        typecheck(field_expr, symbols, current_scope)?;
        let actual_type = determine_type_with_symbols(field_expr, symbols, current_scope)?;

        if !types_compatible(expected_type, &actual_type) {
            return Err(format!(
                "Field '{}' expects type {:?}, got {:?}",
                field_name, expected_type, actual_type
            ));
        }
    }

    Ok(())
}
```

#### Step 4: Interpreter (src/interpreter.rs)
```rust
// In interpret() method:
Expr::StructLiteral { type_name, fields } => {
    // Evaluate each field expression
    let mut field_map = HashMap::new();

    for (field_name, field_expr) in fields {
        let field_value = field_expr.interpret(symbols, current_scope)?;
        field_map.insert(field_name.clone(), field_value);
    }

    Ok(Expr::RuntimeStruct {
        type_name: type_name.clone(),
        fields: field_map,
    })
}

Expr::RuntimeStruct { .. } => Ok(self.clone()),
```

#### Step 5: Display Implementation (src/syntax.rs)
```rust
// In Display trait for Expr:
Expr::StructLiteral { type_name, fields } => {
    write!(f, "{} {{ ", type_name)?;
    for (i, (name, value)) in fields.iter().enumerate() {
        if i > 0 { write!(f, ", ")?; }
        write!(f, "{}: {}", name, value)?;
    }
    write!(f, " }}")
}

Expr::RuntimeStruct { type_name, fields } => {
    write!(f, "{} {{ ", type_name)?;
    let mut sorted_fields: Vec<_> = fields.iter().collect();
    sorted_fields.sort_by_key(|(name, _)| *name);

    for (i, (name, value)) in sorted_fields.iter().enumerate() {
        if i > 0 { write!(f, ", ")?; }
        write!(f, "{}: {}", name, value)?;
    }
    write!(f, " }}")
}
```

#### Step 6: Add Tests
```lift
// tests/test_struct_literals.lt
type Person = struct (name: Str, age: Int)
type Point = struct (x: Int, y: Int)

// Basic struct literal
let alice = Person { name: 'Alice', age: 30 }
output(alice)  // Should output: Person { name: 'Alice', age: 30 }

// Order shouldn't matter
let bob = Person { age: 25, name: 'Bob' }
output(bob)

// Nested structs
type Rectangle = struct (top_left: Point, bottom_right: Point)
let rect = Rectangle {
    top_left: Point { x: 0, y: 10 },
    bottom_right: Point { x: 10, y: 0 }
}

// Should fail: missing field
// let bad = Person { name: 'Charlie' }  // Error: missing 'age'

// Should fail: wrong type
// let bad2 = Person { name: 'Diana', age: 'thirty' }  // Error: age must be Int

// Should fail: extra field
// let bad3 = Person { name: 'Eve', age: 20, height: 180 }  // Error: unknown field 'height'
```

### Phase 3: Add Field Access (2-3 hours)

#### Step 1: Extend AST (src/syntax.rs)
```rust
pub enum Expr {
    // ... existing variants ...

    FieldAccess {
        expr: Box<Expr>,
        field_name: String,
    },
}
```

#### Step 2: Grammar Rules (src/grammar.lalrpop)
```lalrpop
// Modify expression precedence to include field access
// Should be at same level as method calls

ExprFieldOrMethod: Expr = {
    <expr:ExprFieldOrMethod> "." <field:ident> => {
        // This is tricky: need to distinguish between:
        // - Field access: obj.field
        // - Method call: obj.method(...)

        // Look ahead for '(' to determine if it's a method call
        // If next token is '(', it's handled by method call rule
        // Otherwise, it's field access
        Expr::FieldAccess {
            expr: Box::new(expr),
            field_name: field,
        }
    },

    // Method call rule (already exists, but may need adjustment)
    <receiver:ExprFieldOrMethod> "." <method:ident> "(" <args:CommaSeparated<KeywordArg>> ")" => {
        Expr::MethodCall {
            receiver: Box::new(receiver),
            method_name: method,
            fn_index: (0, 0),
            args,
        }
    },

    ExprPrimary,
};
```

**Note**: Grammar precedence is tricky. May need to refactor the dot operator handling to distinguish between field access and method calls based on lookahead.

#### Step 3: Type Checking (src/semantic_analysis.rs)
```rust
// In typecheck():
Expr::FieldAccess { expr, field_name } => {
    // Type check the expression
    typecheck(expr, symbols, current_scope)?;

    // Get the type of the expression
    let expr_type = determine_type_with_symbols(expr, symbols, current_scope)?;

    // Verify it's a struct
    match expr_type {
        DataType::TypeRef(type_name) => {
            // Look up the actual type definition
            let actual_type = symbols.get_type_definition(&type_name)?;

            match actual_type {
                DataType::Struct(params) => {
                    // Verify field exists
                    if !params.iter().any(|p| p.name == *field_name) {
                        return Err(format!(
                            "Struct '{}' has no field '{}'",
                            type_name, field_name
                        ));
                    }
                    Ok(())
                }
                _ => Err(format!("Cannot access field '{}' on non-struct type", field_name))
            }
        }
        DataType::Struct(params) => {
            // Direct struct type
            if !params.iter().any(|p| p.name == *field_name) {
                return Err(format!("Struct has no field '{}'", field_name));
            }
            Ok(())
        }
        _ => Err(format!("Cannot access field '{}' on non-struct type", field_name))
    }
}

// In determine_type_with_symbols():
Expr::FieldAccess { expr, field_name } => {
    let expr_type = determine_type_with_symbols(expr, symbols, current_scope)?;

    match expr_type {
        DataType::TypeRef(type_name) => {
            let actual_type = symbols.get_type_definition(&type_name)?;
            if let DataType::Struct(params) = actual_type {
                // Find the field and return its type
                params.iter()
                    .find(|p| p.name == *field_name)
                    .map(|p| p.data_type.clone())
                    .ok_or_else(|| format!("Field '{}' not found", field_name))
            } else {
                Err(format!("Not a struct type"))
            }
        }
        DataType::Struct(params) => {
            params.iter()
                .find(|p| p.name == *field_name)
                .map(|p| p.data_type.clone())
                .ok_or_else(|| format!("Field '{}' not found", field_name))
        }
        _ => Err(format!("Cannot access field on non-struct"))
    }
}
```

#### Step 4: Interpreter (src/interpreter.rs)
```rust
// In interpret():
Expr::FieldAccess { expr, field_name } => {
    // Evaluate the expression to get the struct
    let struct_value = expr.interpret(symbols, current_scope)?;

    match struct_value {
        Expr::RuntimeStruct { fields, .. } => {
            // Look up the field
            fields.get(field_name)
                .cloned()
                .ok_or_else(|| Box::new(RuntimeError::new(
                    &format!("Field '{}' not found in struct", field_name),
                    None,
                    None,
                )) as Box<dyn Error>)
        }
        _ => Err(Box::new(RuntimeError::new(
            &format!("Cannot access field '{}' on non-struct value", field_name),
            None,
            None,
        )))
    }
}
```

#### Step 5: Display (src/syntax.rs)
```rust
// In Display for Expr:
Expr::FieldAccess { expr, field_name } => {
    write!(f, "{}.{}", expr, field_name)
}
```

#### Step 6: Add Tests
```lift
// tests/test_struct_field_access.lt
type Person = struct (name: Str, age: Int)
type Point = struct (x: Int, y: Int)

// Basic field access
let alice = Person { name: 'Alice', age: 30 }
let name = alice.name
let age = alice.age
output(name)  // 'Alice'
output(age)   // 30

// Field access in expressions
let is_adult = alice.age >= 18
output(is_adult)  // true

// Nested field access
type Rectangle = struct (top_left: Point, bottom_right: Point)
let rect = Rectangle {
    top_left: Point { x: 0, y: 10 },
    bottom_right: Point { x: 10, y: 0 }
}

let x1 = rect.top_left.x
let y1 = rect.top_left.y
output(x1)  // 0
output(y1)  // 10

// Field access in function calls
function greet(name: Str): Str {
    'Hello, ' + name
}
output(greet(name: alice.name))  // 'Hello, Alice'

// Should fail: wrong field
// let bad = alice.height  // Error: Person has no field 'height'
```

### Phase 4: Optimization (Optional - 4-6 hours)

#### Replace HashMap with Vec
**Rationale**: HashMap lookup is O(log n) or O(n). Array indexing is O(1).

**Changes needed**:
1. At semantic analysis, assign each field a numeric index
2. Store fields as `Vec<Expr>` instead of `HashMap<String, Expr>`
3. Compile field access to array index

```rust
// More efficient runtime representation:
RuntimeStruct {
    type_name: String,
    fields: Vec<Expr>,  // Ordered by field definition
}

// Field access becomes:
Expr::FieldAccess {
    expr: Box<Expr>,
    field_name: String,
    field_index: usize,  // Resolved at type-check time
}
```

### Phase 5: Compiler Support (8-12 hours)

**Prerequisites**: Phases 1-3 complete

#### Step 1: Struct Layout (src/codegen.rs)
```rust
// Determine memory layout for structs
fn compute_struct_layout(params: &[Param]) -> StructLayout {
    // Calculate:
    // - Total size
    // - Field offsets
    // - Alignment requirements
}

struct StructLayout {
    size: u32,
    field_offsets: Vec<u32>,
}
```

#### Step 2: Struct Literal Compilation
```rust
// In compile_expr():
Expr::StructLiteral { type_name, fields } => {
    // 1. Allocate memory for struct (stack or heap)
    // 2. For each field:
    //    - Compile field expression
    //    - Store at correct offset
    // 3. Return pointer to struct
}
```

#### Step 3: Field Access Compilation
```rust
// In compile_expr():
Expr::FieldAccess { expr, field_name } => {
    // 1. Compile expression to get struct pointer
    // 2. Calculate field offset
    // 3. Load value from [pointer + offset]
}
```

## Design Considerations

### 1. Field Access Performance

**Current Plan**: HashMap lookup O(log n)
**Better**: Compile-time index resolution O(1)

**Recommendation**: Use Vec with indices computed during type checking.

### 2. Memory Layout

**Options**:
- **Stack allocation**: Good for small structs
- **Heap allocation**: Needed for large structs or recursive types
- **Hybrid**: Small structs on stack, large ones on heap

**Recommendation**: Start with heap allocation (easier), optimize later.

### 3. Value vs Reference Semantics

**Questions**:
- Are structs copied on assignment?
- Does `cpy` parameter work with structs?
- How do struct parameters work?

**Recommendation**: Follow Rust-like semantics:
- Structs are value types (copied)
- Use `cpy` for mutable struct parameters
- Consider adding `&` for borrowing later

### 4. Nested Structs

**Challenge**: Field access chaining `person.address.city`

**Implementation**: Recursive field access in interpreter/compiler

### 5. Methods on Structs

**Status**: Existing method system should work!

**Example**:
```lift
type Person = struct (name: Str, age: Int)

function Person.greet(): Str {
    'Hello, my name is ' + self.name
}

let alice = Person { name: 'Alice', age: 30 }
output(alice.greet())  // Works with existing method system!
```

### 6. Pattern Matching

**Future work**: Struct destructuring in match expressions
```lift
match person {
    Person { name, age } if age >= 18 => 'Adult: ' + name,
    Person { name, age } => 'Minor: ' + name
}
```

## Testing Strategy

### Unit Tests (src/main.rs)

```rust
#[test]
fn test_struct_definition() {
    // Test that struct type definition works
}

#[test]
fn test_struct_literal_basic() {
    // Test creating simple struct instances
}

#[test]
fn test_struct_field_access() {
    // Test reading fields
}

#[test]
fn test_struct_nested() {
    // Test structs containing structs
}

#[test]
fn test_struct_type_checking() {
    // Test type errors are caught
}

#[test]
fn test_struct_in_collections() {
    // Test List of Struct, Map with struct values
}

#[test]
fn test_struct_methods() {
    // Test methods on user-defined structs
}
```

### Integration Tests

```lift
// tests/test_struct_comprehensive.lt
// Comprehensive struct usage test
```

## Known Issues & Limitations

### Current Limitations

1. **No field mutation**: Cannot update struct fields after creation
2. **No struct comparison**: Cannot use `=` or `<>` on structs
3. **No struct copying**: No explicit copy mechanism
4. **No default values**: Cannot specify default field values (Param has `default` field but unused)
5. **No optional fields**: All fields are required
6. **No struct inheritance**: No subtyping or composition patterns
7. **No generic structs**: Cannot do `type Pair<T> = struct (first: T, second: T)`

### Future Enhancements

1. **Mutable field update**:
   ```lift
   alice.age := 31  // Requires mutable struct
   ```

2. **Struct comparison**:
   ```lift
   alice = bob  // Compare all fields
   ```

3. **Struct spreading**:
   ```lift
   let alice2 = Person { ...alice, age: 31 }
   ```

4. **Optional fields**:
   ```lift
   type Person = struct (name: Str, age: Int, nickname: Str?)
   ```

5. **Default values**:
   ```lift
   type Config = struct (timeout: Int = 30, retries: Int = 3)
   ```

6. **Generic structs**:
   ```lift
   type Pair<T> = struct (first: T, second: T)
   let int_pair = Pair<Int> { first: 1, second: 2 }
   ```

## Comparison with Alternative Approaches

### Using Maps Instead

**Current workaround**:
```lift
let person = #{'name': 'Alice', 'age': 30}
let name = person['name']
```

**Problems**:
- No type safety on keys
- No compile-time field verification
- No dot syntax
- String keys are overhead
- No methods on pseudo-structs

**Verdict**: Maps are not a substitute for structs.

### Using Tuples

**Not supported yet**, but would have issues:
- Positional access only (no named fields)
- Hard to maintain when adding fields
- Less readable

## References

- **Type System**: src/syntax.rs lines 54-73
- **Grammar**: src/grammar.lalrpop (search for "struct")
- **Type Checking**: src/semantic_analysis.rs
- **Interpreter**: src/interpreter.rs
- **Compiler**: src/codegen.rs
- **Runtime**: src/runtime.rs

## Estimated Effort

- **Phase 1 (Fix bug)**: 15 minutes
- **Phase 2 (Struct literals)**: 2-3 hours
- **Phase 3 (Field access)**: 2-3 hours
- **Phase 4 (Optimization)**: 4-6 hours (optional)
- **Phase 5 (Compiler)**: 8-12 hours

**Total for interpreter support**: 4-6 hours
**Total with compiler**: 12-18 hours
**Total with optimization**: 16-24 hours

## Next Steps

When picking this up:

1. **Start here**: Phase 1 - Fix the grammar bug
2. **Then**: Phase 2 - Add struct literals
3. **Then**: Phase 3 - Add field access
4. **Test thoroughly** after each phase
5. **Document** as you go

Good luck! The analysis is solid, the plan is clear, and the implementation should be straightforward.
