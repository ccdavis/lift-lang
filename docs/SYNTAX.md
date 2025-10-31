# Lift Language Syntax Reference (BNF)

This document provides a formal description of the Lift programming language syntax using Backus-Naur Form (BNF) notation.

## Notation Conventions

- `<name>` - Non-terminal symbol (a syntax rule)
- `"text"` - Terminal symbol (literal text)
- `|` - Alternative (OR)
- `[ ]` - Optional (zero or one occurrence)
- `{ }` - Repetition (zero or more occurrences)
- `( )` - Grouping
- `/* comment */` - Explanatory comment

---

## Program Structure

```bnf
<program> ::= <expression> { ";" <expression> }

<expression> ::= <block>
               | <type-definition>
               | <output-call>
               | <len-call>
               | <if-expression>
               | <while-loop>
               | <let-binding>
               | <function-definition>
               | <lambda>
               | <assignment>
               | <logic-or>
```

---

## Blocks and Scopes

```bnf
<block> ::= "{" <expression> { ";" <expression> } "}"
```

**Note:** The last expression in a block is its return value (if not followed by semicolon).

---

## Type System

### Type Definitions

```bnf
<type-definition> ::= "type" <identifier> "=" <type-spec>

<type-spec> ::= <simple-type>
              | <range-type>
              | <collection-type>
              | <enum-type>
              | <struct-type>

<simple-type> ::= <data-type>
                | <identifier>  /* type alias reference */

<range-type> ::= <int-literal> "to" <int-literal>
               | <str-literal> "to" <str-literal>

<collection-type> ::= "List" "of" <data-type>
                    | "Map" "of" <data-type> "to" <data-type>
                    | "Set" "of" <data-type>

<enum-type> ::= "(" <identifier> { "," <identifier> } ")"

<struct-type> ::= "struct" "(" <parameter> { "," <parameter> } ")"
```

### Data Types

```bnf
<data-type> ::= "Int"
              | "Flt"
              | "Str"
              | "Bool"
              | "List of" <data-type>
              | "Map of" <data-type> "to" <data-type>
              | <identifier>  /* custom type reference */

<type-annotation> ::= ":" <data-type>
```

---

## Variables and Assignment

### Variable Declarations

```bnf
<let-binding> ::= "let" <identifier> [ <type-annotation> ] "=" <expression>
                | "let" "var" <identifier> [ <type-annotation> ] "=" <expression>
```

**Examples:**
```lift
let x = 5                    /* immutable, type inferred */
let y: Int = 10              /* immutable, explicit type */
let var count = 0            /* mutable */
let var name: Str = 'Alice'  /* mutable, explicit type */
```

### Assignment

```bnf
<assignment> ::= <identifier> ":=" <expression>
               | <expression> "." <identifier> ":=" <expression>  /* field assignment */
```

**Note:** Assignment only works on mutable variables (declared with `let var`).

---

## Control Flow

### If-Else Expressions

```bnf
<if-expression> ::= "if" <expression> <block> [ <else-part> ]

<else-part> ::= "else" "if" <expression> <block> [ <else-part> ]
              | "else" <block>
```

**Examples:**
```lift
if x > 10 { 'big' } else { 'small' }

if score >= 90 { 'A' }
else if score >= 80 { 'B' }
else if score >= 70 { 'C' }
else { 'F' }
```

### While Loops

```bnf
<while-loop> ::= "while" <expression> <block>
```

**Example:**
```lift
while x < 10 {
    x := x + 1
}
```

---

## Functions

### Function Definitions

```bnf
<function-definition> ::= "function" <identifier> "(" <parameter-list> ")" <type-annotation> <block>
                        | "function" <type-name> "." <identifier> "(" <parameter-list> ")" <type-annotation> <block>

<parameter-list> ::= [ <parameter> { "," <parameter> } ]

<parameter> ::= <identifier> <type-annotation>
              | "cpy" <identifier> <type-annotation>
```

**Examples:**
```lift
/* Regular function */
function add(x: Int, y: Int): Int {
    x + y
}

/* Method on a type */
function Str.exclaim(): Str {
    self + '!'
}

/* Function with mutable parameter */
function increment(cpy x: Int): Int {
    x := x + 1;
    x
}
```

### Lambda Expressions

```bnf
<lambda> ::= "Lambda" "(" <parameter-list> ")" <type-annotation> <block>
```

### Function Calls

```bnf
<function-call> ::= <identifier> "(" <argument-list> ")"

<argument-list> ::= [ <keyword-arg> { "," <keyword-arg> } ]

<keyword-arg> ::= <identifier> ":" <expression>
```

**Example:**
```lift
add(x: 5, y: 3)
```

---

## Built-in Functions

```bnf
<output-call> ::= "output" "(" <expression> { "," <expression> } ")"

<len-call> ::= "len" "(" <expression> ")"
```

**Examples:**
```lift
output('Hello, World!')
output(x, y, z)

let length = len('hello')
let count = len([1, 2, 3])
```

---

## Expressions

### Logical Expressions

```bnf
<logic-or> ::= <logic-and> { "or" <logic-and> }

<logic-and> ::= <logic-not> { "and" <logic-not> }

<logic-not> ::= "not" <logic-not>
              | <equality>
```

### Comparison Expressions

```bnf
<equality> ::= <comparison> { <equality-op> <comparison> }

<equality-op> ::= "="   /* equal */
                | "<>"  /* not equal */

<comparison> ::= <range> { <comparison-op> <range> }

<comparison-op> ::= ">"   /* greater than */
                  | ">="  /* greater than or equal */
                  | "<"   /* less than */
                  | "<="  /* less than or equal */
```

### Range Expressions

```bnf
<range> ::= <arithmetic> [ ".." <arithmetic> ]
```

**Example:**
```lift
let r = 1..10
```

### Arithmetic Expressions

```bnf
<arithmetic> ::= <term> { <add-op> <term> }

<add-op> ::= "+" | "-"

<term> ::= <factor> { <mul-op> <factor> }

<mul-op> ::= "*" | "/"

<factor> ::= "-" <factor>  /* unary negation */
           | <postfix>
```

### Postfix Expressions

```bnf
<postfix> ::= <atom> { <postfix-op> }

<postfix-op> ::= "[" <expression> "]"  /* indexing */
               | "." <identifier> "(" <argument-list> ")"  /* method call */
               | "." <identifier>  /* field access */
```

**Examples:**
```lift
myList[0]
myMap['key']
'hello'.upper()
person.name
```

---

## Literals and Atoms

### Atom Expressions

```bnf
<atom> ::= "(" <expression> ")"
         | <literal>
         | <list-literal>
         | <map-literal>
         | <function-call>
         | <struct-literal>
         | <variable>

<variable> ::= <identifier>
```

### Literals

```bnf
<literal> ::= <int-literal>
            | <flt-literal>
            | <str-literal>
            | <bool-literal>

<int-literal> ::= <digit> { <digit> }

<flt-literal> ::= <digit> { <digit> } "." <digit> { <digit> }

<str-literal> ::= "'" { <any-char-except-quote> } "'"

<bool-literal> ::= "true" | "false"
```

### Collection Literals

```bnf
<list-literal> ::= "[" [ <expression> { "," <expression> } ] "]"

<map-literal> ::= "#" "{" [ <map-pair> { "," <map-pair> } ] "}"

<map-pair> ::= <literal> ":" <expression>
```

**Examples:**
```lift
let nums = [1, 2, 3, 4, 5]
let empty: List of Int = []

let ages = #{'Alice': 25, 'Bob': 30}
let scores = #{1: 100, 2: 95, 3: 87}
```

### Struct Literals

```bnf
<struct-literal> ::= <identifier> "(" <argument-list> ")"
```

**Note:** Struct literals use the same syntax as function calls. The semantic analyzer distinguishes them based on whether the identifier is a type or function name.

**Example:**
```lift
type Person = struct (name: Str, age: Int)

let alice = Person(name: 'Alice', age: 30)
```

---

## Lexical Elements

### Identifiers

```bnf
<identifier> ::= <letter> { <letter> | <digit> | "_" | "-" }

<letter> ::= "a" | "b" | ... | "z" | "A" | "B" | ... | "Z"

<digit> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
```

### Type Names

```bnf
<type-name> ::= "Str" | "Int" | "Flt" | "Bool" | "List" | "Map" | "Range" | <identifier>
```

### Comments

```bnf
<line-comment> ::= "//" { <any-char-except-newline> } <newline>

<block-comment> ::= "/*" { <any-char> } "*/"
```

**Examples:**
```lift
// This is a line comment

/* This is a
   block comment */

let x = 42; // Inline comment
```

---

## Operator Precedence

From highest to lowest precedence:

1. **Postfix operators** - `[]` (indexing), `.` (field access, method call)
2. **Unary operators** - `-` (negation), `not`
3. **Multiplicative** - `*`, `/`
4. **Additive** - `+`, `-`
5. **Range** - `..`
6. **Comparison** - `>`, `>=`, `<`, `<=`
7. **Equality** - `=`, `<>`
8. **Logical AND** - `and`
9. **Logical OR** - `or`
10. **Assignment** - `:=`

---

## Keywords

Reserved words in Lift:

```
and          Bool         cpy          else         false
Flt          function     if           Int          Lambda
len          let          List         Map          not
of           or           output       Range        Set
Str          struct       to           true         type
var          while
```

---

## Whitespace and Formatting

- Whitespace (spaces, tabs, newlines) is ignored except as token separators
- Semicolons (`;`) separate expressions within blocks or programs
- The last expression in a block without a trailing semicolon is the block's value
- A semicolon after the last expression makes the block return Unit `()`

---

## Complete Example

```lift
// Type definition
type Age = Int;
type Person = struct (name: Str, age: Age);

// Function definition
function greet(person: Person): Str {
    'Hello, ' + person.name + '!'
};

// Variable declarations
let alice = Person(name: 'Alice', age: 30);
let var count = 0;

// Control flow
if alice.age >= 18 {
    output('Adult');
    count := count + 1
} else {
    output('Minor')
};

// While loop
while count < 5 {
    output(count);
    count := count + 1
};

// Collections
let numbers = [1, 2, 3, 4, 5];
let firstThree = numbers.slice(start: 0, end: 3);
output(firstThree);

// Map
let config = #{'debug': true, 'port': 8080};
if config.contains_key(key: 'debug') {
    output('Debug mode enabled')
}
```

---

## See Also

- [CLAUDE.md](../CLAUDE.md) - Complete language guide with detailed explanations
- [README.md](../README.md) - Getting started guide
- [FEATURE_RECOMMENDATIONS.md](FEATURE_RECOMMENDATIONS.md) - Planned language features
