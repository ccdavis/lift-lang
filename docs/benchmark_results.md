# Mandelbrot Set Visualizer - Performance Comparison

## Test Configuration
- **Resolution**: 60x30 characters (1,800 points)
- **Max Iterations**: 50
- **Algorithm**: Recursive implementation (both languages)
- **Hardware**: Linux on WSL2

## Implementation Details

Both implementations use identical algorithms:
- Recursive Mandelbrot calculation
- Recursive row building
- Recursive rendering

### Lift Implementation (`mandelbrot_recursive.lt`)
- Pure functional approach with tail recursion
- Tree-walking interpreter
- Type-safe execution

### Python Implementation (`mandelbrot.py`)
- Direct translation of Lift code
- Uses Python 3 with increased recursion limit
- Native CPython execution

## Benchmark Results

### Lift (Tree-Walking Interpreter)
```
Run 1: 0:00.08 (user: 0.08s, sys: 0.00s)
Run 2: 0:00.08 (user: 0.08s, sys: 0.00s)
Run 3: 0:00.08 (user: 0.08s, sys: 0.00s)
Average: 80ms
```

### Python 3
```
Run 1: 0:00.01 (user: 0.00s, sys: 0.01s)
Run 2: 0:00.01 (user: 0.00s, sys: 0.01s)
Run 3: 0:00.01 (user: 0.01s, sys: 0.00s)
Average: 10ms
```

## Performance Analysis

**Python is ~8x faster than Lift interpreter**

This is expected because:
1. **CPython is highly optimized** with decades of performance work
2. **Lift uses a tree-walking interpreter** which has overhead for:
   - AST node traversal
   - Runtime type checking
   - Symbol table lookups
   - Function call overhead

3. **Python has optimized built-ins** for arithmetic operations
4. **Lift is a young language** focused on correctness first

## Output Verification

Both implementations produce **identical output** (after trimming whitespace):
```
............................................................
............................................................
......................................*.....................
....................................****....................
....................................****....................
..................................*..**.....................
..............................*..**********.................
.............................******************.............
.............................*****************..............
............................*******************.............
...........................**********************...........
.................*..*......*********************............
.................*******..**********************............
................*********.**********************............
.............*..*********.**********************............
.*********************************************..............
.............*..*********.**********************............
................*********.**********************............
.................*******..**********************............
.................*..*......*********************............
...........................**********************...........
............................*******************.............
.............................*****************..............
.............................******************.............
..............................*..**********.................
..................................*..**.....................
....................................****....................
....................................****....................
......................................*.....................
............................................................
```

The characteristic Mandelbrot shape is clearly visible!

## Future Improvements

### For Lift Performance:
1. **JIT Compilation**: Lift has a Cranelift-based JIT compiler (--compile flag)
   - Expected 10-50x speedup over interpreter
   - Would likely match or exceed Python performance
   
2. **Bytecode VM**: Replace tree-walking with bytecode interpreter
   - 3-5x speedup expected

3. **Tail Call Optimization**: Better handling of recursive calls
   - Reduce stack overhead

### Note on Recursion:
The recursive approach was necessary because Lift's while loops with mutable state had performance issues. The recursive approach is actually quite elegant and demonstrates Lift's functional programming capabilities.

## Conclusion

Despite being 8x slower than Python in interpreted mode, Lift successfully computes and renders the Mandelbrot set with:
- ✅ Correct output matching Python
- ✅ Clean, functional code
- ✅ Type safety
- ✅ Reasonable performance for an interpreter (~80ms)

With JIT compilation enabled, Lift would likely match or exceed Python's performance while maintaining its strong type system and functional programming features.
