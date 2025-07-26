# lift-lang

I intended `Lift` to give me an excuse to mess around with the Cranelift  compiler backend and parser generators. Haven't made the compiler yet. I hope the parser-generator approach makes it easier to add LSP support as well as JIT from Cranelift.

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

## Interpreter and REPL

Currently the interpreter (tree-walking type) supports most of the language. However I went nuts adding different kinds of data types like ranges and special enums. Implementing all that is hard and I should probably rethink all that. 

The type checking step is fully implemented for all expression types, with strict checking suitable for compilation and type inference from literals.

Run the interpreter by passing a source file:
```bash
cargo run tests/test_else_if.lt
```
or with `cargo run --release tests/test_else_if.lt`

The REPL works decently well now. Due to the syntax with expression separators it's a bit hard to enter multi-line expressions.  Continue an expression with '\'; if you're delaying evaluation in a multi expression block you also need to add the ';'  as expression separators according to the syntax rules.

Use control-D to clear the buffer and control-C to quit the REPL.





