# Lift Language Test Programs

This directory contains test programs written in the Lift language (.lt files).

## Test Files

### Control Flow Tests
- `test_else_if.lt` - Demonstrates else if chains with grade calculation
- `test_if_no_else.lt` - Shows if statements without else clauses (returns Unit)

### Type Checking Tests  
- `test_typechecker.lt` - Basic type checking examples with variables and expressions
- `test_type_error.lt` - Demonstrates a type error (trying to add string and int)
- `test_needs_annotation.lt` - Shows when explicit type annotations are required
- `type_checking_examples.lt` - Various type checking scenarios including functions

### Demo Programs
- `mandelbrot_visual.lt` - A static ASCII art representation of the Mandelbrot set (used in tests)
- `mandelbrot_tiny_computed.lt` - Computes membership for specific points in the Mandelbrot set (used in tests)
- `test_pattern.lt` - Simple pattern output test (used in tests)
- `test_recursion_depth.lt` - Tests recursion depth capability (used in tests)

### Other Test Files
- `test_float.lt` - Tests floating point operations (has type warnings but runs)
- `test_output.lt` - Tests output formatting
- `mandelbrot_working.lt` - Alternative static Mandelbrot pattern

## Running Tests

To run a test file:
```bash
cargo run tests/test_file.lt
```

For example:
```bash
cargo run tests/test_else_if.lt
```

## Writing New Tests

When creating new test programs:
1. Use descriptive names that indicate what is being tested
2. Include comments explaining the expected behavior
3. Keep tests focused on a single feature or concept
4. Consider both success and error cases