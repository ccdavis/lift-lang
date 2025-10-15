#!/bin/bash
# Benchmark script for Mandelbrot iterative implementations
# Compares Python, Ruby, Lift Interpreter, and Lift Compiler

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MANDELBROT_DIR="$PROJECT_ROOT/examples/mandelbrot"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo "=========================================="
echo "  Mandelbrot Benchmark Suite"
echo "=========================================="
echo ""

# Check if required programs are available
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}ERROR: $1 is not installed${NC}"
        return 1
    fi
    return 0
}

echo "Checking dependencies..."
MISSING_DEPS=0
check_command python3 || MISSING_DEPS=1
check_command ruby || MISSING_DEPS=1
check_command cargo || MISSING_DEPS=1

if [ $MISSING_DEPS -eq 1 ]; then
    echo -e "${RED}Please install missing dependencies${NC}"
    exit 1
fi
echo -e "${GREEN}All dependencies found${NC}"
echo ""

# Build Lift in release mode
echo "Building Lift (release mode)..."
cd "$PROJECT_ROOT"
cargo build --release --quiet 2>&1 | grep -v "warning:" || true
echo -e "${GREEN}Build complete${NC}"
echo ""

LIFT_BIN="$PROJECT_ROOT/target/release/lift-lang"
LIFT_PROGRAM="$MANDELBROT_DIR/mandelbrot_iterative.lt"
PYTHON_PROGRAM="$MANDELBROT_DIR/mandelbrot_iterative.py"
RUBY_PROGRAM="$MANDELBROT_DIR/mandelbrot_iterative.rb"

# Make Ruby script executable
chmod +x "$RUBY_PROGRAM" 2>/dev/null || true

# Number of runs for averaging
RUNS=5

echo "Configuration:"
echo "  - Program: Mandelbrot Set (60x30, 50 iterations)"
echo "  - Runs per implementation: $RUNS"
echo "  - Output: Suppressed (timing only)"
echo ""
echo "=========================================="
echo ""

# Function to run benchmark
run_benchmark() {
    local name="$1"
    local cmd="$2"
    local color="$3"

    echo -e "${color}Testing: $name${NC}"

    local total_time=0
    local times=()

    for i in $(seq 1 $RUNS); do
        # Run and capture time (redirecting output to /dev/null)
        local start=$(date +%s%N)
        eval "$cmd" > /dev/null 2>&1
        local end=$(date +%s%N)

        # Calculate elapsed time in milliseconds
        local elapsed=$(( (end - start) / 1000000 ))
        times+=($elapsed)
        total_time=$((total_time + elapsed))

        echo -e "  Run $i: ${elapsed}ms"
    done

    # Calculate average
    local avg=$((total_time / RUNS))

    # Calculate min and max
    local min=${times[0]}
    local max=${times[0]}
    for time in "${times[@]}"; do
        if [ $time -lt $min ]; then min=$time; fi
        if [ $time -gt $max ]; then max=$time; fi
    done

    echo -e "${color}  Average: ${avg}ms  (min: ${min}ms, max: ${max}ms)${NC}"
    echo ""

    # Return average for comparison
    echo $avg
}

# Run benchmarks
echo "Starting benchmarks..."
echo ""

# Python
python_avg=$(run_benchmark "Python 3" "python3 '$PYTHON_PROGRAM'" "$YELLOW")

# Ruby
ruby_avg=$(run_benchmark "Ruby" "ruby '$RUBY_PROGRAM'" "$RED")

# Lift Interpreter
lift_interp_avg=$(run_benchmark "Lift Interpreter" "'$LIFT_BIN' '$LIFT_PROGRAM'" "$CYAN")

# Lift Compiler
lift_comp_avg=$(run_benchmark "Lift Compiler (JIT)" "'$LIFT_BIN' --compile '$LIFT_PROGRAM'" "$GREEN")

# Summary
echo "=========================================="
echo "  SUMMARY"
echo "=========================================="
echo ""

printf "%-25s %10s\n" "Implementation" "Avg Time"
printf "%-25s %10s\n" "-------------------------" "----------"
printf "%-25s %10sms\n" "Python 3" "$python_avg"
printf "%-25s %10sms\n" "Ruby" "$ruby_avg"
printf "%-25s %10sms\n" "Lift Interpreter" "$lift_interp_avg"
printf "%-25s %10sms\n" "Lift Compiler (JIT)" "$lift_comp_avg"
echo ""

# Calculate speedups relative to Python
echo "Speedup vs Python:"
python_vs_python=$(echo "scale=2; $python_avg / $python_avg" | bc)
ruby_vs_python=$(echo "scale=2; $python_avg / $ruby_avg" | bc)
lift_interp_vs_python=$(echo "scale=2; $python_avg / $lift_interp_avg" | bc)
lift_comp_vs_python=$(echo "scale=2; $python_avg / $lift_comp_avg" | bc)

printf "  %-23s %8.2fx\n" "Python 3" "$python_vs_python"
printf "  %-23s %8.2fx\n" "Ruby" "$ruby_vs_python"
printf "  %-23s %8.2fx\n" "Lift Interpreter" "$lift_interp_vs_python"
printf "  %-23s %8.2fx\n" "Lift Compiler (JIT)" "$lift_comp_vs_python"
echo ""

# Calculate speedup of compiler vs interpreter
lift_comp_vs_interp=$(echo "scale=2; $lift_interp_avg / $lift_comp_avg" | bc)
echo "Lift Compiler vs Interpreter:"
printf "  Speedup: %8.2fx faster\n" "$lift_comp_vs_interp"
echo ""

echo "=========================================="
echo -e "${GREEN}Benchmark complete!${NC}"
echo "=========================================="
