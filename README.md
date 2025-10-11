# lift-lang

I intended `Lift` to give me an excuse to mess around with the Cranelift compiler backend and parser generators. The project now includes both a tree-walking interpreter and a **Cranelift JIT compiler** with native x86-64 code generation. The parser-generator approach (LALRPOP) makes it easier to add LSP support and enables the JIT compilation backend.

## The Language

Lift consists of expressions and type definitions. Technically a type definition is a special expression. Function definitions count as expressions also, with the special form of `function NAME(args-list): return-type` as a way to assign the function to a named variable of type Function. 

```Scala
function cube(input: Int): Int {
    input * input * input
};

Or for special single expression functions:
```scala
function cube(input: Int): Int = input * input * input;
```

(Semicolons serve as expression separators, not terminators.)

Type definitions and function definitions evaluate to the unit type while updating their scope with their definition.

```scala
type Html = String;
```

Besides structure and enum types, assigning  existing types to names make a new type, not an alias. 

This does the same as the special function definition form:
```scala
let cube = lambda(input: Int): Int {
    input * input * input
};
```

As with other expression languages like Rust or Ruby, lists of expressions make up function bodies and blocks after conditionals.  The last expression in a list provides the return value, no explicit return required.

```scala
function microblog_post_html(headline: String, content: String, author: String): String {
    let posting_time = "<h3> Posted at " + String(date_time()) + "</h3>";
    let heading = "<h1>" + headline + "</h1>" + posting_time;
    heading + "<p>" + content + "</p>"
};
```

## Interpreter, Compiler, and REPL

Lift supports two execution modes:

### Tree-Walking Interpreter
The interpreter supports the full language (primitives, collections, functions, methods, control flow, etc.).

Run the interpreter:
```bash
cargo run -- tests/test_else_if.lt
# or with release build for faster execution
cargo run --release -- tests/test_else_if.lt
```

### Cranelift JIT Compiler ⚡
The JIT compiler provides native x86-64 code generation for most language features (85%+ coverage). Expected 10-50x performance improvement for arithmetic and 5-20x for function calls.

Run the compiler:
```bash
cargo run -- --compile tests/test_else_if.lt
```

Compare interpreter vs compiler:
```bash
# Run same file with both modes
cargo run -- tests/test_file.lt           # Interpreter
cargo run -- --compile tests/test_file.lt # Compiler
```

### REPL
The REPL works with the interpreter (compiler mode not available in REPL yet). Use `\` to continue expressions across lines. Use Ctrl-D to clear buffer and Ctrl-C to quit.

```bash
cargo run  # Starts REPL
```

### Type Checking
The type checking step is fully implemented for all expression types, with strict checking suitable for compilation and type inference from literals.





