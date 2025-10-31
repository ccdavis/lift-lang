# Lift Language Documentation

This directory contains comprehensive documentation for the Lift programming language.

## Performance & Benchmarks

### Mandelbrot Benchmarks
- **[LARGE_SCALE_BENCHMARKS.md](LARGE_SCALE_BENCHMARKS.md)** - Comprehensive performance comparison at 200×100 resolution
  - Lift JIT: 10ms (1st place 🥇)
  - Python Iterative: 40ms (2nd place)
  - Python Recursive: 130ms (3rd place)
  - Lift Interpreter: 2,080ms (4th place)

- **[COMPILER_RESULTS.md](COMPILER_RESULTS.md)** - JIT compiler fix and performance results
  - How the global variable issue was fixed
  - 208x speedup over interpreter
  - Compiler limitations discovered

- **[benchmark_results.md](benchmark_results.md)** - Original benchmark results (60×30 resolution)
  - Lift interpreter vs Python comparison
  - 8x performance difference

- **[iterative_comparison.md](iterative_comparison.md)** - Recursive vs iterative implementation analysis
  - Why iterative is 750x slower in Lift interpreter
  - Python handles both approaches efficiently

- **[MANDELBROT_SUMMARY.md](MANDELBROT_SUMMARY.md)** - Complete project summary
  - All implementations and results
  - Key insights and recommendations

## Language Features

### Type System & Methods
- **[TIER1_IMPLEMENTATION_SUMMARY.md](TIER1_IMPLEMENTATION_SUMMARY.md)** - Built-in methods documentation
  - String methods (8 methods)
  - List methods (7 methods)
  - Map methods (4 methods)
  - Method syntax and UFCS

- **[METHOD_SYSTEM_ANALYSIS.md](METHOD_SYSTEM_ANALYSIS.md)** - Method system design analysis
- **[USER_DEFINED_TYPE_METHODS.md](USER_DEFINED_TYPE_METHODS.md)** - User-defined method support
- **[COLLECTION_TYPE_ALIAS_METHODS.md](COLLECTION_TYPE_ALIAS_METHODS.md)** - Methods on type aliases

### Language Semantics
- **[MUTABILITY_GUIDE.md](MUTABILITY_GUIDE.md)** - Guide to `let` vs `let var`
  - Immutable by default
  - Assignment expressions
  - Compile-time checking

- **[PARAMETER_SEMANTICS.md](PARAMETER_SEMANTICS.md)** - Function parameter behavior
  - Immutable parameters (default)
  - `cpy` modifier for mutable parameters
  - Pass-by-reference vs pass-by-value

### What's New
- **[WHATS_NEW.md](WHATS_NEW.md)** - Recent language additions and changes

### Language Reference
- **[SYNTAX.md](SYNTAX.md)** - Formal syntax specification in BNF notation

## Compiler & Implementation

### Compiler Status
- **[COMPILER_STATUS.md](COMPILER_STATUS.md)** - JIT compiler feature coverage and current status

### Struct Implementation
- **[STRUCT_IMPLEMENTATION_STATUS.md](STRUCT_IMPLEMENTATION_STATUS.md)** - Current struct support status (100% complete)
- **[STRUCT_COMPILER_DESIGN.md](STRUCT_COMPILER_DESIGN.md)** - Struct compiler design

## Feature Planning
- **[FEATURE_ANALYSIS.md](FEATURE_ANALYSIS.md)** - Feature analysis and planning
- **[FEATURE_RECOMMENDATIONS.md](FEATURE_RECOMMENDATIONS.md)** - Recommended features and priorities
- **[DOCUMENTATION_INDEX.md](DOCUMENTATION_INDEX.md)** - Documentation organization

## Quick Links

### Getting Started
- See [../README.md](../README.md) for installation and basic usage
- See [../CLAUDE.md](../CLAUDE.md) for development guidelines

### Examples
- Mandelbrot visualizers: [../examples/mandelbrot/](../examples/mandelbrot/)
- Test programs: [../tests/](../tests/)

### Performance Summary

**Lift JIT Compiler Performance (200×100 Mandelbrot):**
- ✅ 10ms execution time
- ✅ 4x faster than Python iterative
- ✅ 13x faster than Python recursive
- ✅ 208x faster than Lift interpreter
- ✅ 2,000 points/ms throughput

**Key Takeaway:** Lift's JIT compiler produces production-ready native code that outperforms Python significantly on compute-intensive tasks!
