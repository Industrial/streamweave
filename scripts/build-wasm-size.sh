#!/usr/bin/env bash
# Build WASM with cdylib crate type for size measurement

set -euo pipefail

# Create a temporary Cargo.toml with cdylib for WASM
TEMP_CARGO=$(mktemp)
cat Cargo.toml > "$TEMP_CARGO"

# Add cdylib crate-type for WASM builds (using build script approach)
# Actually, we'll use RUSTFLAGS to override

# Build with cdylib
RUSTFLAGS="-C link-arg=--no-entry" cargo build \
  --target wasm32-unknown-unknown \
  --features wasm \
  --no-default-features \
  --release \
  --lib \
  -Z unstable-options \
  --crate-type cdylib 2>&1 || {
    echo "Failed to build as cdylib. Falling back to rlib size estimation."
    # Fall back to rlib
    cargo build \
      --target wasm32-unknown-unknown \
      --features wasm \
      --no-default-features \
      --release \
      --lib
    
    RLIB=$(find target/wasm32-unknown-unknown/release -name "libstreamweave*.rlib")
    SIZE=$(stat -c%s "$RLIB" 2>/dev/null || stat -f%z "$RLIB")
    echo "RLIB size: $((SIZE / 1024)) KB (approximation)"
    exit 0
  }

# Find WASM file
WASM_FILE=$(find target/wasm32-unknown-unknown/release -name "*.wasm" | head -1)

if [ -n "$WASM_FILE" ]; then
  SIZE=$(stat -c%s "$WASM_FILE" 2>/dev/null || stat -f%z "$WASM_FILE")
  GZIP_SIZE=$(gzip -c "$WASM_FILE" | wc -c)
  echo "$((GZIP_SIZE / 1024))"
else
  echo "0"
fi

