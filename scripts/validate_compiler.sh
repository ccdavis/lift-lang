#!/bin/bash
# Cranelift Compiler Integration Test Script

FAILED=0
PASSED=0
SKIPPED=0

echo "=========================================="
echo "Cranelift Compiler Integration Tests"
echo "=========================================="
echo ""

for file in tests/*.lt; do
    filename=$(basename "$file")

    # Skip error test files (they're supposed to fail)
    if [[ "$filename" == *"error"* ]]; then
        echo "SKIP: $filename (error test)"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Skip files that use features not yet in compiler
    # (for loops, match expressions, closures)
    if [[ "$filename" == *"for_loop"* ]] || \
       [[ "$filename" == *"match"* ]] || \
       [[ "$filename" == *"closure"* ]]; then
        echo "SKIP: $filename (unsupported feature)"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    echo -n "Testing: $filename ... "

    # Run interpreter (ground truth)
    INTERP_OUT=$(cargo run --quiet -- "$file" 2>&1)
    INTERP_EXIT=$?

    # Run compiler
    COMP_OUT=$(cargo run --quiet -- --compile "$file" 2>&1)
    COMP_EXIT=$?

    # Compare outputs
    if [ "$INTERP_OUT" = "$COMP_OUT" ] && [ $INTERP_EXIT -eq $COMP_EXIT ]; then
        echo "✅ PASS"
        PASSED=$((PASSED + 1))
    else
        echo "❌ FAIL"
        echo "  Interpreter output: $INTERP_OUT"
        echo "  Compiler output:    $COMP_OUT"
        FAILED=$((FAILED + 1))
    fi
done

echo ""
echo "=========================================="
echo "Results: $PASSED passed, $FAILED failed, $SKIPPED skipped"
echo "=========================================="

if [ $FAILED -eq 0 ]; then
    echo "✅ ALL TESTS PASSED"
    exit 0
else
    echo "❌ SOME TESTS FAILED"
    exit 1
fi
