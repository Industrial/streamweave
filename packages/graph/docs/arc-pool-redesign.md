# ArcPool Redesign

## Overview

This document outlines the redesign of `ArcPool` to implement actual pooling logic. The key change is to pool underlying values (`T`) instead of `Arc<T>` instances.

## Current Implementation Issues

1. **No Actual Pooling**: `get_or_create` always creates new `Arc` instances
2. **Can't Reuse Arc**: `Arc` instances can't be easily reused when they have multiple references
3. **Placeholder Logic**: `return_arc` doesn't actually store values for reuse

## Redesigned Architecture

### Core Change: Value-Based Pooling

**Old Structure**:
```rust
pub struct ArcPool<T> {
    pool: Arc<Mutex<Vec<Arc<T>>>>,  // Pool of Arc instances (can't reuse)
    max_size: usize,
}
```

**New Structure**:
```rust
pub struct ArcPool<T> {
    pool: Arc<Mutex<Vec<T>>>,  // Pool of values (can reuse)
    max_size: usize,
    // Statistics (to be added in next subtask)
}
```

### Method Changes

#### `get_or_create(value: T) -> Arc<T>`

**Old Behavior**: Always creates new `Arc::new(value)`

**New Behavior**:
1. Try to pop a value from the pool
2. If pool has a value, wrap it in `Arc` and return
3. If pool is empty, wrap the provided `value` in `Arc` and return

**Benefits**:
- Reuses values from the pool when available
- Reduces allocation overhead
- Still creates new `Arc` wrapper (necessary for sharing)

#### `return_arc(arc: Arc<T>)`

**Old Behavior**: Tries to unwrap but doesn't store the value

**New Behavior**:
1. Try to unwrap the `Arc` using `Arc::try_unwrap()`
2. If successful (only one reference):
   - Check if pool has space
   - If yes, push the value to the pool
   - If no, drop the value
3. If failed (multiple references):
   - Drop the `Arc` (value will be dropped when last reference is dropped)

**Benefits**:
- Actually stores values for reuse
- Only pools values from single-reference `Arc`s (expected behavior)
- Handles full pool gracefully

### Implementation Details

**Pool Storage**:
- Store `Vec<T>` instead of `Vec<Arc<T>>`
- Use `Mutex` for thread-safe access
- Limit pool size to `max_size`

**Value Lifecycle**:
1. Value created or retrieved from pool
2. Wrapped in `Arc` for sharing
3. When `Arc` is returned and has single reference, value goes back to pool
4. When pool is full, values are dropped

**Thread Safety**:
- `Mutex` protects the pool vector
- `Arc` provides thread-safe sharing of values
- All operations are thread-safe

## Migration Strategy

1. Change `pool` field type from `Vec<Arc<T>>` to `Vec<T>`
2. Update `get_or_create` to pop from pool and wrap in `Arc`
3. Update `return_arc` to unwrap and push to pool
4. Update `len()` and `is_empty()` to work with new structure
5. Add statistics tracking (next subtask)

## Benefits

1. **Actual Pooling**: Values are reused, reducing allocations
2. **Works with Fan-Out**: Can pool values before wrapping in `Arc`
3. **Thread-Safe**: Uses `Mutex` for pool access
4. **Memory Bounded**: Pool size is limited to `max_size`

## Limitations

1. **Single Reference Requirement**: Only values from single-reference `Arc`s can be pooled
2. **Arc Allocation**: Still allocates new `Arc` wrapper (but reuses values)
3. **Pool Size**: Must be tuned based on usage patterns

## Next Steps

1. Implement value-based pooling structure
2. Update `get_or_create` and `return_arc` methods
3. Add statistics tracking
4. Integrate into fan-out operations
5. Add tests and benchmarks

