# StreamWeave WASM Browser Example

This example demonstrates how to use StreamWeave in a browser environment with Vite.

## Setup

1. **Install dependencies:**

```bash
npm install
```

2. **Build the WASM module:**

```bash
# This will build the WASM module using wasm-pack
npm run build:wasm
```

3. **Start the development server:**

```bash
npm run dev
```

4. **Open your browser:**

Navigate to `http://localhost:5173` (or the port shown in the terminal)

## Project Structure

```
wasm_browser/
├── src/
│   ├── lib.rs          # Rust WASM module
│   ├── main.js         # JavaScript entry point
│   └── index.html      # HTML page
├── package.json        # Node.js dependencies
└── vite.config.js      # Vite configuration
```

## How It Works

1. The Rust code (`src/lib.rs`) defines a WASM module that uses StreamWeave
2. `wasm-pack` compiles it to WASM and generates JavaScript bindings
3. Vite serves the application and handles WASM loading
4. The browser executes the WASM module and displays results

## Features Demonstrated

- Basic stream processing in WASM
- Data transformation (map, filter)
- Result collection and display
- Async/await with WASM

## Building for Production

```bash
npm run build
npm run preview
```

This creates an optimized production build with tree-shaking and minification.

