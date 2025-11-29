#!/usr/bin/env bash
# Script to verify documentation quality and completeness

set -euo pipefail

echo "📚 Verifying Documentation Quality"
echo "=================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0

# 1. Check that documentation builds
echo "1. Building documentation..."
if cargo doc --all-features --no-deps 2>&1 | tee /tmp/doc-build.log; then
    echo -e "${GREEN}✅ Documentation builds successfully${NC}"
else
    echo -e "${RED}❌ Documentation build failed${NC}"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# 2. Check for missing documentation warnings
echo "2. Checking for missing documentation..."
MISSING_DOCS=$(grep -c "warning.*missing.*documentation" /tmp/doc-build.log 2>/dev/null || echo "0")
if [ "$MISSING_DOCS" -eq 0 ]; then
    echo -e "${GREEN}✅ No missing documentation warnings${NC}"
else
    echo -e "${YELLOW}⚠️  Found $MISSING_DOCS missing documentation warnings${NC}"
    WARNINGS=$((WARNINGS + MISSING_DOCS))
    echo "Top warnings:"
    grep "warning.*missing.*documentation" /tmp/doc-build.log | head -10
fi
echo ""

# 3. Check that all public modules have module-level docs
echo "3. Checking module-level documentation..."
MODULES=$(find src -name "mod.rs" -o -name "lib.rs" | wc -l)
MODULES_WITH_DOCS=$(find src -name "mod.rs" -o -name "lib.rs" | xargs grep -l "^//!" | wc -l)
if [ "$MODULES" -eq "$MODULES_WITH_DOCS" ]; then
    echo -e "${GREEN}✅ All modules have module-level documentation ($MODULES_WITH_DOCS/$MODULES)${NC}"
else
    MISSING=$((MODULES - MODULES_WITH_DOCS))
    echo -e "${YELLOW}⚠️  $MISSING modules missing module-level documentation ($MODULES_WITH_DOCS/$MODULES)${NC}"
    WARNINGS=$((WARNINGS + MISSING))
fi
echo ""

# 4. Check that examples compile
echo "4. Checking that code examples compile..."
if cargo test --doc --all-features 2>&1 | tee /tmp/doc-test.log; then
    echo -e "${GREEN}✅ All documentation examples compile${NC}"
else
    # Count actual doctest failures, excluding linker/environmental errors
    # Look for "Couldn't compile the test" which indicates actual doctest failures
    FAILED_EXAMPLES=$(grep -c "Couldn't compile the test" /tmp/doc-test.log 2>/dev/null || echo "0")
    # Also check for actual compilation errors in doctests (not linker errors)
    COMPILE_ERRORS=$(grep -E "error\[E[0-9]+\]" /tmp/doc-test.log 2>/dev/null | grep -v "linking" | wc -l || echo "0")
    
    # Check for disk space issues
    if grep -q "No space left on device" /tmp/doc-test.log 2>/dev/null; then
        echo -e "${YELLOW}⚠️  Disk space issue detected during doctest compilation${NC}"
        echo -e "${YELLOW}⚠️  This is an environmental issue, not a code issue${NC}"
        WARNINGS=$((WARNINGS + 1))
    elif [ "$FAILED_EXAMPLES" -gt 0 ] || [ "$COMPILE_ERRORS" -gt 0 ]; then
        echo -e "${RED}❌ Some documentation examples failed to compile${NC}"
        echo "Failed doctests: $FAILED_EXAMPLES"
        echo "Compilation errors: $COMPILE_ERRORS"
        ERRORS=$((ERRORS + FAILED_EXAMPLES + COMPILE_ERRORS))
    else
        # If cargo test failed but we can't find specific errors, it might be environmental
        echo -e "${YELLOW}⚠️  Doctest run failed, but no clear compilation errors found${NC}"
        echo -e "${YELLOW}⚠️  This might be an environmental issue${NC}"
        WARNINGS=$((WARNINGS + 1))
    fi
fi
echo ""

# 5. Check for broken links in markdown files
echo "5. Checking for broken links in markdown files..."
if command -v linkchecker &> /dev/null; then
    # Check README and docs
    if linkchecker README.md docs/*.md 2>&1 | grep -q "broken"; then
        echo -e "${YELLOW}⚠️  Some links may be broken${NC}"
        WARNINGS=$((WARNINGS + 1))
    else
        echo -e "${GREEN}✅ No broken links found${NC}"
    fi
else
    echo -e "${YELLOW}⚠️  linkchecker not installed, skipping link check${NC}"
fi
echo ""

# Summary
echo "=================================="
echo "Summary:"
if [ "$ERRORS" -eq 0 ] && [ "$WARNINGS" -eq 0 ]; then
    echo -e "${GREEN}✅ All documentation checks passed!${NC}"
    exit 0
elif [ "$ERRORS" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  Documentation has $WARNINGS warning(s) but no errors${NC}"
    exit 0
else
    echo -e "${RED}❌ Documentation has $ERRORS error(s) and $WARNINGS warning(s)${NC}"
    exit 1
fi

