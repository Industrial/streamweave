# WASM Getting Started Guide

This guide will help you get started with StreamWeave in WebAssembly environments, including browsers and Node.js.

## Overview

StreamWeave supports WebAssembly (WASM) through feature flags, allowing you to build stream processing pipelines that run in:
- Modern web browsers
- Node.js environments
- Edge computing platforms
- Serverless WASM runtimes

## Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-pack` (optional, for easier WASM builds)
- Node.js 16+ (for Node.js examples)
- Modern browser (for browser examples)

## Installation

### Adding wasm32 Target

```bash
rustup target add wasm32-unknown-unknown
```

### Building for WASM

```bash
# Standard WASM build
cargo build --target wasm32-unknown-unknown --features wasm --no-default-features --release --lib

# Minimal WASM build (smallest size)
cargo build --target wasm32-unknown-unknown --features wasm-minimal --no-default-features --release --lib
```

## Feature Flags

StreamWeave provides two WASM feature sets:

### `wasm` (Standard)
Includes core stream processing with minimal tokio features:
- `tokio/sync`
- `tokio/macros`
- `tokio/rt`
- `tokio/time`

**Target bundle size:** < 500KB gzipped

### `wasm-minimal` (Minimal)
Smallest possible build for maximum compatibility:
- `tokio/sync`
- `tokio/rt`
- `tokio/time`
- `tokio/macros`

**Target bundle size:** < 200KB gzipped

## Browser Usage

### Using wasm-pack (Recommended)

1. **Create a new Rust project:**

```bash
cargo new --lib my-streamweave-wasm
cd my-streamweave-wasm
```

2. **Add StreamWeave as a dependency:**

```toml
[dependencies]
streamweave = { version = "0.1", features = ["wasm"], default-features = false }
wasm-bindgen = "0.2"
```

3. **Create your WASM module:**

```rust
use wasm_bindgen::prelude::*;
use streamweave::pipeline::Pipeline;
use streamweave::producers::range::RangeProducer;
use streamweave::transformers::map::MapTransformer;
use streamweave::consumers::vec::VecConsumer;

#[wasm_bindgen]
pub fn process_stream() -> Vec<u32> {
    // Create a pipeline
    let pipeline = Pipeline::<u32, u32>::new()
        .with_producer(RangeProducer::new(1, 10))
        .with_transformer(MapTransformer::new(|x| x * 2))
        .with_consumer(VecConsumer::new());
    
    // Run the pipeline (synchronous for WASM)
    // Note: Async execution requires wasm-bindgen-futures
    todo!() // Implementation depends on your async setup
}
```

4. **Build with wasm-pack:**

```bash
wasm-pack build --target web --features wasm --no-default-features
```

5. **Use in HTML:**

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>StreamWeave WASM Example</title>
</head>
<body>
    <script type="module">
        import init, { process_stream } from './pkg/my_streamweave_wasm.js';
        
        async function run() {
            await init();
            const result = process_stream();
            console.log('Result:', result);
        }
        
        run();
    </script>
</body>
</html>
```

### Using Vite

1. **Create a Vite project:**

```bash
npm create vite@latest my-streamweave-app -- --template vanilla
cd my-streamweave-app
```

2. **Add Rust dependencies:**

```toml
[dependencies]
streamweave = { version = "0.1", features = ["wasm"], default-features = false }
wasm-bindgen = "0.2"
```

3. **Build with wasm-pack:**

```bash
wasm-pack build --target web --out-dir ./src/pkg
```

4. **Import and use in your JavaScript:**

```javascript
import init, { process_stream } from './pkg/my_streamweave_wasm.js';

async function main() {
    await init();
    const result = process_stream();
    console.log('Stream processing result:', result);
}

main();
```

## Node.js Usage

### Basic Example

1. **Create a Node.js project:**

```bash
mkdir streamweave-nodejs
cd streamweave-nodejs
npm init -y
```

2. **Build WASM for Node.js:**

```bash
wasm-pack build --target nodejs --features wasm --no-default-features
```

3. **Use in Node.js:**

```javascript
const { process_stream } = require('./pkg/streamweave_wasm');

const result = process_stream();
console.log('Result:', result);
```

### Async/Await Example

For async operations, use `wasm-bindgen-futures`:

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
pub async fn process_async() -> JsValue {
    // Your async pipeline code here
    JsValue::from_str("result")
}
```

```javascript
import init, { process_async } from './pkg/streamweave_wasm.js';

await init();
const result = await process_async();
```

## Performance Tips

### 1. Use Minimal Features

Use `wasm-minimal` feature set for smallest bundle size:

```toml
streamweave = { version = "0.1", features = ["wasm-minimal"], default-features = false }
```

### 2. Enable Optimization

Build with release profile and use `wasm-opt`:

```bash
# Build optimized WASM
cargo build --target wasm32-unknown-unknown --features wasm --no-default-features --release

# Further optimize with wasm-opt
wasm-opt -Oz -o optimized.wasm target/wasm32-unknown-unknown/release/your_module.wasm
```

### 3. Lazy Load WASM

Load WASM only when needed to reduce initial page load:

```javascript
async function loadWasm() {
    const { default: init, process_stream } = await import('./pkg/streamweave_wasm.js');
    await init();
    return { process_stream };
}
```

### 4. Use Web Workers

For heavy processing, offload to Web Workers:

```javascript
// worker.js
import init, { process_stream } from './pkg/streamweave_wasm.js';

self.onmessage = async (e) => {
    await init();
    const result = process_stream(e.data);
    self.postMessage(result);
};
```

### 5. Batch Processing

Process data in batches to avoid blocking the main thread:

```rust
#[wasm_bindgen]
pub fn process_batch(data: &[u32], batch_size: usize) -> Vec<u32> {
    // Process in batches
    data.chunks(batch_size)
        .flat_map(|chunk| {
            // Process each chunk
            chunk.iter().map(|x| x * 2).collect::<Vec<_>>()
        })
        .collect()
}
```

## Common Issues

### Issue: "wasm target is unsupported by mio"

**Solution:** Ensure you're using `--features wasm --no-default-features` to exclude native-only features.

### Issue: "getrandom wasm_js configuration flag required"

**Solution:** If you need random number generation in WASM, you'll need to use browser crypto APIs or Node.js crypto module. StreamWeave's `random` feature is not available in WASM.

### Issue: Large bundle size

**Solutions:**
- Use `wasm-minimal` features instead of `wasm`
- Run `wasm-opt -Oz` on the generated WASM file
- Check that you're building with `--release`
- Remove unused dependencies

### Issue: Async/await not working

**Solution:** Use `wasm-bindgen-futures` to bridge Rust futures with JavaScript promises:

```toml
[dependencies]
wasm-bindgen-futures = "0.4"
```

### Issue: Memory errors

**Solution:**
- Increase WASM memory limit if needed
- Process data in smaller batches
- Use streaming/chunked processing

## Limitations in WASM

The following features are **not available** in WASM builds:

1. **File I/O**: FileProducer, FileConsumer, CSV/JSONL/Parquet file operations
2. **Process Spawning**: CommandProducer, CommandConsumer
3. **Random Number Generation**: RandomNumberProducer, SampleTransformer (unless using browser APIs)
4. **Network Operations**: Native networking (use browser fetch/WebSocket APIs instead)
5. **File-based State**: FileOffsetStore (use in-memory stores only)

## Migration Guide

### From Native to WASM

1. **Replace file operations:**
   ```rust
   // Native
   FileProducer::new("data.csv")?;
   
   // WASM - use fetch API or pass data directly
   ArrayProducer::new(data_vec);
   ```

2. **Use in-memory stores:**
   ```rust
   // Native
   FileOffsetStore::new("offsets.json")?;
   
   // WASM
   InMemoryOffsetStore::new();
   ```

3. **Remove command operations:**
   ```rust
   // Not available in WASM
   // CommandProducer::new("ls", &[])?;
   ```

## Examples

See the `examples/` directory for complete working examples:

- **Browser Example** (`examples/wasm_browser/`): Complete Vite-based browser example with HTML/JS integration
  - Demonstrates WASM module structure for browsers
  - Shows how to integrate with Vite build system
  - Includes example functions for number and string processing

- **Node.js Example** (`examples/wasm_nodejs/`): Node.js WASM integration example
  - Shows WASM module structure for Node.js
  - Demonstrates synchronous and async patterns
  - Includes example functions for stream processing

Both examples are structured as templates that you can adapt for your own use cases. They demonstrate the basic patterns for:
- Setting up wasm-pack builds
- Creating WASM bindings with wasm-bindgen
- Integrating with build tools (Vite for browsers, npm for Node.js)
- Calling WASM functions from JavaScript

## Further Reading

- [WASM Compatibility](./WASM_COMPATIBILITY.md) - Detailed compatibility matrix
- [Bundle Size Optimization](./BUNDLE_SIZE.md) - Size optimization guide
- [wasm-bindgen guide](https://rustwasm.github.io/wasm-bindgen/)
- [wasm-pack documentation](https://rustwasm.github.io/wasm-pack/)

