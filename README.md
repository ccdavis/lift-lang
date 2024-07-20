# lift-lang

I intended `Lift` to give me an excuse to mess around with the Cranelift  compiler backend and parser generators. Haven't made the compiler yet. I hope this approach makes it easier to add LSP support as well as JIT from Cranelift.

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






