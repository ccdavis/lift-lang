# Mutability in Lift

## Overview

Lift now supports both immutable and mutable variables with explicit syntax:

- **`let`** - Declares an immutable variable (default, recommended)
- **`let var`** - Declares a mutable variable (use when reassignment is needed)

## Syntax

### Immutable Variables (let)

```lift
let x = 5;              // Immutable, type inferred
let y: Int = 10;        // Immutable with type annotation
let name = 'Alice';     // Immutable string

// x := 10;             // COMPILE ERROR: Cannot assign to immutable variable
```

### Mutable Variables (let var)

```lift
let var count = 0;      // Mutable, type inferred
count := count + 1;     // OK: Assignment allowed
count := 5;             // OK: Reassignment works

let var message: Str = 'Hello';  // Mutable with type annotation
message := 'World';     // OK: Can reassign
```

## Assignment Operator (`:=`)

The assignment operator `:=` is used to reassign mutable variables:

- **Only works on `let var` variables**
- **Compile-time error** if used on immutable (`let`) variables
- Returns `Unit` (no value, expression-based)
- Type-checked for compatibility

```lift
let var x = 5;
x := 10;              // OK
x := x + 5;           // OK: Can use in expressions

let y = 5;
// y := 10;           // ERROR: Cannot assign to immutable variable 'y'
```

## Examples

### Basic Usage

```lift
// Immutable - value never changes
let pi = 3.14159;
output(pi);

// Mutable - value can change
let var counter = 0;
counter := counter + 1;
output(counter);  // 1
counter := counter + 1;
output(counter);  // 2
```

### With Type Annotations

```lift
let var score: Int = 0;
score := score + 10;
score := score + 5;
output(score);  // 15
```

### Scope Rules

```lift
let var outer = 1;
{
    output(outer);     // 1
    outer := 2;        // Can modify outer scope mutable variable
    output(outer);     // 2
}
output(outer);         // 2 - change persists
```

### With Collections

```lift
let var numbers = [1, 2, 3];
output(len(numbers));   // 3
numbers := [4, 5, 6, 7];
output(len(numbers));   // 4
```

## Error Messages

### Assigning to Immutable Variable

```lift
let x = 5;
x := 10;
```

**Error:** `Type check Error: Cannot assign to immutable variable 'x'. Use 'let var' to declare mutable variables.`

### Type Mismatch

```lift
let var x: Int = 5;
x := 'hello';
```

**Error:** `Type check Error: Cannot assign Str to variable x of type Int`

## Best Practices

1. **Default to immutable (`let`)**: Use `let` by default for all variables
2. **Use `let var` only when needed**: Only use mutable variables when you actually need to reassign
3. **Minimize mutation**: Prefer creating new values over mutating existing ones
4. **Clear naming**: Use descriptive names for mutable variables (e.g., `counter`, `accumulator`)

## Design Rationale

- **Safety**: Immutability by default prevents accidental mutations
- **Clarity**: Explicit `var` keyword makes mutation intent clear
- **Compile-time checking**: Mutability errors caught during compilation, not runtime
- **Expression-based**: Assignment is an expression returning `Unit`, consistent with language design

## Test Files

- `tests/test_mutability.lt` - Comprehensive mutability examples
- `tests/demo_let_vs_let_var.lt` - Comparison of let vs let var
- `tests/test_assignment.lt` - Assignment expression tests
- `tests/test_immutable_error.lt` - Immutability error demonstration
