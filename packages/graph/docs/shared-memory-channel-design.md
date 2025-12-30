# Shared Memory Channel Architecture Design

## Overview

This document designs the architecture for shared memory channels in StreamWeave, enabling ultra-high performance data sharing between nodes using OS-native shared memory segments.

## Design Goals

1. **Zero-copy data transfer**: Data stays in shared memory, only metadata/indices are passed
2. **Lock-free where possible**: Use atomic operations for synchronization
3. **Backward compatible**: Integrate seamlessly with existing `ChannelItem` enum
4. **Type-safe extraction**: Maintain type safety at node level
5. **Resource management**: Proper cleanup of shared memory segments

## Architecture Components

### 1. Extended ChannelItem Enum

Add a new variant to `ChannelItem` for shared memory references:

```rust
pub enum ChannelItem {
    Bytes(Bytes),                    // Distributed mode
    Arc(Arc<dyn Any + Send + Sync>), // In-process zero-copy
    SharedMemory(SharedMemoryRef),   // Shared memory mode (NEW)
}
```

### 2. SharedMemoryRef

A lightweight reference to data in shared memory:

```rust
pub struct SharedMemoryRef {
    /// Unique identifier for the shared memory segment
    segment_id: String,
    /// Offset within the segment where data starts
    offset: usize,
    /// Size of the data in bytes
    size: usize,
    /// Type identifier for safe downcasting
    type_id: TypeId,
}
```

### 3. SharedMemoryChannel

Wrapper around shared memory segment with ring buffer:

```rust
pub struct SharedMemoryChannel {
    /// The shared memory segment
    shmem: Shmem,
    /// Ring buffer metadata (stored at start of shared memory)
    metadata: &'static SharedMemoryMetadata,
    /// Data buffer (after metadata)
    data_buffer: &'static [u8],
}
```

### 4. SharedMemoryMetadata Layout

Ring buffer metadata stored at the beginning of shared memory:

```rust
#[repr(C, align(64))] // Cache line alignment
struct SharedMemoryMetadata {
    /// Producer write position (atomic)
    write_pos: AtomicUsize,
    /// Consumer read position (atomic)
    read_pos: AtomicUsize,
    /// Buffer capacity in bytes
    capacity: usize,
    /// Number of items currently in buffer
    item_count: AtomicUsize,
    /// Padding to cache line boundary
    _padding: [u8; 64 - 4 * std::mem::size_of::<usize>()],
}
```

## Ring Buffer Design

### Layout

```
+------------------+------------------+------------------+
| Metadata (64B)   | Data Buffer      | Reserved         |
| - write_pos      | [item1][item2]   | (for alignment)  |
| - read_pos       | ...               |                  |
| - capacity       |                   |                  |
| - item_count     |                   |                  |
+------------------+------------------+------------------+
```

### Item Format in Buffer

Each item in the buffer has a header followed by data:

```
+--------+--------+--------+
| Size   | TypeId | Data   |
| (8B)   | (8B)   | (N B)  |
+--------+--------+--------+
```

### Synchronization

1. **Producer (Writer)**:
   - Atomically increment `write_pos`
   - Write item header + data
   - Atomically increment `item_count`
   - Use memory barriers to ensure visibility

2. **Consumer (Reader)**:
   - Check `item_count > 0`
   - Read item header to get size
   - Read data
   - Atomically increment `read_pos`
   - Atomically decrement `item_count`

### Lock-Free Algorithm

```rust
// Producer
fn send(&self, data: &[u8]) -> Result<(), Error> {
    let item_size = data.len() + 16; // header size
    let current_write = self.metadata.write_pos.load(Ordering::Acquire);
    let next_write = (current_write + item_size) % self.metadata.capacity;
    
    // Check if buffer has space
    if next_write == self.metadata.read_pos.load(Ordering::Acquire) {
        return Err(Error::BufferFull);
    }
    
    // Write item
    let offset = current_write + std::mem::size_of::<SharedMemoryMetadata>();
    self.data_buffer[offset..offset+8].copy_from_slice(&data.len().to_le_bytes());
    self.data_buffer[offset+8..offset+16].copy_from_slice(&type_id.to_le_bytes());
    self.data_buffer[offset+16..offset+16+data.len()].copy_from_slice(data);
    
    // Update write position
    self.metadata.write_pos.store(next_write, Ordering::Release);
    self.metadata.item_count.fetch_add(1, Ordering::Release);
    
    Ok(())
}
```

## Integration Points

### 1. GraphExecutor

Update `create_channels_with_buffer_size` to support shared memory:

```rust
pub fn create_channels_with_buffer_size(
    &mut self,
    buffer_size: usize,
) -> Result<(), ExecutionError> {
    let use_shared_memory = matches!(
        self.execution_mode,
        ExecutionMode::InProcess { use_shared_memory: true, .. }
    );
    
    for conn in self.graph.get_connections() {
        if use_shared_memory {
            // Create shared memory channel
            let channel = SharedMemoryChannel::new(
                format!("{}_{}_{}", conn.source.0, conn.source.1, conn.target.1),
                buffer_size,
            )?;
            // Store both sender and receiver (same underlying segment)
            self.shared_memory_channels.insert(
                (conn.source.0.clone(), conn.source.1),
                channel.clone(),
            );
            self.shared_memory_channels.insert(
                (conn.target.0.clone(), conn.target.1),
                channel,
            );
        } else {
            // Use regular channels
            let (sender, receiver): (TypeErasedSender, TypeErasedReceiver) =
                mpsc::channel(buffer_size);
            // ... existing code
        }
    }
    Ok(())
}
```

### 2. Node Execution

Nodes check for shared memory mode and extract data accordingly:

```rust
// In ProducerNode::spawn_execution_task
if use_shared_memory {
    // Get shared memory channel
    let channel = self.get_shared_memory_channel(...)?;
    
    // Serialize item to bytes
    let serialized = serialize(&item)?;
    
    // Write to shared memory
    let ref = channel.send(serialized)?;
    
    // Send reference (lightweight)
    output_channels.send(ChannelItem::SharedMemory(ref)).await?;
} else {
    // Existing Arc<T> or Bytes logic
}
```

### 3. Type Extraction

Nodes extract typed data from shared memory:

```rust
// In TransformerNode or ConsumerNode
match channel_item {
    ChannelItem::SharedMemory(ref) => {
        // Open shared memory segment
        let channel = SharedMemoryChannel::open(ref.segment_id)?;
        
        // Read data from shared memory
        let bytes = channel.receive(ref.offset, ref.size)?;
        
        // Deserialize to typed value
        let item: T::Input = deserialize(bytes)?;
        
        // Process item...
    }
    // ... other variants
}
```

## Performance Optimizations

### 1. Batch Operations

Process multiple items in one shared memory access:

```rust
fn receive_batch(&self, count: usize) -> Result<Vec<Bytes>, Error> {
    // Read multiple items in one operation
    // Reduces atomic operations overhead
}
```

### 2. Memory Alignment

- Align metadata to cache line (64 bytes)
- Align data items to 8-byte boundaries
- Reduces false sharing

### 3. Pre-allocation

- Pre-allocate shared memory segments at graph creation
- Avoid runtime allocation overhead
- Reuse segments across multiple executions

### 4. Zero-Copy Deserialization

- For types that can be safely transmuted, use zero-copy deserialization
- Use `bytemuck` or similar for POD types
- Avoid unnecessary copies

## Resource Management

### 1. Segment Naming

Use unique names for each connection:
```
format!("streamweave_{}_{}_{}_{}", 
    graph_id, source_node, source_port, target_port)
```

### 2. Cleanup Strategy

- **On Drop**: Automatically cleanup when `SharedMemoryChannel` is dropped
- **On Shutdown**: Explicit cleanup in `GraphExecutor::shutdown()`
- **On Error**: Cleanup on execution errors

### 3. Lifecycle

```rust
impl Drop for SharedMemoryChannel {
    fn drop(&mut self) {
        // Mark segment for deletion
        // OS will cleanup when last reference is dropped
    }
}
```

## Error Handling

### Error Types

```rust
pub enum SharedMemoryError {
    /// Buffer is full
    BufferFull,
    /// Buffer is empty
    BufferEmpty,
    /// Failed to create shared memory segment
    CreationFailed(String),
    /// Failed to open existing segment
    OpenFailed(String),
    /// Type mismatch
    TypeMismatch { expected: TypeId, got: TypeId },
    /// Serialization error
    SerializationError(String),
}
```

## Testing Strategy

1. **Unit Tests**:
   - Test ring buffer operations
   - Test synchronization correctness
   - Test type safety

2. **Integration Tests**:
   - Test full graph execution with shared memory
   - Test cleanup on shutdown
   - Test error recovery

3. **Performance Tests**:
   - Benchmark vs Arc<T> channels
   - Benchmark vs Bytes channels
   - Measure throughput and latency

## Migration Path

1. **Phase 1**: Implement `SharedMemoryChannel` as optional feature
2. **Phase 2**: Add `SharedMemory` variant to `ChannelItem`
3. **Phase 3**: Update `GraphExecutor` to support shared memory mode
4. **Phase 4**: Update nodes to handle shared memory items
5. **Phase 5**: Add tests and benchmarks
6. **Phase 6**: Enable by default when `use_shared_memory=true`

## Dependencies

Add to `Cargo.toml`:
```toml
[dependencies]
shared_memory = "0.12"
bytemuck = { version = "1.14", features = ["derive"] } # For zero-copy POD types
```

## Future Enhancements

1. **Multiple Segments**: Support multiple shared memory segments per connection
2. **Dynamic Resizing**: Resize shared memory segments dynamically
3. **NUMA Awareness**: Place shared memory on appropriate NUMA node
4. **Memory Pooling**: Pool shared memory segments for reuse
5. **Zero-Copy Types**: Direct transmute for POD types without serialization

