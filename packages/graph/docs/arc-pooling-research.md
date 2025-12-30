# Arc Pooling Research

## Overview

This document summarizes research on Arc pooling strategies for Rust, focusing on the limitations and best practices for pooling `Arc<T>` instances.

## Key Findings

### 1. Arc Limitations

**Problem**: `Arc<T>` instances cannot be easily reused because:
- `Arc::try_unwrap()` only succeeds when there's exactly one strong reference
- If an `Arc` has multiple references (which is common in fan-out scenarios), it cannot be unwrapped
- Once an `Arc` is cloned, the original and clones share the same underlying data
- You cannot "reset" an `Arc` to point to a different value

**Implication**: Direct pooling of `Arc<T>` instances is not practical for fan-out scenarios where multiple references exist.

### 2. Value-Based Pooling Strategy

**Solution**: Pool the underlying values (`T`) instead of `Arc<T>` instances.

**Approach**:
1. Store `Vec<T>` in the pool (not `Vec<Arc<T>>`)
2. When `get_or_create(value)` is called:
   - Try to pop a value from the pool
   - If pool is empty, use the provided value
   - Wrap the value in `Arc` and return
3. When `return_arc(arc)` is called:
   - Try to unwrap the `Arc` using `Arc::try_unwrap()`
   - If successful (only one reference), return the value to the pool
   - If failed (multiple references), drop the `Arc` (value will be dropped when last reference is dropped)

**Benefits**:
- Reduces allocation overhead for the `Arc` wrapper itself
- Reuses the underlying values when possible
- Works even when `Arc` has multiple references (we just can't pool it)

**Limitations**:
- Only values from `Arc`s with a single reference can be pooled
- In fan-out scenarios, the original `Arc` may have multiple references, so we can't pool it
- However, we can still pool values that are created fresh and not yet shared

### 3. Pooling Strategy for Fan-Out

**Fan-Out Scenario**:
```
Producer -> Arc<T> -> [Transformer1, Transformer2, Transformer3]
```

**Current Behavior**:
- Producer creates `Arc<T>` with value
- Clones `Arc` for each transformer (zero-copy)
- Each transformer eventually drops its `Arc`
- When last `Arc` is dropped, value is dropped

**Pooling Strategy**:
- Producer gets value from pool (or creates new)
- Wraps in `Arc` and clones for fan-out
- When transformers drop their `Arc` clones, nothing can be pooled (multiple refs)
- When producer's original `Arc` is dropped (if it's the last), we can try to pool the value

**Optimization**:
- For values that are created but not yet shared, we can pool them immediately
- For values that have been shared, we wait until the last reference is dropped
- Use `Arc::try_unwrap()` to check if we can extract the value

### 4. Implementation Details

**Pool Structure**:
```rust
pub struct ArcPool<T: Clone + Send + Sync + 'static> {
    pool: Arc<Mutex<Vec<T>>>,  // Pool of values, not Arc<T>
    max_size: usize,
    // Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    allocations: AtomicU64,
}
```

**get_or_create Implementation**:
```rust
pub fn get_or_create(&self, value: T) -> Arc<T> {
    let mut pool = self.pool.lock().unwrap();
    
    // Try to get a value from the pool
    if let Some(pooled_value) = pool.pop() {
        self.hits.fetch_add(1, Ordering::Relaxed);
        Arc::new(pooled_value)  // Wrap pooled value
    } else {
        self.misses.fetch_add(1, Ordering::Relaxed);
        self.allocations.fetch_add(1, Ordering::Relaxed);
        Arc::new(value)  // Use provided value
    }
}
```

**return_arc Implementation**:
```rust
pub fn return_arc(&self, arc: Arc<T>) {
    // Try to unwrap - only succeeds if this is the only reference
    if let Ok(value) = Arc::try_unwrap(arc) {
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_size {
            pool.push(value);  // Return value to pool
        }
        // If pool is full, drop the value
    }
    // If arc has multiple references, it will be dropped when last ref is dropped
}
```

### 5. Statistics and Monitoring

**Metrics to Track**:
- **Pool hits**: Number of times a value was reused from the pool
- **Pool misses**: Number of times a new value had to be allocated
- **Pool size**: Current number of values in the pool
- **Hit rate**: `hits / (hits + misses)`
- **Allocations saved**: `hits` (each hit represents one allocation avoided)

**Use Cases**:
- Monitor pool effectiveness
- Tune pool size based on hit rate
- Identify scenarios where pooling is beneficial

### 6. Best Practices

1. **Pool Size**: Start with a pool size that matches expected concurrency (e.g., number of fan-out targets)
2. **Value Types**: Pooling works best for types that are:
   - Expensive to allocate (large buffers, complex structures)
   - Frequently reused in similar patterns
   - `Clone + Send + Sync + 'static`
3. **Fan-Out Optimization**: In fan-out scenarios, pool values before wrapping in `Arc`, not after
4. **Statistics**: Always track pool effectiveness to understand if pooling is beneficial

### 7. Alternative Approaches

**Option A: Value Pooling (Recommended)**
- Pool `T` values, wrap in `Arc` when needed
- Pros: Works with fan-out, reduces allocations
- Cons: Only pools values from single-reference `Arc`s

**Option B: Weak Reference Pooling**
- Use `Weak<T>` to track pooled values
- Upgrade to `Arc` when needed
- Pros: Can track values even when `Arc` is dropped
- Cons: More complex, requires `Arc` allocation anyway

**Option C: No Pooling**
- Always allocate new `Arc` instances
- Pros: Simple, no overhead
- Cons: More allocations in hot paths

**Recommendation**: Option A (Value Pooling) - best balance of simplicity and effectiveness

## References

- Current implementation: `packages/graph/src/zero_copy.rs:154-280`
- Arc documentation: Rust std::sync::Arc
- Arc::try_unwrap: Only succeeds with single reference

