# Lift Language - Feature Recommendations

**Last Updated:** 2025-10-04

After analyzing the current state of Lift with newly added method syntax and UFCS, here are the most valuable additions to consider next, organized by implementation difficulty and value.

## ✅ COMPLETED FEATURES (2025-10-04)

**Tier 1 Implementation - 17 Built-in Methods Added:**
- ✅ String methods (8): `substring`, `contains`, `trim`, `split`, `replace`, `starts_with`, `ends_with`, `is_empty`
- ✅ List methods (5): `contains`, `slice`, `reverse`, `join`, `is_empty`
- ✅ Map methods (4): `keys`, `values`, `contains_key`, `is_empty`

See `TIER1_IMPLEMENTATION_SUMMARY.md` for complete documentation.

---

## Current State Summary

**What Lift Has:**
- Static typing with type inference
- Primitives: Int, Flt, Str, Bool, Unit
- Collections: List, Map, Range
- Control flow: if/else if/else, while loops
- Functions with named parameters
- Method syntax with UFCS
- Variables (immutable `let`, mutable `let var`)
- Built-in functions: `output()`, `len()`
- Built-in methods: `upper()`, `lower()`, `first()`, `last()`
- Type aliases
- Indexing for lists and maps
- Return statements (may need refinement)

**What's Missing:**
- For loops (only while loops exist)
- Break/continue statements
- Most string/list/map manipulation methods
- String interpolation
- Anonymous functions/lambdas
- Pattern matching (Match defined but not implemented)
- Tuples
- Module/import system

---

## TIER 1: Easiest to Add, Highest Value ⭐

### 1. Additional String Methods (VERY EASY, HIGH VALUE)
These follow the same pattern as `upper()` and `lower()` we just implemented.

**Recommended methods:**
```lift
// String slicing
let text = 'Hello World';
let sub = text.substring(start: 0, end: 5);  // 'Hello'

// String searching
let has = text.contains(substring: 'World');  // true
let starts = text.starts_with(prefix: 'Hello');  // true
let ends = text.ends_with(suffix: 'World');  // true

// String transformation
let trimmed = '  hello  '.trim();  // 'hello'
let parts = text.split(delimiter: ' ');  // ['Hello', 'World']
let replaced = text.replace(old: 'World', new: 'Lift');  // 'Hello Lift'

// String inspection
let isEmpty = ''.is_empty();  // true
```

**Implementation notes:**
- Add to `SymbolTable::add_builtins()` like existing methods
- Implement in interpreter's `MethodCall` handling
- Type checking similar to existing methods
- Most are simple string operations in Rust

**Estimated effort:** 2-4 hours

---

### 2. Additional List Methods (EASY, HIGH VALUE)
Essential for practical list manipulation.

**Recommended methods:**
```lift
let numbers = [1, 2, 3, 4, 5];

// Membership testing
let has = numbers.contains(item: 3);  // true

// Slicing (similar to substring)
let slice = numbers.slice(start: 1, end: 3);  // [2, 3]

// Reversal
let reversed = numbers.reverse();  // [5, 4, 3, 2, 1]

// Join (for string lists)
let words = ['Hello', 'World'];
let joined = words.join(separator: ' ');  // 'Hello World'

// Size
let count = numbers.count();  // 5 (could use len() but this is more consistent)

// Check if empty
let isEmpty = [].is_empty();  // true
```

**Implementation notes:**
- Follow same pattern as `first()` and `last()`
- `join()` requires type checking (only works on List of Str)
- `slice()` returns new list with same element type
- Most are straightforward Vec operations

**Estimated effort:** 3-5 hours

---

### 3. Additional Map Methods (EASY, HIGH VALUE)
Critical for working with maps effectively.

**Recommended methods:**
```lift
let ages = #{'Alice': 25, 'Bob': 30, 'Carol': 35};

// Get all keys
let names = ages.keys();  // ['Alice', 'Bob', 'Carol']

// Get all values
let ageList = ages.values();  // [25, 30, 35]

// Check key membership
let hasAlice = ages.contains_key(key: 'Alice');  // true

// Size
let count = ages.count();  // 3

// Check if empty
let isEmpty = #{}.is_empty();  // true

// Safe get with default (optional - more advanced)
let age = ages.get(key: 'Dave', default: 0);  // 0
```

**Implementation notes:**
- `keys()` and `values()` return lists
- Type inference: `Map of K to V` → `keys()` returns `List of K`, `values()` returns `List of V`
- Requires iterating over HashMap in interpreter

**Estimated effort:** 3-4 hours

---

### 4. For Loops (MEDIUM, VERY HIGH VALUE)
The single most valuable feature to add. Enables natural iteration.

**Syntax proposals:**
```lift
// Iterate over range
for i in 1..10 {
    output(i);
}

// Iterate over list
let numbers = [10, 20, 30];
for num in numbers {
    output(num);
}

// Iterate over map (gets key-value pairs or just keys?)
let ages = #{'Alice': 25, 'Bob': 30};
for name in ages.keys() {
    output(name);
}

// With index (advanced - could defer)
for (i, value) in numbers.enumerate() {
    output(i, value);
}
```

**Implementation requirements:**
1. **Grammar changes:**
   - Add `for var in expr { body }` syntax
   - Parse into new `Expr::For` variant

2. **AST addition:**
   ```rust
   For {
       var_name: String,
       iterable: Box<Expr>,
       body: Box<Expr>,
       index: (usize, usize),  // for the loop variable
   }
   ```

3. **Type checking:**
   - Check iterable is Range, List, or Map
   - Infer loop variable type from iterable element type
   - Add loop variable to nested scope

4. **Interpreter:**
   - Evaluate iterable expression
   - Create new scope with loop variable
   - Iterate and bind loop variable for each iteration
   - Execute body each iteration

**Estimated effort:** 6-10 hours

**Priority:** VERY HIGH - This transforms the language's usability

---

## TIER 2: Medium Effort, High Value

### 5. Break and Continue Statements (MEDIUM, HIGH VALUE)
Essential for loop control.

**Syntax:**
```lift
// Find first even number
for i in 1..100 {
    if i % 2 = 0 {
        output(i);
        break;  // Exit loop
    }
}

// Skip odd numbers
for i in 1..10 {
    if i % 2 <> 0 {
        continue;  // Skip to next iteration
    }
    output(i);
}
```

**Implementation requirements:**
1. Add `Expr::Break` and `Expr::Continue` variants
2. Grammar: `break;` and `continue;` statements
3. Interpreter: Use Rust control flow or special return values
4. Type checking: Ensure only used inside loops
5. While loops should also support break/continue

**Estimated effort:** 4-6 hours

**Note:** Should be done after or alongside for loops

---

### 6. String Interpolation (MEDIUM, HIGH VALUE)
Makes string building much more ergonomic.

**Syntax options:**
```lift
let name = 'Alice';
let age = 25;

// Option 1: Curly brace interpolation
let msg = "Hello, {name}! You are {age} years old.";

// Option 2: Template syntax (if double quotes are for interpolation)
let msg = 'Hello, ' + name + '! You are ' + str(age) + ' years old.';  // current way
```

**Implementation requirements:**
1. **Grammar:** Parse string literals differently when they contain `{}`
2. **AST:** New `StringInterpolation` expression with parts
3. **Type checking:** Check that interpolated expressions exist and can be converted to strings
4. **Interpreter:** Build string by evaluating each interpolation

**Alternative:** Add `str()` or `to_string()` method first for explicit conversion, defer interpolation

**Estimated effort:** 6-10 hours for full interpolation, 2 hours for `str()` method

---

### 7. Anonymous Functions / Lambdas (MEDIUM-HARD, HIGH VALUE)
Enables functional programming patterns.

**Syntax:**
```lift
// Lambda definition
let double = |x: Int| -> Int { x * 2 };
let result = double(5);  // 10

// Inline in function calls
let numbers = [1, 2, 3, 4, 5];
let doubled = numbers.map(|x| x * 2);  // [2, 4, 6, 8, 10]

// Multiple parameters
let add = |x: Int, y: Int| -> Int { x + y };
```

**Implementation requirements:**
1. **Grammar:** Parse `|params| -> type { body }` or `|params| { body }`
2. **AST:** Lambda is already defined! Just need syntax
3. **Type checking:** Infer lambda type, check parameter and return types
4. **Interpreter:** Create closure capturing environment
5. **First-class functions:** Allow storing in variables, passing as arguments

**Estimated effort:** 10-15 hours (closures are tricky)

**Note:** Once you have this, you can add `map()`, `filter()`, `reduce()` methods

---

### 8. Higher-Order List Methods (REQUIRES LAMBDAS)
Powerful functional programming tools.

**Recommended methods:**
```lift
let numbers = [1, 2, 3, 4, 5];

// Map - transform each element
let doubled = numbers.map(|x| x * 2);  // [2, 4, 6, 8, 10]

// Filter - select elements
let evens = numbers.filter(|x| x % 2 = 0);  // [2, 4]

// Reduce - combine elements
let sum = numbers.reduce(|acc, x| acc + x, 0);  // 15

// ForEach - side effects
numbers.forEach(|x| output(x));

// Any - check if any element satisfies predicate
let hasEven = numbers.any(|x| x % 2 = 0);  // true

// All - check if all elements satisfy predicate
let allPositive = numbers.all(|x| x > 0);  // true
```

**Estimated effort:** 6-8 hours (after lambdas are implemented)

---

## TIER 3: More Complex Features

### 9. Tuples (MEDIUM, MEDIUM VALUE)
Lightweight composite type for grouping values.

**Syntax:**
```lift
// Tuple creation
let point = (10, 20);
let person = ('Alice', 25, true);

// Tuple destructuring
let (x, y) = point;
output(x);  // 10

// Tuple indexing
let name = person.0;  // 'Alice'
let age = person.1;   // 25

// Function returning tuple
function divmod(a: Int, b: Int): (Int, Int) {
    (a / b, a % b)
}

let (quotient, remainder) = divmod(a: 17, b: 5);
```

**Estimated effort:** 12-20 hours

---

### 10. Pattern Matching (HARD, HIGH VALUE)
Match expression is defined but not implemented.

**Syntax:**
```lift
match value {
    1 => 'one',
    2 => 'two',
    3 => 'three',
    _ => 'other'
}

// With destructuring (advanced)
match point {
    (0, 0) => 'origin',
    (0, _) => 'on y-axis',
    (_, 0) => 'on x-axis',
    _ => 'elsewhere'
}
```

**Estimated effort:** 15-25 hours

---

## Recommended Implementation Order

### ✅ Phase 1: String/Collection Methods (COMPLETED - 2025-10-04)
1. ✅ String methods: `substring`, `contains`, `trim`, `split`, `replace`, `starts_with`, `ends_with`, `is_empty`
2. ✅ List methods: `contains`, `slice`, `reverse`, `join`, `is_empty`
3. ✅ Map methods: `keys`, `values`, `contains_key`, `is_empty`

**Impact:** Makes the language immediately more practical for string and data manipulation ✨

**Status:** All 17 methods implemented, tested, and documented!

---

### 🔄 Phase 2: For Loops (NEXT - Estimated 1 week)
4. For loops over ranges and collections
   - `for i in 1..10`
   - `for item in list`
   - `for k, v in map`
5. Break/continue statements

**Impact:** Transforms usability, eliminates need for recursion in simple cases

**Status:** AST structure designed, ready for implementation

---

### Phase 3: Functional Features (2-3 weeks)
6. Anonymous functions/lambdas
7. Higher-order list methods (map, filter, reduce)

**Impact:** Enables functional programming style, very powerful for data processing

---

### Phase 4: Advanced Features (2-4 weeks)
8. String interpolation
9. Tuples with destructuring
10. Pattern matching (Match implementation)

---

## Summary

### ✅ Phase 1 Complete!

**Phase 1 is DONE!** - All 17 string, list, and map methods have been successfully implemented, tested, and documented. The standard library has been dramatically expanded, making Lift much more usable for real programs.

### 🎯 Next Steps

**Phase 2 (For loops)** is the highest-value individual feature and should be implemented next. This single feature will transform the language's usability by enabling natural iteration patterns.

**Remaining estimated effort for Phase 2:** 1 week of focused development

With Phase 1 complete and Phase 2 on deck, Lift is well on its way to becoming a genuinely practical language for everyday programming tasks!

### 📊 Progress Tracking

- ✅ **Phase 1:** String/Collection Methods (17 methods) - **COMPLETE**
- 🔄 **Phase 2:** For Loops + Break/Continue - **NEXT**
- ⏸️ **Phase 3:** Functional Features (lambdas, higher-order methods) - **FUTURE**
- ⏸️ **Phase 4:** Advanced Features (interpolation, tuples, pattern matching) - **FUTURE**
