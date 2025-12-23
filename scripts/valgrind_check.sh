#!/bin/bash
# Valgrind memory check script for Lift language
# Usage: ./scripts/valgrind_check.sh [lift_file.lt]
#
# Runs valgrind with the Lift suppression file to check for memory leaks.
# The suppression file filters out expected Cranelift JIT allocations.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SUPP_FILE="$PROJECT_DIR/lift.supp"

# Build release if needed
if [ ! -f "$PROJECT_DIR/target/release/lift-lang" ]; then
    echo "Building release binary..."
    cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"
fi

LIFT_BINARY="$PROJECT_DIR/target/release/lift-lang"

if [ -z "$1" ]; then
    echo "Usage: $0 <lift_file.lt>"
    echo ""
    echo "Example:"
    echo "  $0 tests/test_refcount_basic.lt"
    echo "  $0 examples/mandelbrot/mandelbrot_recursive.lt"
    exit 1
fi

LIFT_FILE="$1"

if [ ! -f "$LIFT_FILE" ]; then
    echo "Error: File not found: $LIFT_FILE"
    exit 1
fi

echo "Running valgrind on: $LIFT_FILE"
echo "Using suppressions: $SUPP_FILE"
echo "----------------------------------------"

valgrind \
    --leak-check=full \
    --show-leak-kinds=all \
    --track-origins=yes \
    --suppressions="$SUPP_FILE" \
    "$LIFT_BINARY" --compile "$LIFT_FILE"

echo "----------------------------------------"
echo "Valgrind check complete."
