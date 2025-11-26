# WASM Bundle Size Optimization

This document describes the bundle size optimization strategy for StreamWeave WASM builds.

## Optimization Settings

The following optimization settings are configured in `Cargo.toml`:

```toml
[profile.release]
lto = true              # Link-time optimization
codegen-units = 1       # Single codegen unit for better optimization
panic = "abort"         # Smaller panic handler
strip = true            # Strip debug symbols

[profile.wasm-release]
inherits = "release"
opt-level = "z"         # Optimize for size (not speed)
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## Bundle Size Targets

| Build Type | Feature Set | Target Size |
|------------|-------------|-------------|
| Minimal | `wasm-minimal` | < 200KB gzipped |
| Standard | `wasm` | < 500KB gzipped |

## Measuring Bundle Size

For library crates, bundle size depends on what code is actually used. To measure:

1. **Build the library:**
   ```bash
   cargo build --target wasm32-unknown-unknown --features wasm --no-default-features --release --lib
   ```

2. **Check optimization settings:**
   ```bash
   ./bin/wasm-build
   ```

3. **Build a minimal binary example:**
   ```bash
   cargo build --target wasm32-unknown-unknown --features wasm --no-default-features --release --example wasm_minimal
   ```

4. **Further optimize with wasm-opt (if installed):**
   ```bash
   wasm-opt -Oz -o optimized.wasm target/wasm32-unknown-unknown/release/examples/wasm_minimal.wasm
   gzip -c optimized.wasm | wc -c
   ```

## Size Optimization Techniques

1. **Feature Flags**: Use `wasm-minimal` for smallest bundle
2. **LTO**: Enabled by default for better dead code elimination
3. **Codegen Units**: Single unit allows better cross-module optimization
4. **Panic Abort**: Smaller than unwinding
5. **Strip Symbols**: Removes debug info
6. **wasm-opt**: Additional post-build optimization (recommended)

## CI Size Checks

The CI workflow includes WASM build verification:
- Build succeeds with WASM features
- Optimization settings are verified
- Build artifacts are checked

## Notes

- Library crate size (`rlib`) is an approximation
- Actual bundle size depends on:
  - Features enabled
  - Code actually used from the library
  - Dependencies included
  - Final binary's codegen settings

