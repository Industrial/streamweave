# Documentation Coverage Metrics

This document describes how to track and improve documentation coverage for StreamWeave.

## Overview

Documentation coverage measures how well the codebase is documented. This includes:
- Public API documentation
- Code examples in documentation
- Module-level documentation
- Guide completeness

## Metrics to Track

### 1. Public API Documentation Coverage

Track the percentage of public items (types, functions, methods) that have doc comments.

**How to measure:**
```bash
# Count public items with documentation
cargo doc --all-features --no-deps 2>&1 | grep -c "warning.*missing.*documentation" || echo "0"

# Or use cargo-doc-coverage if available
cargo install cargo-doc-coverage
cargo doc-coverage
```

**Target:** >95% of public APIs should have documentation.

### 2. Code Example Coverage

Track the percentage of public APIs that include code examples in their documentation.

**How to measure:**
- Manual review of generated documentation
- Check for `# Example` sections in doc comments
- Use tools that parse doc comments for example blocks

**Target:** >80% of public APIs should have examples.

### 3. Module Documentation Coverage

Track the percentage of public modules that have module-level documentation (`//!`).

**How to measure:**
```bash
# Count modules with //! documentation
grep -r "^//!" src/ | wc -l
```

**Target:** 100% of public modules should have module-level docs.

### 4. Guide Completeness

Track which guides exist and their completeness:
- [x] Getting Started Guide
- [x] Architecture Overview
- [x] Common Use Cases
- [x] Troubleshooting Guide
- [ ] Advanced Patterns (planned)
- [ ] Performance Guide (planned)
- [ ] Migration Guide (planned)

## Automated Coverage Reports

### Using cargo-doc-coverage

If available, install and use:
```bash
cargo install cargo-doc-coverage
cargo doc-coverage --output coverage.json
```

### Custom Script

Create a script to generate coverage reports:

```bash
#!/bin/bash
# scripts/doc-coverage.sh

echo "Documentation Coverage Report"
echo "=============================="
echo ""

# Count total public items
TOTAL=$(cargo doc --all-features --no-deps 2>&1 | grep -c "warning.*missing" || echo "0")

# Count documented items
DOCUMENTED=$(find src -name "*.rs" -exec grep -l "^///" {} \; | wc -l)

# Calculate percentage
if [ "$TOTAL" -gt 0 ]; then
    PERCENTAGE=$((DOCUMENTED * 100 / TOTAL))
    echo "Coverage: $PERCENTAGE%"
else
    echo "Coverage: 100%"
fi
```

## CI/CD Integration

Add documentation coverage checks to CI:

```yaml
# .github/workflows/ci.yml
docs-coverage:
  name: 📊 Documentation Coverage
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - name: Check documentation coverage
      run: |
        cargo doc --all-features --no-deps 2>&1 | tee doc-warnings.txt
        WARNINGS=$(grep -c "warning.*missing" doc-warnings.txt || echo "0")
        if [ "$WARNINGS" -gt 10 ]; then
          echo "⚠️ Too many undocumented items: $WARNINGS"
          exit 1
        fi
```

## Improving Coverage

### Priority Order

1. **Public APIs** - Start with the most commonly used APIs
2. **Complex Types** - Document types that are hard to understand
3. **Error Types** - Document all error variants and when they occur
4. **Examples** - Add examples to frequently used APIs
5. **Module Docs** - Add overview documentation to each module

### Documentation Standards

Follow these standards when adding documentation:

- Use `///` for public items
- Use `//!` for module-level documentation
- Include examples in doc comments
- Document error conditions
- Explain the "why" not just the "what"
- Keep documentation up to date with code changes

## Periodic Audits

Schedule periodic documentation audits:

- **Monthly**: Review new APIs for documentation
- **Quarterly**: Full documentation audit
- **Before Releases**: Ensure all new features are documented
- **After Major Refactors**: Update documentation to match new structure

## Tools and Resources

- `cargo doc` - Generate and view documentation
- `cargo-doc-coverage` - Measure documentation coverage (if available)
- `doxidize` - Alternative documentation generator
- GitHub Actions - Automated coverage tracking

## Goals

- **Short-term (1 month)**: >90% public API coverage
- **Medium-term (3 months)**: >95% public API coverage, >70% example coverage
- **Long-term (6 months)**: >95% public API coverage, >80% example coverage, all guides complete

