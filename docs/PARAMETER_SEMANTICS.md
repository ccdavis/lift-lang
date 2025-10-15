# Function Parameter Semantics in Lift

## Overview

Lift enforces immutability for function parameters by default, preparing for a compiled version with reference semantics. The `cpy` modifier enables pass-by-value behavior when mutation is needed.

## Parameter Types

### 1. Regular Parameters (Pass by Reference - Immutable)

**Default behavior:** Parameters are immutable inside the function.

```lift
function increment(x: Int): Int {
    // x := x + 1;    // ERROR: Cannot assign to immutable variable 'x'
    x + 1             // Must use expressions instead
};
```

**Rationale:**
- Prepares for compiled version with reference semantics
- Prevents accidental mutation of caller's data
- Encourages functional programming style
- More efficient (no copying) for large data structures

### 2. Copy Parameters (Pass by Value - Mutable)

**With `cpy` keyword:** Parameters are copied and mutable inside the function.

```lift
function increment(cpy x: Int): Int {
    x := x + 1;       // OK: x is a mutable copy
    x
};

let value = 5;
let result = increment(x: value);
output(value);        // 5 (original unchanged)
output(result);       // 6
```

**Key Points:**
- Use `cpy` keyword before parameter name
- Parameter is copied when function is called
- Can be modified inside function without affecting caller
- Original value is never changed

## Syntax

```lift
function name(param1: Type1, cpy param2: Type2, param3: Type3): ReturnType {
    // param1 is immutable
    // param2 is mutable (copied)
    // param3 is immutable
    ...
};
```

## Examples

### Basic Immutability

```lift
function double(x: Int): Int {
    // Cannot modify x
    x * 2
};
```

### Using cpy for Local Mutation

```lift
function accumulate(cpy total: Int, value: Int): Int {
    total := total + value;
    total := total * 2;
    total
};

let sum = accumulate(total: 10, value: 5);
output(sum);  // 30: (10 + 5) * 2
```

### Mixed Parameters

```lift
function process(base: Int, cpy accumulator: Int, multiplier: Int): Int {
    // base := base + 1;           // ERROR: immutable
    accumulator := accumulator + base;  // OK: cpy parameter
    // multiplier := multiplier * 2;    // ERROR: immutable
    accumulator * multiplier
};
```

### String Example

```lift
function append_suffix(cpy text: Str, suffix: Str): Str {
    text := text + suffix;   // OK: text is copied
    text
};

let original = 'Hello';
let modified = append_suffix(text: original, suffix: ' World');
output(original);  // 'Hello' (unchanged)
output(modified);  // 'Hello World'
```

### When NOT to Use cpy

```lift
// Don't use cpy if you don't need to modify the parameter
function calculate(cpy x: Int, cpy y: Int): Int {  // Unnecessary copies!
    x + y  // Never modify x or y, so cpy is wasteful
};

// Better:
function calculate(x: Int, y: Int): Int {
    x + y
};
```

## Error Messages

### Attempting to Modify Immutable Parameter

```lift
function bad(x: Int): Int {
    x := x + 1;
    x
};
```

**Error:** `Type check Error: Cannot assign to immutable variable 'x'. Use 'let var' to declare mutable variables.`

**Note:** The error message suggests `let var`, but for parameters, use `cpy` instead.

### Type Mismatch

```lift
function bad(cpy x: Int): Str {
    x := 'hello';  // Wrong type
    x
};
```

**Error:** `Type check Error: Cannot assign Str to variable x of type Int`

## Best Practices

1. **Default to Immutable**: Use regular parameters by default
2. **Use `cpy` Sparingly**: Only use when you need local mutation
3. **Consider Alternatives**: Often you can use expressions instead of mutation
4. **Performance**: `cpy` involves copying data; avoid for large structures unless necessary
5. **Clarity**: `cpy` makes mutation intent explicit

## Design Rationale

### Why Immutable by Default?

1. **Safety**: Prevents accidental modification of caller's data
2. **Reasoning**: Easier to reason about function behavior
3. **Compilation**: Prepares for reference semantics in compiled version
4. **Functional Style**: Encourages pure functions

### Why Provide `cpy`?

1. **Convenience**: Avoids manual copying in function body
2. **Clarity**: Makes intent explicit at function signature
3. **Flexibility**: Allows imperative style when needed
4. **Common Pattern**: Many algorithms naturally use accumulator patterns

## Future Considerations

When Lift gets a compiler:
- Regular parameters will use reference semantics (no copying)
- `cpy` parameters will continue to copy
- This design ensures current code works correctly when compiled

## Comparison with Other Languages

### Rust
```rust
fn increment(x: i32) -> i32 {  // Immutable by default
    x + 1
}

fn increment_mut(mut x: i32) -> i32 {  // Similar to cpy
    x += 1;
    x
}
```

### Swift
```swift
func increment(x: Int) -> Int {  // Immutable by default
    return x + 1
}

func increment(var x: Int) -> Int {  // Was similar to cpy (deprecated)
    x += 1
    return x
}
```

### Lift
```lift
function increment(x: Int): Int {  // Immutable by default
    x + 1
};

function increment(cpy x: Int): Int {  // Mutable copy with cpy
    x := x + 1;
    x
};
```

## Test Files

- `tests/test_cpy_params.lt` - Demonstrates cpy parameter usage
- `tests/test_immutable_params_error.lt` - Shows immutability errors
