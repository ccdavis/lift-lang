# Project Organization Summary

## Directory Structure

```
lift-lang/
├── README.md                  # Main project documentation
├── CLAUDE.md                  # Development guidelines for Claude Code
├── docs/                      # All documentation (25 files)
│   ├── README.md             # Documentation index
│   ├── LARGE_SCALE_BENCHMARKS.md
│   ├── COMPILER_RESULTS.md
│   ├── MANDELBROT_SUMMARY.md
│   ├── benchmark_results.md
│   ├── iterative_comparison.md
│   └── [20 more documentation files]
├── examples/                  # Example programs
│   └── mandelbrot/           # Mandelbrot visualizers (8 files)
│       ├── README.md
│       ├── mandelbrot_recursive.lt
│       ├── mandelbrot_large.lt
│       ├── mandelbrot_iterative.lt
│       ├── mandelbrot.py
│       ├── mandelbrot_large.py
│       ├── mandelbrot_large_iterative.py
│       └── mandelbrot_iterative.py
├── tests/                     # Test programs
├── src/                       # Rust source code
└── [build artifacts and config files]
```

## Changes Made

### Documentation Organized (docs/)
Moved 24 markdown files to `docs/`:

**Performance & Benchmarks:**
- LARGE_SCALE_BENCHMARKS.md (main benchmark results)
- COMPILER_RESULTS.md (compiler fix documentation)
- MANDELBROT_SUMMARY.md (complete project summary)
- benchmark_results.md (original benchmarks)
- iterative_comparison.md (recursive vs iterative analysis)

**Language Features:**
- TIER1_IMPLEMENTATION_SUMMARY.md (built-in methods)
- METHOD_SYSTEM_ANALYSIS.md
- USER_DEFINED_TYPE_METHODS.md
- COLLECTION_TYPE_ALIAS_METHODS.md
- MUTABILITY_GUIDE.md
- PARAMETER_SEMANTICS.md
- WHATS_NEW.md

**Compiler & Implementation:**
- COMPILER_STATUS.md
- COMPILER_TODO_CHECKLIST.md
- COMPILER_COMPLETION_HANDOFF.md
- STRUCT_IMPLEMENTATION_STATUS.md
- STRUCT_IMPLEMENTATION_PLAN.md
- STRUCT_COMPILER_DESIGN.md
- PHASE5_COMPLETE.md
- PHASE6_HANDOFF.md
- SEMANTIC_REFACTORING_STATUS.md

**Planning:**
- FEATURE_ANALYSIS.md
- FEATURE_RECOMMENDATIONS.md
- DOCUMENTATION_INDEX.md

### Examples Organized (examples/mandelbrot/)
Moved 8 program files:

**Lift Programs:**
- mandelbrot_recursive.lt (60×30, works with JIT)
- mandelbrot_large.lt (200×100, best for benchmarking)
- mandelbrot_iterative.lt (reference only, has compiler bug)

**Python Programs:**
- mandelbrot.py (recursive, small)
- mandelbrot_large.py (recursive, large)
- mandelbrot_iterative.py (iterative, small)
- mandelbrot_large_iterative.py (iterative, large)

### Removed Files
Deleted 14 temporary test/debug files:
- mandelbrot_debug.lt
- mandelbrot_simple.lt
- mandelbrot_test.lt
- mandelbrot_tiny.lt
- mandelbrot_iter_tiny.lt
- test_call_in_while.lt
- test_iter.lt
- test_let_in_while.lt
- test_mini_mandel.lt
- test_nested_while.lt
- test_viz_mandel.lt
- test_viz_simple.lt
- test_while_and.lt
- test_while_compile.lt

### Files Kept in Root
- **README.md** - Main project documentation (kept as requested)
- **CLAUDE.md** - Development guidelines (kept as requested)
- Cargo.toml, build.rs, etc. (build configuration)

## Quick Access

### For Users
- **Start here**: [README.md](README.md)
- **Documentation**: [docs/README.md](docs/README.md)
- **Examples**: [examples/mandelbrot/README.md](examples/mandelbrot/README.md)

### For Developers
- **Development guide**: [CLAUDE.md](CLAUDE.md)
- **Compiler status**: [docs/COMPILER_STATUS.md](docs/COMPILER_STATUS.md)
- **Feature planning**: [docs/FEATURE_RECOMMENDATIONS.md](docs/FEATURE_RECOMMENDATIONS.md)

### For Performance Data
- **Main benchmarks**: [docs/LARGE_SCALE_BENCHMARKS.md](docs/LARGE_SCALE_BENCHMARKS.md)
- **Compiler results**: [docs/COMPILER_RESULTS.md](docs/COMPILER_RESULTS.md)
- **Complete summary**: [docs/MANDELBROT_SUMMARY.md](docs/MANDELBROT_SUMMARY.md)

## Key Performance Results

**Lift JIT Compiler (200×100 Mandelbrot):**
- 🥇 10ms - Fastest implementation
- 4x faster than Python iterative
- 13x faster than Python recursive
- 208x faster than Lift interpreter

See [docs/LARGE_SCALE_BENCHMARKS.md](docs/LARGE_SCALE_BENCHMARKS.md) for full analysis.
