# Mandelbrot Set Visualizer Examples

This directory contains Mandelbrot set visualizers implemented in Lift and Python, demonstrating the performance characteristics of different approaches and implementations.

## Files

### Lift Implementations

#### Working Versions
- **`mandelbrot_recursive.lt`** - Recursive implementation (60×30, 50 iterations)
  - ✅ Works with interpreter (80ms)
  - ✅ **Works with JIT compiler (<1ms)** - RECOMMENDED
  - Uses functional recursion
  - Best for Lift's JIT compiler

- **`mandelbrot_large.lt`** - Large recursive version (200×100, 100 iterations)
  - ✅ Works with interpreter (2,080ms)
  - ✅ **Works with JIT compiler (10ms)** - RECOMMENDED FOR BENCHMARKING
  - 20,000 points for accurate performance measurements
  - **Fastest implementation overall!**

#### Reference Versions
- **`mandelbrot_iterative.lt`** - Iterative implementation (60×30, 50 iterations)
  - ✅ Code is correct
  - ❌ Interpreter too slow (>60s timeout)
  - ❌ JIT compiler has bug (let in while loops)
  - For reference only

### Python Implementations

#### Small Versions (60×30, 50 iterations)
- **`mandelbrot.py`** - Recursive implementation
  - Average: 13ms
  - Uses recursion like Lift

- **`mandelbrot_iterative.py`** - Iterative implementation
  - Average: 10ms
  - Standard imperative approach

#### Large Versions (200×100, 100 iterations)
- **`mandelbrot_large.py`** - Large recursive version
  - Average: 130ms (excluding warmup)
  - Good for comparison with Lift

- **`mandelbrot_large_iterative.py`** - Large iterative version
  - Average: 40ms
  - Python's fastest approach

## Usage

### Running Lift Versions

```bash
# Interpreter (slow but works everywhere)
cargo run --release -- examples/mandelbrot/mandelbrot_recursive.lt

# JIT Compiler (FAST! - recommended)
cargo run --release -- --compile examples/mandelbrot/mandelbrot_recursive.lt

# Large benchmark version
cargo run --release -- --compile examples/mandelbrot/mandelbrot_large.lt
```

### Running Python Versions

```bash
# Small versions
python3 examples/mandelbrot/mandelbrot.py
python3 examples/mandelbrot/mandelbrot_iterative.py

# Large versions
python3 examples/mandelbrot/mandelbrot_large.py
python3 examples/mandelbrot/mandelbrot_large_iterative.py
```

## Performance Results

### Small Scale (60×30, 50 iterations = 1,800 points)

| Implementation | Time | Status |
|----------------|------|--------|
| Lift JIT (Recursive) | <1ms | 🥇 Fastest |
| Python (Iterative) | 10ms | ✅ Good |
| Python (Recursive) | 13ms | ✅ Good |
| Lift Interpreter (Recursive) | 80ms | ✅ OK |

### Large Scale (200×100, 100 iterations = 20,000 points)

| Implementation | Time | vs Best |
|----------------|------|---------|
| **Lift JIT (Recursive)** | **10ms** | **1.0x (baseline)** |
| Python (Iterative) | 40ms | 4.0x slower |
| Python (Recursive) | 130ms | 13.0x slower |
| Lift Interpreter (Recursive) | 2,080ms | 208x slower |

## Key Insights

1. **Lift JIT is the fastest** - Beats Python by 4-13x
2. **Recursive works best in Lift** - Both interpreter and compiler support it well
3. **JIT provides massive speedup** - 208x faster than interpreter
4. **Use --compile flag** - Essential for production performance

## Benchmarking Tips

1. **Use the large versions** for accurate timing (small version is too fast to measure accurately)
2. **Run multiple times** to account for warmup/caching
3. **Redirect output** to `/dev/null` when benchmarking to avoid I/O overhead
4. **Use `/usr/bin/time -f`** for accurate timing measurements

Example benchmark command:
```bash
/usr/bin/time -f "Time: %E (user: %U, sys: %S)" \
  ./target/release/lift-lang --compile examples/mandelbrot/mandelbrot_large.lt > /dev/null
```

## Documentation

See [../../docs/](../../docs/) for detailed performance analysis:
- `LARGE_SCALE_BENCHMARKS.md` - Comprehensive benchmark results
- `COMPILER_RESULTS.md` - How the compiler was fixed
- `MANDELBROT_SUMMARY.md` - Complete project summary

## Output

All versions produce ASCII art visualizations of the Mandelbrot set:
- `*` = point is in the Mandelbrot set
- `.` = point escaped (not in set)

The characteristic bulbous fractal shape should be clearly visible in the output.
