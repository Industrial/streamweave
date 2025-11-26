# StreamWeave WASM Node.js Example

This example demonstrates how to use StreamWeave in a Node.js environment with WASM.

## Setup

1. **Install dependencies:**

```bash
npm install
```

2. **Build the WASM module:**

```bash
npm run build:wasm
```

3. **Run the example:**

```bash
npm start
```

Or use Node.js directly:

```bash
node src/index.js
```

## Project Structure

```
wasm_nodejs/
├── src/
│   ├── lib.rs          # Rust WASM module
│   └── index.js        # Node.js entry point
├── package.json        # Node.js dependencies
└── Cargo.toml          # Rust dependencies
```

## How It Works

1. The Rust code (`src/lib.rs`) defines a WASM module using StreamWeave
2. `wasm-pack` compiles it to WASM with Node.js bindings
3. Node.js loads and executes the WASM module
4. Results are displayed in the console

## Features Demonstrated

- Stream processing in Node.js
- Async/await with WASM
- Data transformation
- Error handling

## Building for Production

```bash
npm run build:wasm
```

The built WASM module is in the `pkg/` directory and can be published to npm.

