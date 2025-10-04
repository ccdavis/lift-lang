# User-Defined Methods on Type Aliases

## Overview

Lift now supports defining custom methods on type aliases. This allows you to extend primitive and composite types with domain-specific behavior while maintaining type safety.

## Features

### ✅ Custom Methods on Type Aliases
Define methods that are associated with your type alias, not the underlying type:

```lift
type Temperature = Flt;

function Temperature.to_celsius(): Temperature {
    (self - 32.0) / 1.8
};

let temp: Temperature = 98.6;
output(temp.to_celsius());  // Works!
```

### ✅ Type Alias Inheritance of Built-in Methods
Type aliases automatically inherit all built-in methods from their underlying type:

```lift
type PersonName = Str;

let name: PersonName = 'alice';
output(name.upper());  // 'ALICE' - uses built-in upper() method
```

### ✅ Method Chaining
Combine custom and built-in methods:

```lift
type Message = Str;

function Message.exclaim(): Message {
    self + '!'
};

let msg: Message = 'hello';
output(msg.exclaim().upper());  // 'HELLO!'
```

### ✅ Type Safety
Methods are properly type-checked and associated with the specific type alias:

```lift
type Celsius = Flt;
type Fahrenheit = Flt;

function Celsius.to_fahrenheit(): Fahrenheit {
    self * 1.8 + 32.0
};

// Only works on Celsius, not on regular Flt or Fahrenheit
let temp_c: Celsius = 20.0;
output(temp_c.to_fahrenheit());  // ✓ Works

let temp_f: Fahrenheit = 68.0;
// temp_f.to_fahrenheit()  // ✗ Error: method not found for Fahrenheit
```

## How It Works

### Method Resolution Order

1. **Check for method on type alias**: If the receiver is a type alias (e.g., `Temperature`), first look for methods defined on that specific alias
2. **Fallback to underlying type**: If not found, look for methods on the underlying type (e.g., `Flt`)
3. **Built-in methods**: Built-in methods are always available through the underlying type

### Type Preservation

- Type aliases are preserved throughout the type system
- Method parameters maintain their TypeRef association
- This allows methods to be properly associated with the alias, not the base type

## Examples

### Temperature Conversion

```lift
type Celsius = Flt;
type Fahrenheit = Flt;

function Celsius.to_fahrenheit(): Fahrenheit {
    self * 1.8 + 32.0
};

function Fahrenheit.to_celsius(): Celsius {
    (self - 32.0) / 1.8
};

let temp_c: Celsius = 20.0;
let temp_f: Fahrenheit = temp_c.to_fahrenheit();
output('20C =', temp_f, 'F');
```

### Domain-Specific Methods

```lift
type Age = Int;

function Age.is_adult(): Bool {
    self >= 18
};

function Age.is_senior(): Bool {
    self >= 65
};

let age: Age = 25;
if age.is_adult() {
    output('You can vote!');
};
```

### String Enhancement

```lift
type PersonName = Str;

function PersonName.formal_greeting(): Str {
    'Dear ' + self + ','
};

function PersonName.casual_greeting(): Str {
    'Hey ' + self + '!'
};

let name: PersonName = 'Alice';
output(name.formal_greeting());  // 'Dear Alice,'
output(name.casual_greeting());  // 'Hey Alice!'
```

## Test Files

- `tests/test_type_alias_comprehensive.lt` - Full demonstration of all features
- `tests/test_custom_type_alias_methods.lt` - Custom methods on various type aliases
- `tests/test_type_alias_methods_simple.lt` - Built-in methods on type aliases

## Implementation Details

### Key Changes

1. **Type Preservation in Parameters** (src/semantic_analysis.rs:396-401)
   - Parameter types are validated but not resolved
   - TypeRef associations are preserved
   - Enables proper method-to-alias binding

2. **Method Resolution** (src/semantic_analysis.rs:258-335)
   - First tries to find method on type alias name
   - Falls back to underlying type if not found
   - Supports both custom and built-in methods

3. **Type Checking** (src/semantic_analysis.rs:1022-1160)
   - Resolves TypeRefs only for compatibility checking
   - Preserves original types for error messages
   - Handles both TypeRef and resolved type comparisons

4. **Generic Method Dispatch** (src/interpreter.rs:129-178)
   - Uses BuiltinMethod enum for all built-in methods
   - No hard-coded method names in interpreter
   - Extensible for future built-in methods

## Limitations

### Current
- Collection type aliases (e.g., `type Names = List of Str`) work but have some edge cases with methods
- Methods return the declared type, which may be the alias or the underlying type depending on the signature

### Future Enhancements
- User-defined structs with custom methods
- Methods on collection type aliases with full support
- Method overloading based on parameter types
- Generic methods with type parameters

## Compatibility

- ✅ All existing built-in methods work unchanged
- ✅ Method syntax (`obj.method()`) fully supported
- ✅ UFCS syntax (`method(self: obj)`) fully supported
- ✅ Type inference works with type aliases
- ✅ Backward compatible with existing code
