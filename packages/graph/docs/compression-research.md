# Rust Compression Libraries Research

## Overview

This document compares Rust compression libraries for implementing compression in StreamWeave's distributed execution mode. The goal is to reduce network bandwidth by compressing serialized data before transmission.

## Requirements

- **Gzip support**: Required for `CompressionAlgorithm::Gzip`
- **Zstd support**: Required for `CompressionAlgorithm::Zstd`
- **Async support**: Preferable for non-blocking compression in async contexts
- **Performance**: High throughput and low latency
- **Compression levels**: Configurable compression levels (1-9 for gzip, 1-22 for zstd)
- **Error handling**: Robust error handling for corrupted data
- **Cross-platform**: Works on Linux, macOS, Windows

## Library Comparison

### 1. flate2

**Library**: `flate2`
**Repository**: https://github.com/rust-lang/flate2-rs
**Version**: Latest stable (1.0.x)
**License**: MIT / Apache-2.0

#### Pros
- ✅ **Mature and well-maintained**: Official Rust-lang project, widely used
- ✅ **Gzip support**: Full gzip compression/decompression support
- ✅ **Multiple backends**: Supports zlib, miniz, zlib-ng backends
- ✅ **Streaming API**: Supports streaming compression/decompression
- ✅ **Compression levels**: Configurable compression levels (0-9)
- ✅ **High performance**: Can use zlib-ng for better performance

#### Cons
- ⚠️ **Synchronous only**: No native async support (but can be used in async with `spawn_blocking`)
- ⚠️ **No Zstd**: Only supports DEFLATE-based formats (gzip, zlib, deflate)

#### Use Case
Best for gzip compression when async isn't critical or when using `spawn_blocking`.

#### Example Usage
```rust
use flate2::Compression;
use flate2::write::GzEncoder;
use std::io::prelude::*;

// Compress
let mut encoder = GzEncoder::new(Vec::new(), Compression::new(6));
encoder.write_all(data)?;
let compressed = encoder.finish()?;

// Decompress
use flate2::read::GzDecoder;
let mut decoder = GzDecoder::new(&compressed[..]);
let mut decompressed = Vec::new();
decoder.read_to_end(&mut decompressed)?;
```

### 2. async-compression

**Library**: `async-compression`
**Repository**: https://github.com/Nemo157/async-compression
**Version**: Latest (0.4.x)
**License**: MIT / Apache-2.0

#### Pros
- ✅ **Native async**: Built for async/await, non-blocking compression
- ✅ **Gzip support**: Full gzip compression/decompression
- ✅ **Streaming**: Works with async streams
- ✅ **Tokio support**: Integrates well with tokio
- ✅ **Multiple formats**: Supports gzip, deflate, zlib, brotli, etc.

#### Cons
- ⚠️ **No Zstd**: Doesn't support zstd compression
- ⚠️ **Smaller ecosystem**: Less widely used than flate2

#### Use Case
Best for async gzip compression when you need non-blocking operations.

#### Example Usage
```rust
use async_compression::tokio::write::GzipEncoder;
use tokio::io::AsyncWriteExt;

let mut encoder = GzipEncoder::new(Vec::new());
encoder.write_all(data).await?;
encoder.shutdown().await?;
let compressed = encoder.into_inner();
```

### 3. zstd

**Library**: `zstd` (zstd-rs)
**Repository**: https://github.com/gyscos/zstd-rs
**Version**: Latest (0.13.x)
**License**: MIT / Apache-2.0

#### Pros
- ✅ **High performance**: Very fast compression and decompression
- ✅ **Good compression ratio**: Better compression than gzip at similar speeds
- ✅ **Configurable levels**: Compression levels 1-22 (default 3)
- ✅ **Streaming support**: Supports streaming compression/decompression
- ✅ **Dictionary support**: Can use dictionaries for better compression
- ✅ **Cross-platform**: Works on all major platforms

#### Cons
- ⚠️ **Synchronous only**: No native async support (but can use `spawn_blocking`)
- ⚠️ **No Gzip**: Only supports zstd format

#### Use Case
Best for zstd compression when high performance is needed.

#### Example Usage
```rust
use zstd::encode_all;
use zstd::decode_all;

// Compress
let compressed = encode_all(data, 3)?; // level 3

// Decompress
let decompressed = decode_all(&compressed[..])?;
```

### 4. zstd (Facebook's C library bindings)

**Library**: `zstd-sys` / `zstd-safe`
**Repository**: https://github.com/facebook/zstd
**Note**: This is the C library, Rust bindings exist but are lower-level

#### Assessment
Not recommended for direct use - prefer `zstd` crate (zstd-rs) which provides a better Rust API.

## Recommendation: **Use Both flate2 and zstd**

For StreamWeave's use case, we should use:

1. **flate2** for `CompressionAlgorithm::Gzip`
2. **zstd** (zstd-rs) for `CompressionAlgorithm::Zstd`

### Rationale

- **flate2** is the standard for gzip in Rust, mature and well-tested
- **zstd** (zstd-rs) provides excellent zstd support with good Rust API
- Both can be used in async contexts with `spawn_blocking` for CPU-intensive work
- This gives us both compression algorithms we need
- Both libraries are actively maintained and widely used

### Alternative: async-compression for Gzip

If we want native async support for gzip, we could use `async-compression` for gzip and `zstd` for zstd. However, since compression is CPU-intensive, using `spawn_blocking` with synchronous libraries is often acceptable and may even be more efficient.

## Implementation Strategy

### Compression Trait

Create a `Compression` trait that abstracts over different compression algorithms:

```rust
pub trait Compression: Send + Sync {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;
    fn compression_level(&self) -> u32;
}
```

### Implementation

1. **GzipCompression**: Wraps `flate2::write::GzEncoder` and `flate2::read::GzDecoder`
2. **ZstdCompression**: Wraps `zstd::encode_all` and `zstd::decode_all`

### Async Integration

Since compression is CPU-intensive, use `tokio::task::spawn_blocking` to run compression/decompression in a thread pool:

```rust
// Compress
let compressed = tokio::task::spawn_blocking(move || {
    compression.compress(&data)
}).await??;

// Decompress
let decompressed = tokio::task::spawn_blocking(move || {
    compression.decompress(&compressed)
}).await??;
```

## Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
flate2 = { version = "1.0", features = ["zlib"] }
zstd = "0.13"
```

## Performance Considerations

1. **Compression Level Trade-offs**:
   - Lower levels (1-3): Faster compression, larger output
   - Higher levels (6-9 for gzip, 10-22 for zstd): Slower compression, smaller output
   - Default: gzip level 6, zstd level 3

2. **CPU vs Bandwidth**:
   - Compression reduces bandwidth but increases CPU usage
   - For high-bandwidth scenarios, compression is beneficial
   - For low-latency scenarios, consider disabling compression

3. **Async vs Sync**:
   - Using `spawn_blocking` is acceptable for CPU-intensive compression
   - Native async compression may not provide significant benefits
   - Consider benchmarking both approaches

## Next Steps

1. Add `flate2` and `zstd` dependencies to `Cargo.toml`
2. Create `Compression` trait
3. Implement `GzipCompression` using flate2
4. Implement `ZstdCompression` using zstd
5. Integrate into distributed execution pipeline
6. Add compression level configuration
7. Add error handling and tests

## References

- flate2: https://github.com/rust-lang/flate2-rs
- zstd-rs: https://github.com/gyscos/zstd-rs
- async-compression: https://github.com/Nemo157/async-compression
- Facebook zstd: https://github.com/facebook/zstd

