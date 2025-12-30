# Batching Architecture Design

## Overview

This document designs the architecture for batching items in StreamWeave's distributed execution mode. Batching groups multiple items together before serialization and transmission, reducing network overhead and improving throughput.

## Requirements

- **Buffer items**: Collect items until batch conditions are met
- **Batch size threshold**: Flush when `batch_size` items are collected
- **Timeout threshold**: Flush when `batch_timeout_ms` expires
- **Serialization**: Serialize entire batch as a single unit
- **Deserialization**: Deserialize batches and split into individual items
- **No data loss**: Ensure all items are sent, even if batch is incomplete at shutdown
- **Async-friendly**: Work seamlessly with async/await patterns

## Architecture Components

### 1. BatchBuffer

A buffer that collects items and flushes when conditions are met:

```rust
pub struct BatchBuffer<T> {
    items: Vec<T>,
    batch_size: usize,
    timeout: Duration,
    last_flush: Instant,
}
```

### 2. BatchingChannel

A wrapper around `TypeErasedSender` that adds batching logic:

```rust
pub struct BatchingChannel {
    inner: TypeErasedSender,
    buffer: Arc<Mutex<BatchBuffer<Bytes>>>,
    config: BatchConfig,
    flush_task: Option<JoinHandle<()>>,
}
```

### 3. Batch Format

Batches are serialized as a sequence of items. We need a format that:
- Allows deserialization of individual items
- Handles variable-length items
- Supports efficient splitting

**Option 1: Length-prefixed items**
```
[item1_len][item1_bytes][item2_len][item2_bytes]...
```

**Option 2: Array/Sequence format**
Serialize as `Vec<T>` where T is the item type.

**Option 3: Delimited format**
Use a delimiter between items (requires escaping).

**Recommendation**: Use Option 1 (length-prefixed) for flexibility and efficiency.

## Design Decisions

### 1. Where to Apply Batching

**Option A: Channel Wrapper**
- Wrap `TypeErasedSender` with `BatchingChannel`
- Transparent to nodes
- Pros: Clean separation, easy to enable/disable
- Cons: Need to handle both batched and non-batched channels

**Option B: Node-Level Batching**
- Nodes check for batching config and buffer internally
- Pros: More control, can optimize per node
- Cons: Code duplication, harder to maintain

**Recommendation**: Option A (Channel Wrapper) - cleaner architecture

### 2. Batch Serialization Format

**Format**: Length-prefixed items
- Each item: `[u64: length][bytes: data]`
- Batch: `[u64: item_count][item1][item2]...`

This allows:
- Efficient deserialization
- Streaming deserialization (don't need entire batch in memory)
- Easy to split into individual items

### 3. Flush Strategy

**Immediate flush triggers**:
1. Batch size reached (`batch_size` items)
2. Timeout expired (`batch_timeout_ms`)
3. Shutdown signal

**Flush logic**:
```rust
async fn flush_if_needed(&self) -> Result<(), Error> {
    let mut buffer = self.buffer.lock().await;
    
    // Check size threshold
    if buffer.items.len() >= self.config.batch_size {
        return self.flush_buffer(&mut buffer).await;
    }
    
    // Check timeout
    if buffer.last_flush.elapsed() >= Duration::from_millis(self.config.batch_timeout_ms) {
        return self.flush_buffer(&mut buffer).await;
    }
    
    Ok(())
}
```

### 4. Background Flush Task

A background task periodically checks for timeout:

```rust
async fn start_flush_task(&self) {
    let interval = Duration::from_millis(self.config.batch_timeout_ms);
    loop {
        tokio::time::sleep(interval).await;
        if let Err(e) = self.flush_if_needed().await {
            // Handle error
            break;
        }
    }
}
```

## Integration Points

### 1. GraphExecutor

Update `create_channels_with_buffer_size` to wrap channels with batching:

```rust
if let Some(batch_config) = batching {
    let batching_channel = BatchingChannel::new(sender, batch_config);
    // Store batching channel instead of regular sender
}
```

### 2. Node Execution

Nodes use `BatchingChannel` transparently - it implements the same `send` interface:

```rust
// In ProducerNode or TransformerNode
batching_channel.send(ChannelItem::Bytes(item)).await?;
// Internally buffers and flushes when needed
```

### 3. StreamWrapper

Update `StreamWrapper` to handle batched data:

```rust
match channel_item {
    ChannelItem::Bytes(bytes) => {
        if is_batched {
            // Deserialize batch and yield individual items
            let batch = deserialize_batch(bytes)?;
            for item in batch {
                yield item;
            }
        } else {
            // Regular single item
            yield deserialize(bytes)?;
        }
    }
}
```

## Serialization Format

### Batch Structure

```rust
struct Batch {
    item_count: u64,
    items: Vec<Bytes>,  // Length-prefixed items
}

// Serialization:
// [u64: item_count][u64: item1_len][item1_bytes][u64: item2_len][item2_bytes]...
```

### Deserialization

```rust
fn deserialize_batch(bytes: &[u8]) -> Result<Vec<Bytes>, Error> {
    let mut items = Vec::new();
    let mut offset = 0;
    
    // Read item count
    let item_count = read_u64(&bytes[offset..])?;
    offset += 8;
    
    // Read each item
    for _ in 0..item_count {
        let item_len = read_u64(&bytes[offset..])? as usize;
        offset += 8;
        let item = Bytes::copy_from_slice(&bytes[offset..offset + item_len]);
        offset += item_len;
        items.push(item);
    }
    
    Ok(items)
}
```

## Error Handling

- **Buffer full**: If buffer grows beyond reasonable size, force flush
- **Serialization error**: Return error, don't lose items
- **Send error**: Retry or return error
- **Shutdown**: Flush remaining items before exit

## Performance Considerations

1. **Memory**: Batches use more memory (buffer multiple items)
2. **Latency**: Items wait in buffer until flush (adds latency)
3. **Throughput**: Batching improves throughput by reducing per-item overhead
4. **CPU**: Serialization of batches may be more CPU-intensive

## Configuration

```rust
pub struct BatchConfig {
    pub batch_size: usize,        // Max items per batch
    pub batch_timeout_ms: u64,    // Max time to wait (ms)
}
```

**Validation**:
- `batch_size > 0`
- `batch_timeout_ms > 0` and reasonable (< 1 hour)

## Next Steps

1. Implement `BatchBuffer` struct
2. Implement `BatchingChannel` wrapper
3. Add batch serialization/deserialization
4. Integrate into `GraphExecutor`
5. Update `StreamWrapper` for batch deserialization
6. Add tests and benchmarks

