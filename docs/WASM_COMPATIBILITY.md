# WASM Compatibility Audit for StreamWeave

This document details the WASM compatibility status of StreamWeave, identified blockers, and the plan for full WASM support.

## Summary

**Current Status**: ✅ WASM-compatible with `--features wasm --no-default-features`

**Implementation**:
- Feature flags split functionality between native and WASM
- Native-only modules conditionally compiled
- Core stream processing works in WASM

**Limitations in WASM**:
1. No file I/O (FileProducer, FileConsumer, etc.)
2. No process spawning (CommandProducer)
3. No random number generation (SampleTransformer, RandomNumberProducer)
4. No UUID-based message IDs
5. No JSON Schema validation

## Dependency Compatibility Matrix

| Dependency | Native | WASM | Notes |
|------------|--------|------|-------|
| tokio (full) | ✅ | ❌ | Needs feature split; no "full" in WASM |
| tokio (sync, time) | ✅ | ✅ | These features work in WASM |
| futures | ✅ | ✅ | Pure Rust, fully compatible |
| async-trait | ✅ | ✅ | Pure Rust macro |
| async-compression | ✅ | ⚠️ | Needs tokio features adjusted |
| serde / serde_json | ✅ | ✅ | Pure Rust |
| csv | ✅ | ✅ | Pure Rust |
| rmp-serde | ✅ | ✅ | Pure Rust |
| arrow | ✅ | ⚠️ | Has WASM support with cfg flags |
| parquet | ✅ | ⚠️ | Native compression libs need adjustment |
| jsonschema | ✅ | ⚠️ | May have issues with regex |
| chrono | ✅ | ✅ | Pure Rust (with serde feature) |
| rand | ✅ | ⚠️ | Needs getrandom wasm_js feature |
| uuid | ✅ | ⚠️ | Needs getrandom wasm_js feature |
| pin-project | ✅ | ✅ | Pure Rust macro |
| async-stream | ✅ | ✅ | Pure Rust |
| hyper-util | ✅ | ❌ | tokio networking dependent |
| hyper-tungstenite | ✅ | ❌ | WebSocket with mio |
| jsonwebtoken | ✅ | ⚠️ | May have ring crypto issues |
| regex | ✅ | ✅ | Pure Rust |
| sha2 | ✅ | ✅ | Pure Rust |
| thiserror | ✅ | ✅ | Pure Rust |
| tokio-stream | ✅ | ⚠️ | Depends on tokio features |
| tokio-tungstenite | ✅ | ❌ | WebSocket with mio |
| tracing | ✅ | ✅ | Pure Rust |
| tracing-subscriber | ✅ | ⚠️ | Some features may not work |

**Legend**: ✅ Works | ⚠️ Conditional | ❌ Doesn't work

## Module Compatibility

### Core (WASM-Compatible)
These modules should work in WASM with proper feature flags:

- **Producers**: `array`, `channel`, `range`, `string`, `hash_map`, `hash_set`, `interval`
- **Transformers**: `map`, `filter`, `reduce`, `flat_map`, `group_by`, `distinct`, `sample`, `join`, `broadcast`, `round_robin`, `router`, `ordered_merge`
- **Consumers**: `vec`, `array`, `string`, `hash_map`, `hash_set`, `channel`
- **Stateful Processing**: In-memory state stores only
- **Windowing**: All window types (without persistent state)
- **Pipeline Builder**: Full builder pattern

### Native-Only (Not WASM-Compatible)
These modules require OS-level features unavailable in browsers:

- **Producers**: `command` (process spawning), `file` (filesystem), `env_var` (environment), `parquet` (file I/O), `csv` (file I/O), `jsonl` (file I/O), `msgpack` (file I/O)
- **Consumers**: `command`, `file`, `console` (stdout), `parquet`, `csv`, `jsonl`, `msgpack`
- **WebSocket**: Native WebSocket implementations (need browser APIs instead)
- **File-based state/offset stores**: `FileOffsetStore`

## Implemented Feature Flags

### Cargo.toml Features (Implemented)

```toml
[features]
default = ["native"]

# Native platform features (full functionality)
native = [
    "tokio/full",
    "tokio-stream/io-util",
    "dep:async-compression",
    "dep:hyper-tungstenite",
    "dep:hyper-util",
    "dep:jsonschema",
    "dep:tokio-tungstenite",
    "dep:tokio-util",
    "dep:tracing-subscriber",
    "file-formats",
    "random",
]

# WebAssembly platform features (browser/Node.js compatible)
wasm = [
    "tokio/sync",
    "tokio/macros",
    "tokio/rt",
    "tokio/time",
]

# Optional file format support (native only)
file-formats = ["dep:arrow", "dep:parquet", "dep:csv", "dep:rmp-serde"]

# Random number generation support
random = ["dep:rand", "dep:uuid"]

# Compression support (native only)
compression = ["dep:async-compression"]
```

### Conditional Compilation (Implemented)

```rust
// In producers/mod.rs
#[cfg(not(target_arch = "wasm32"))]
pub mod command;
#[cfg(not(target_arch = "wasm32"))]
pub mod file;

#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub mod csv;
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub mod jsonl;

#[cfg(feature = "random")]
pub mod random_number;
```

### Building for WASM

```bash
# Build for WASM
cargo build --target wasm32-unknown-unknown --features wasm --no-default-features

# Check WASM compatibility
cargo check --target wasm32-unknown-unknown --features wasm --no-default-features
```

### Future WASM Enhancements

For browser environments, consider providing:
- `wasm-bindgen-futures` for async integration
- Browser `fetch` API for HTTP
- `WebSocket` API for real-time connections
- `localStorage`/`IndexedDB` for persistence

## Testing Strategy

1. **CI Integration**: Add `wasm32-unknown-unknown` target to CI
2. **Feature Matrix**: Test both `native` and `wasm` features
3. **Browser Tests**: Use `wasm-pack test` for browser testing
4. **Bundle Size**: Monitor and track WASM bundle size

## Bundle Size Goals

| Build Type | Target Size |
|------------|-------------|
| Core (minimal) | < 200KB gzipped |
| Full WASM | < 500KB gzipped |

## Implementation Phases

### Phase 1: Feature Flags (Subtask 8.2)
- Add `native` and `wasm` feature flags
- Make native-only dependencies optional
- Conditional compilation for platform-specific code

### Phase 2: Bundle Optimization (Subtask 8.3)
- Enable LTO for release builds
- Use `wasm-opt` for size optimization
- Tree shaking for unused code

### Phase 3: Documentation & Examples (Subtask 8.4)
- WASM getting started guide
- Browser example with Vite/webpack
- Node.js WASM example
- Performance tips for WASM

## Error Messages Encountered

```
error: This wasm target is unsupported by mio. If using Tokio, disable the net feature.
  --> mio-1.0.4/src/lib.rs:44:1

error: The wasm32-unknown-unknown targets are not supported by default; 
  you may need to enable the "wasm_js" configuration flag.
  --> getrandom-0.3.3/src/backends.rs:168:9
```

## References

- [getrandom WASM support](https://docs.rs/getrandom/latest/getrandom/#webassembly-support)
- [tokio WASM considerations](https://docs.rs/tokio/latest/tokio/#wasm-support)
- [wasm-bindgen guide](https://rustwasm.github.io/wasm-bindgen/)
- [wasm-pack documentation](https://rustwasm.github.io/wasm-pack/)

