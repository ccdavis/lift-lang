# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run the REPL
cargo run

# Run with a source file
cargo run -- test.lt

# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run interpreter tests
cargo test test_interpreter

# Run type checking tests
cargo test test_typecheck

# Format code
cargo fmt

# Run linter
cargo clippy

# Check code without building
cargo check
```

## Architecture Overview

Lift is a statically-typed, expression-based programming language that can be both interpreted and compiled. It uses a tree-walking interpreter and LALRPOP parser generator.

### Core Components

1. **Parser Pipeline**
   - `src/grammar.lalrpop`: LALRPOP grammar definition that generates the parser
   - `build.rs`: Build script that invokes LALRPOP to generate `grammar.rs`
   - Parser produces AST nodes defined in `syntax.rs`

2. **AST and Type System** (`src/syntax.rs`)
   - Central `Expr` enum defines all expression types
   - `DataType` enum represents the type system (Int, Flt, Str, Bool, List, Map, Range, Unsolved)
   - `LiteralData` for literal values
   - Operators defined with proper precedence

3. **Interpreter** (`src/interpreter.rs`)
   - Tree-walking interpreter that evaluates `Expr` nodes
   - `interpret()` method on `Expr` is the main entry point
   - Handles runtime values and operations
   - Variable scoping through environment passing
   - Supports logical operators (And, Or)
   - Handles Unit expressions

4. **Type Checking** (`src/semantic_analysis.rs`)
   - **FULLY IMPLEMENTED** - Type checking for all expression types
   - Strict type checking suitable for compiled language
   - Type inference from literal values
   - `typecheck()` performs semantic analysis
   - `determine_type()` for type inference
   - No Unsolved types allowed in operations

5. **Symbol Table** (`src/symboltable.rs`)
   - Manages variable and function definitions
   - Supports nested scopes with hierarchical structure
   - Stores compile-time and runtime values separately
   - Methods: `get_symbol_type()`, `get_symbol_value()`
   - Used during both type checking and interpretation

6. **REPL** (`src/main.rs`)
   - Interactive shell with syntax highlighting (rustyline)
   - Multi-line input support (use `\` for continuation)
   - Command history
   - Handles both REPL mode and file execution
   - Contains comprehensive test suite

7. **Compiler** (Cranelift JIT) (`src/codegen.rs`, `src/compiler.rs`, `src/runtime.rs`)
   - JIT compiler using Cranelift backend
   - Compiles AST to native x86-64 machine code
   - Runtime library with 21+ built-in method functions
   - Supports most language features (85%+ coverage)
   - Use `--compile` flag to run compiled mode

### Language Design Principles

#### Expression-Based
- Everything in Lift is an expression that returns a value
- Blocks return the value of their last expression
- Even control structures like `if` and `while` are expressions

#### Semicolons
- **IMPORTANT**: Semicolons are expression separators, NOT terminators
- The last expression in a block should NOT have a semicolon if you want the block to return that value
- Adding a semicolon after the last expression creates a Unit expression `()`

#### Type System
- Statically typed with type inference
- All types must be resolved at compile time (no Unsolved types in operations)
- Types can be inferred from literal values
- Explicit type annotations required when inference isn't possible
- Numeric types (Int/Flt) can be mixed in operations

### Compiler Status

The Cranelift JIT compiler supports **most Lift language features** with native x86-64 code generation:

#### ✅ Fully Supported
- **Primitives**: Int, Flt, Bool, Str
- **Collections**: List, Map, Range (including empty collections)
- **Operators**: All arithmetic, comparison, logical, range (`..`)
- **Control Flow**: if/else, else if, while loops
- **Variables**: let (immutable), let var (mutable), assignment (`:=`)
- **Functions**: User-defined functions with recursion
- **Parameters**: Regular (immutable) and `cpy` (mutable) parameters
- **Built-in Functions**: `output()`, `len()`
- **Built-in Methods**: All 21 methods
  - String: `upper()`, `lower()`, `substring()`, `contains()`, `trim()`, `split()`, `replace()`, `starts_with()`, `ends_with()`, `is_empty()`
  - List: `first()`, `last()`, `contains()`, `slice()`, `reverse()`, `join()`, `is_empty()`
  - Map: `keys()`, `values()`, `contains_key()`, `is_empty()`
- **Method Syntax**: Both dot notation (`obj.method()`) and UFCS (`method(self: obj)`)
- **User-Defined Methods**: Define methods on built-in types (`function Str.exclaim(): Str { self + '!' }`)
- **Method Chaining**: Mix built-in and user methods (`'hello'.upper().exclaim()`)

#### ❌ Not Yet Supported
- For loops (only while loops available)
- Match expressions
- Closures (functions cannot capture outer variables)
- User-defined types (structs, enums) - type aliases partially supported
- Module system (import/export)
- Typed collection output (lists/maps of strings show pointers instead of values)

#### 🚀 Usage

**Command Line**:
```bash
# Compile and run a Lift program
cargo run -- --compile your_file.lt

# Compare interpreter vs compiler
cargo run -- your_file.lt           # Interpreter
cargo run -- --compile your_file.lt # Compiler
```

**Running Tests**:
```bash
# All compiler unit tests (52 tests - 100% passing)
cargo test test_compile

# Specific feature tests
cargo test test_compile_function    # User functions
cargo test test_compile_str         # String methods
cargo test test_compile_list        # List methods
cargo test test_compile_map         # Map methods

# Integration tests (39/78 tests passing - 50% coverage)
./scripts/validate_compiler.sh
```

#### 📊 Performance

The JIT compiler provides:
- **Instant startup**: No separate compilation step
- **Native speed**: Code runs at native x86-64 speeds
- **Memory efficiency**: Direct stack/register allocation
- **Type safety**: Full compile-time type checking

Expected performance improvements over interpreter:
- Arithmetic: 10-50x faster
- Function calls: 5-20x faster
- Collection operations: 3-10x faster

#### 🔧 Implementation Details

**Architecture**:
- **Frontend**: LALRPOP parser → AST
- **Middle**: Type checking & symbol resolution
- **Backend**: Cranelift IR → JIT compilation

**Key Components**:
- `src/codegen.rs`: AST → Cranelift IR compilation (~1800 lines)
- `src/compiler.rs`: JIT module setup and execution (~1900 lines, 52 tests)
- `src/runtime.rs`: Runtime library (21+ method functions, ~800 lines)

**Type System**:
- Cranelift types: I64 (Int/Bool), F64 (Float), Pointer (Str/List/Map/Range)
- Automatic type conversions for operations
- Boolean methods return i8, auto-extended to i64

**Memory Model**:
- Stack allocation for primitives
- Heap allocation for collections (Box-wrapped)
- C-compatible strings (*const c_char)
- Note: Currently no GC (acceptable for short programs)

### Language Features

#### Data Types
- **Primitive**: Int, Flt (not Float!), Str, Bool, Unit
- **Composite**: List, Map, Range
- **Special**: Unsolved (used during type checking)

#### Variables

**Immutable Variables (default)**:
```lift
let x = 5;              // Type inferred as Int
let y: Int = 10;        // Explicit type annotation
let name = 'Alice';     // Strings use single quotes
// x := 10;             // ERROR: Cannot assign to immutable variable
```

**Mutable Variables**:
```lift
let var count = 0;      // Mutable variable
count := count + 1;     // Assignment allowed
count := 5;             // Can reassign

let var message: Str = 'Hello';  // With type annotation
message := 'World';     // Reassignment works
```

**Assignment Expressions** (`:=`):
- Only work on variables declared with `let var`
- Compile-time error if used on immutable (`let`) variables
- Return `Unit` (no value)
- Type-checked for compatibility

#### Functions

**Basic Functions**:
```lift
function add(x: Int, y: Int): Int {
    x + y              // No semicolon - returns the value
};

// Function calls use named arguments
add(x: 5, y: 3)
```

**Parameter Immutability**:
```lift
function process(x: Int): Int {
    // x := x + 1;     // ERROR: Parameters are immutable by default
    x + 1              // Must use expressions, not assignment
};
```

**Copy Parameters (`cpy`)**:
```lift
function increment(cpy x: Int): Int {
    x := x + 1;        // OK: cpy makes parameter mutable (pass by value)
    x
};

let value = 5;
let result = increment(x: value);
output(value);         // 5 (original unchanged)
output(result);        // 6

// Mixed parameters
function process(immutable: Int, cpy mutable: Int): Int {
    // immutable := 10;       // ERROR: immutable parameter
    mutable := mutable + 10;  // OK: cpy parameter
    immutable + mutable
};
```

**Parameter Semantics**:
- Regular parameters: Pass by reference (will be), immutable inside function
- `cpy` parameters: Pass by value (copied), mutable inside function
- Original values never affected, even with `cpy`

#### Control Flow

**If/Else/Else If** (Latest feature):
```lift
// Basic if-else
if condition { expr1 } else { expr2 }

// Else if chains
if score >= 90 {
    'A'
} else if score >= 80 {
    'B'
} else if score >= 70 {
    'C'
} else {
    'F'
}

// Optional else (returns Unit if condition is false)
if x > 5 { output('Greater') }
```

**While Loops**:
```lift
while x < 10 {
    x = x + 1
}
```

#### Operators
- **Arithmetic**: `+`, `-`, `*`, `/`
- **Comparison**: `>`, `<`, `>=`, `<=`, `=`, `<>`
- **Logical**: `and`, `or`, `not`
- **String**: `+` (concatenation)
- **Range**: `..` (creates a range from start to end)

**Note on 'not' operator**:
```lift
// The 'not' operator negates boolean values
let a = not true;        // false
let b = not false;       // true
let c = not not true;    // true (double negation)
let d = not (5 > 10);    // true (negates comparison result)
```

#### Comments
```lift
// Single-line comment
let x = 42; // Inline comment

/* Multi-line comment
   can span multiple lines
   and is useful for documentation */
   
let y = 10 /* inline block comment */ + 5;
```

#### Range Types
```lift
// Range literals create a range from start to end
let r1 = 1..10;          // Range from 1 to 10
let r2 = -5..5;          // Range from -5 to 5
let r3 = (1 + 2)..(10 - 3);  // Expressions in ranges

// Ranges are displayed as "start..end"
output(r1);  // Outputs: 1..10
```

#### User-Defined Types
```lift
// Simple type aliases
type Age = Int;
type Name = Str;

// Type with range constraint (syntax supported, constraints not enforced yet)
type Score = 0 to 100;

// Collection type aliases
type Names = List of Str;
type Numbers = List of Int;
type AgeMap = Map of Str to Int;

// Using custom types
let myAge: Age = 25;
let myName: Name = 'Alice';
let scores: Numbers = [85, 90, 95];
let ages: AgeMap = #{'Alice': 25, 'Bob': 30};

// Custom types in functions
function greet(name: Name): Name {
    'Hello ' + name
}

// Nested type definitions
type StringList = List of Str;
type ListOfLists = List of StringList;
```

#### Built-in Methods

Lift provides comprehensive built-in methods for strings, lists, and maps. All methods support both **method syntax** (`obj.method()`) and **UFCS** (`method(self: obj)`).

##### String Methods

```lift
let text = 'Hello World';

// Transformation
text.upper()                        // 'HELLO WORLD'
text.lower()                        // 'hello world'
text.substring(start: 0, end: 5)    // 'Hello'
text.trim()                         // Removes whitespace (for '  hi  ' → 'hi')
text.replace(old: 'World', new: 'Lift')  // 'Hello Lift'

// Searching
text.contains(substring: 'World')   // true
text.starts_with(prefix: 'Hello')   // true
text.ends_with(suffix: 'World')     // true

// Query
text.is_empty()                     // false
''.is_empty()                       // true

// Splitting (returns List of Str)
'a,b,c'.split(delimiter: ',')       // ['a', 'b', 'c']

// Method chaining
'hello'.upper().replace(old: 'E', new: '3')  // 'H3LLO'

// UFCS syntax
upper(self: text)                   // 'HELLO WORLD'
```

##### List Methods

```lift
let numbers = [10, 20, 30, 40, 50];

// Access
numbers.first()                     // 10
numbers.last()                      // 50

// Searching
numbers.contains(item: 30)          // true
numbers.contains(item: 99)          // false

// Slicing
numbers.slice(start: 1, end: 4)     // [20, 30, 40]

// Transformation
numbers.reverse()                   // [50, 40, 30, 20, 10]

// Joining (List of Str only)
['a', 'b', 'c'].join(separator: ',')  // 'a,b,c'

// Query
numbers.is_empty()                  // false
[].is_empty()                       // true (requires type annotation)

// Method chaining
[5, 1, 3, 2, 4].reverse().slice(start: 1, end: 4)  // [3, 2, 1]

// UFCS syntax
first(self: numbers)                // 10
```

##### Map Methods

```lift
let ages = #{'Alice': 25, 'Bob': 30, 'Carol': 35};

// Keys and values (returned in sorted order)
ages.keys()                         // ['Alice', 'Bob', 'Carol']
ages.values()                       // [25, 30, 35]

// Searching
ages.contains_key(key: 'Alice')     // true
ages.contains_key(key: 'Dave')      // false

// Query
ages.is_empty()                     // false

// UFCS syntax
keys(self: ages)                    // ['Alice', 'Bob', 'Carol']
```

**Notes:**
- All methods return new values (immutable - original unchanged)
- `join()` only works on `List of Str`
- `keys()` and `values()` return results in sorted order for consistency
- Empty collections require type annotations: `let empty: List of Int = []`

### Recent Changes

1. **Built-in Methods for String, List, and Map** (Latest - 2025-10-04)
   - Added 17 comprehensive built-in methods
   - **String methods (8):** `substring()`, `contains()`, `trim()`, `split()`, `replace()`, `starts_with()`, `ends_with()`, `is_empty()`
   - **List methods (5):** `contains()`, `slice()`, `reverse()`, `join()`, `is_empty()` (plus existing `first()`, `last()`)
   - **Map methods (4):** `keys()`, `values()`, `contains_key()`, `is_empty()`
   - All methods support both method syntax (`obj.method()`) and UFCS (`method(self: obj)`)
   - Full type safety with proper type inference for generic return types
   - Test files: `tests/test_string_methods.lt`, `tests/test_list_methods.lt`, `tests/test_map_methods.lt`
   - See `docs/TIER1_IMPLEMENTATION_SUMMARY.md` for complete documentation

2. **Method Syntax and UFCS** (2025-10-04)
   - Methods can be defined on types: `function Type.method_name(params): ReturnType { ... }`
   - Implicit `self` parameter available in method body
   - Call with dot notation: `obj.method()` or UFCS: `method(self: obj)`
   - Both syntaxes are equivalent and fully interchangeable
   - Enables method chaining: `'hello'.upper().replace(old: 'E', new: '3')`
   - Future-proof for user-defined types, structs, and enums
   - Test files: `tests/test_implicit_self.lt`, `tests/test_ufcs.lt`, `tests/test_builtins.lt`

3. **Immutable Function Parameters with `cpy` Modifier**
   - Function parameters are immutable by default (cannot use `:=` on them)
   - Use `cpy` keyword before parameter name for pass-by-value/mutable parameters
   - Syntax: `function name(x: Int, cpy y: Int): Int { ... }`
   - `cpy` parameters can be modified inside function without affecting caller
   - Prepares for compiled version with reference semantics
   - Test files: `tests/test_cpy_params.lt`, `tests/test_immutable_params_error.lt`

2. **Mutable Variables with `let var`**
   - Added `let var` syntax for mutable variable declarations
   - Immutable by default with `let`, explicit mutability with `let var`
   - Assignment expressions (`:=`) only work on mutable variables
   - Compile-time mutability checking prevents accidental mutations
   - Test files: `tests/test_mutability.lt`, `tests/demo_let_vs_let_var.lt`

3. **Built-in `len()` Function**
   - Returns length of strings, lists, and maps as `Int`
   - Works on: `Str`, `List of T`, `Map of K to V`
   - Fully type-checked and integrated
   - Test file: `tests/test_len.lt`

4. **Assignment Expressions** (`:=`)
   - Reassign existing mutable variables
   - Type-checked for compatibility
   - Returns `Unit` (expression-based)
   - Works across scopes
   - Test file: `tests/test_assignment.lt`

5. **Type Checker Completion**
   - Implemented type checking for ALL expression types
   - Added strict checking for compiled language support
   - Better error messages for type mismatches
   - Type inference from literals

6. **Else If Support**
   - Grammar updated to support `else if` chains
   - Each `else if` is transformed into nested `If` expressions
   - Missing `else` clauses default to `Unit`
   - Full test coverage added

3. **Negative Number Support**
   - Negative numbers are now properly supported as unary operators
   - Works for both integers (`-42`) and floats (`-3.14`)
   - Handles parser ambiguity correctly (e.g., `1-3` is parsed as `1 - 3`, not `1` and `-3`)
   - Negative literals are optimized to true literals in the AST
   - Full test coverage added

4. **Comment Support**
   - Single-line comments with `//`
   - Multi-line comments with `/* */`
   - Implemented at lexer level using LALRPOP match patterns
   - Comments are properly ignored in all contexts

5. **List Literals**
   - Syntax: `[elem1, elem2, ...]`
   - Type inference from elements (all elements must have compatible types)
   - Empty lists require type annotations (e.g., `let empty: List of Int = []`)
   - Converted to `RuntimeList` (Vec-backed) during interpretation
   - Supports nested lists and expressions as elements
   - Display format: `[1,2,3]`

6. **List Indexing**
   - Syntax: `list[index]`
   - Index must be an integer expression
   - Zero-based indexing (first element is at index 0)
   - Supports nested indexing: `matrix[0][1]`
   - Type inference: indexing returns the element type of the list
   - Runtime bounds checking with descriptive error messages
   - Examples:
     ```lift
     let nums = [10, 20, 30];
     output(nums[0]);        // Outputs: 10
     output(nums[1 + 1]);    // Outputs: 30
     let i = 2;
     output(nums[i]);        // Outputs: 30
     let matrix = [[1, 2], [3, 4]];
     output(matrix[0][1]);   // Outputs: 2
     ```

7. **Map Literals**
   - Syntax: `#{key: value, key2: value2, ...}`
   - Uses `#{}` to avoid conflict with block syntax `{}`
   - Key types: Int, Bool, Str (no Float keys due to HashMap requirements)
   - Type inference from first key-value pair
   - Empty maps require type annotations
   - Converted to `RuntimeMap` (HashMap-backed) during interpretation
   - Supports nested structures (maps of lists, etc.)
   - Display format: `{1:'one',2:'two'}` (sorted by key)

8. **Map Indexing**
   - Syntax: `map[key]`
   - Key must match the map's key type (Int, Str, or Bool)
   - Float keys are not allowed
   - Returns the value associated with the key
   - Runtime error if key not found
   - Supports nested indexing: `nested_map['outer']['inner']`
   - Type inference: indexing returns the value type of the map
   - Examples:
     ```lift
     let ages = #{1: 25, 2: 30, 3: 35};
     output(ages[2]);        // Outputs: 30
     
     let capitals = #{'USA': 'Washington', 'France': 'Paris'};
     output(capitals['France']); // Outputs: 'Paris'
     
     let key = 'USA';
     output(capitals[key]);  // Outputs: 'Washington'
     ```

9. **Comment Support**
   - Single-line comments with `//` syntax
   - Multi-line comments with `/* */` syntax
   - Comments can appear anywhere in the code
   - Properly handles comment-like text in strings
   - Works with inline comments in expressions
   - Full test coverage added

10. **REPL Improvements** (Latest)
   - Fixed infinite loop issue where REPL would hang after evaluating expressions
   - REPL now continues after errors instead of exiting (better for learning)
   - Error messages are shown immediately without duplication
   - Symbol table maintains integrity - erroneous code doesn't pollute it
   - Consistent error handling between REPL and file execution modes

11. **Enhanced Type Inference** (Latest)
   - Fixed type inference for let bindings with variable expressions
   - Added `determine_type_with_symbols` function that can look up variable types
   - Now supports type inference for:
     - Variable references: `let y = x`
     - Expressions with variables: `let y = x + 5`
     - Function calls: `let result = add(x: 10, y: 20)`
     - Complex expressions: `let result = (x + y) * z`
   - Significantly reduces need for explicit type annotations

12. **Range Type Support**
   - Added range operator `..` for creating ranges
   - Syntax: `start..end` where both must be integers
   - Supports expressions: `(1 + 2)..(10 - 3)`
   - Ranges are first-class values that can be stored in variables
   - Display format: `1..10`
   - Future: Could be used for array slicing, iteration

13. **User-Defined Types**
   - Added `type` keyword for type definitions
   - Simple aliases: `type Age = Int`
   - Collection aliases: `type Names = List of Str`
   - Map aliases: `type AgeMap = Map of Str to Int`
   - Range constraints (syntax only): `type Score = 0 to 100`
   - Types can be used in variable declarations and function signatures
   - Support for nested type definitions
   - Types are resolved during semantic analysis phase
      let stepped = 1..10 step 2; // Range from 1 to 10 with step of 2
      ```
    - Range types are inferred based on start and end values
    - Supports both integer and floating-point ranges
    - Implemented user-defined types through custom type definitions with type aliases and structs
    - Type aliases allow creating named types from existing types
    - Structs provide a way to define complex, composite types with named fields

### Test Programs
Test programs are located in the `tests/` directory:
- `tests/test_else_if.lt` - Demonstrates else if chains
- `tests/test_if_no_else.lt` - Shows optional else behavior
- `tests/test_typechecker.lt` - Type checking examples
- `tests/test_type_error.lt` - Type error example
- `tests/test_needs_annotation.lt` - Shows when type annotations are needed
- `tests/type_checking_examples.lt` - Various type checking scenarios
- `tests/test_negative_numbers.lt` - Tests negative number literals and operations
- `tests/test_comments.lt` - Tests comment support with various scenarios
- `tests/test_lists.lt` - List literal tests (contains error case)
- `tests/test_lists_simple.lt` - List literal tests without errors
- `tests/test_maps.lt` - Map literal tests
- `tests/test_maps_simple.lt` - Map literal tests without errors
- `tests/test_runtime_collections.lt` - Tests runtime collection behavior
- `tests/test_type_resolution.lt` - Tests let binding type resolution
- `tests/test_repl.lt` - Documents expected REPL behavior
- `tests/simple_list.lt` - Simple empty list test case
- `tests/test_range.lt` - Tests for the new Range type
- `tests/test_list_indexing_simple.lt` - Basic list indexing tests
- `tests/test_list_indexing_final.lt` - Comprehensive list indexing tests
- `tests/test_list_indexing_errors.lt` - Error handling for list indexing
- `tests/test_map_indexing_simple.lt` - Basic map indexing tests
- `tests/test_map_indexing_comprehensive.lt` - Complex map indexing scenarios
- `tests/test_map_indexing_errors.lt` - Error handling for map indexing
- `tests/test_implicit_self.lt` - Tests implicit self parameter in methods
- `tests/test_ufcs.lt` - Tests Uniform Function Call Syntax
- `tests/test_builtins.lt` - Tests built-in methods with both syntaxes
- `tests/test_string_methods.lt` - Comprehensive string method tests
- `tests/test_list_methods.lt` - Comprehensive list method tests
- `tests/test_map_methods.lt` - Comprehensive map method tests

### Known Limitations

1. **For loops not yet implemented** - only while loops available
2. **Match expressions not yet implemented** - defined in AST but not functional
3. **No module/import system** - all code must be in one file
4. **Limited standard library** - `output()` and `len()` are the only standalone functions
5. **Limited user-defined type support** - work in progress (type aliases work)
6. **Return statements exist but may need refinement**
7. **No automatic Int to Flt conversion** - types must match exactly in operations
8. **While loops don't return values** - use recursion for computed loops or wait for for loops
9. **`output()` adds quotes around strings** - no control over formatting
10. **No mutation methods for collections** - all methods return new values (immutable)
11. **Empty list/map literals require explicit type annotations** - type inference cannot infer from empty collections
12. **No break/continue statements** - cannot exit loops early
13. **No string interpolation** - must use concatenation with `+`

[... rest of the file remains the same ...]