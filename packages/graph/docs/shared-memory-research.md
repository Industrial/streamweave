# Rust Shared Memory Libraries Research

## Overview

This document compares Rust libraries for implementing shared memory support in StreamWeave's in-process execution mode. The goal is to enable ultra-high performance data sharing between nodes using shared memory segments.

## Requirements

- **Cross-platform support**: Linux, macOS, Windows
- **High performance**: Minimal overhead for data passing
- **Type safety**: Safe Rust API where possible
- **Synchronization**: Support for concurrent access patterns
- **Memory management**: Proper cleanup and resource management
- **API ergonomics**: Easy to integrate with existing channel architecture

## Library Comparison

### 1. memmap2

**Library**: `memmap2` (maintained fork of `memmap`)
**Repository**: https://github.com/RazrFalcon/memmap2
**Version**: Latest stable (0.9.x)
**License**: Apache-2.0 / MIT

#### Pros
- ✅ **Mature and well-maintained**: Active development, widely used
- ✅ **Cross-platform**: Works on Linux, macOS, Windows
- ✅ **File-backed or anonymous**: Supports both file-backed and anonymous memory maps
- ✅ **High performance**: Direct memory access, zero-copy semantics
- ✅ **Flexible API**: `MmapOptions` for fine-grained control
- ✅ **Copy-on-write support**: `map_copy()` for isolated writes
- ✅ **Large codebase coverage**: 806 code snippets in Context7

#### Cons
- ⚠️ **Requires file handle**: For cross-platform shared memory, typically uses named files
- ⚠️ **Manual synchronization**: No built-in synchronization primitives
- ⚠️ **Unsafe API**: Requires `unsafe` blocks for mapping operations

#### Use Case
Best for file-backed shared memory or when you need fine-grained control over memory mapping behavior.

#### Example Usage
```rust
use memmap2::MmapOptions;
use std::fs::OpenOptions;

// Create shared memory file
let file = OpenOptions::new()
    .read(true)
    .write(true)
    .create(true)
    .truncate(true)
    .open("/tmp/shared_mem")?;
file.set_len(1024 * 1024)?; // 1MB

// Map to memory
let mut mmap = unsafe {
    MmapOptions::new().map_mut(&file)?
};

// Access as mutable slice
mmap[0..10].copy_from_slice(b"Hello");
```

### 2. shared_memory

**Library**: `shared_memory`
**Repository**: https://github.com/elast0ny/shared_memory-rs
**Version**: Latest (0.12.x)
**License**: MIT

#### Pros
- ✅ **Purpose-built for IPC**: Designed specifically for inter-process shared memory
- ✅ **Cross-platform**: Linux (POSIX shm), macOS (POSIX shm), Windows (file mapping)
- ✅ **Named segments**: Easy-to-use named shared memory segments
- ✅ **Type-safe wrappers**: Provides `Shmem` wrapper with safe API
- ✅ **Automatic cleanup**: Can auto-cleanup on drop (optional)
- ✅ **OS-native**: Uses platform-specific shared memory APIs

#### Cons
- ⚠️ **Less flexible**: More opinionated API compared to memmap2
- ⚠️ **Smaller ecosystem**: Less widely used than memmap2
- ⚠️ **Manual synchronization**: Still requires external synchronization

#### Use Case
Best for true inter-process shared memory with named segments. Ideal when you need OS-native shared memory APIs.

#### Example Usage
```rust
use shared_memory::{Shmem, ShmemConf};

// Create shared memory segment
let mut shmem = ShmemConf::new()
    .size(1024 * 1024)
    .create()?;

// Access as slice
let slice: &mut [u8] = shmem.as_mut();
slice[0..10].copy_from_slice(b"Hello");
```

### 3. shmem (OpenSHMEM-style)

**Library**: `shmem` (if exists)
**Status**: Not commonly found in Rust ecosystem
**Note**: OpenSHMEM is primarily a C/Fortran standard, Rust bindings are rare

#### Assessment
Not recommended - limited Rust ecosystem support.

## Recommendation: **shared_memory**

For StreamWeave's use case, **`shared_memory`** is the best choice because:

1. **Purpose-built for IPC**: Designed specifically for shared memory between processes/threads
2. **Named segments**: Easy to create and share named memory segments
3. **Type-safe API**: `Shmem` wrapper provides safe access patterns
4. **Cross-platform**: Uses native OS shared memory APIs
5. **Clean integration**: Can be wrapped in a channel-like interface

### Alternative: memmap2

If we need more control or file-backed memory maps, `memmap2` is an excellent alternative. It's more flexible but requires more setup code.

## Implementation Strategy

### Shared Memory Channel Design

1. **Create `SharedMemoryChannel` struct**:
   - Wraps `Shmem` from `shared_memory` crate
   - Provides `send()` and `receive()` methods
   - Handles serialization/deserialization if needed

2. **Ring Buffer Pattern**:
   - Use shared memory as a ring buffer
   - Atomic indices for producer/consumer positions
   - Lock-free or minimal-lock synchronization

3. **Integration Points**:
   - Extend `ChannelItem` enum to include `SharedMemory` variant
   - Update `GraphExecutor` to create shared memory channels when `use_shared_memory` is true
   - Nodes extract data from shared memory segments

### Performance Considerations

- **Zero-copy**: Data stays in shared memory, only pointers/indices are passed
- **Lock-free**: Use atomic operations for synchronization where possible
- **Batch operations**: Process multiple items in one shared memory access
- **Memory alignment**: Ensure proper alignment for performance

### Synchronization Patterns

1. **Atomic Counters**: For producer/consumer indices
2. **Memory Barriers**: Ensure visibility of writes
3. **Condition Variables**: For blocking when buffer is full/empty (if needed)
4. **Spinlocks**: For short critical sections

## Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
shared_memory = "0.12"
# Or alternatively:
# memmap2 = "0.9"
```

## Next Steps

1. Implement `SharedMemoryChannel` using `shared_memory` crate
2. Design ring buffer layout in shared memory
3. Integrate with existing `ChannelItem` enum
4. Update `GraphExecutor` to support shared memory mode
5. Add tests and benchmarks

## References

- memmap2: https://github.com/RazrFalcon/memmap2
- shared_memory: https://github.com/elast0ny/shared_memory-rs
- Context7 memmap2 docs: /websites/rs_memmap2

