# Type-Erased Zero-Copy Channel Strategy Design

## Problem Statement

The current graph execution architecture uses `Bytes` channels for all inter-node communication, even in in-process mode. This requires serialization/deserialization overhead even when all nodes are in the same process. We need a type-erased channel strategy that:

1. Works with `Box<dyn NodeTrait>` (type-erased nodes)
2. Supports both `Bytes` (distributed) and `Arc<T>` (in-process zero-copy) channels
3. Maintains type safety at the node level
4. Allows runtime selection based on `ExecutionMode`

## Current Architecture

### Channel Storage
- `GraphExecutor` stores channels as:
  - `channel_senders: HashMap<(String, usize), mpsc::Sender<Bytes>>`
  - `channel_receivers: HashMap<(String, usize), mpsc::Receiver<Bytes>>`
- `NodeTrait::spawn_execution_task` signature:
  ```rust
  fn spawn_execution_task(
    &self,
    input_channels: HashMap<usize, mpsc::Receiver<Bytes>>,
    output_channels: HashMap<usize, mpsc::Sender<Bytes>>,
    ...
  )
  ```

### Problem
- All channels are `Bytes` type, even in `ExecutionMode::InProcess`
- Nodes must serialize/deserialize even when in same process
- No way to pass `Arc<T>` directly between nodes

## Solution: Enum-Based Type-Erased Channels

### Design Approach

Use an enum to represent either `Bytes` (distributed) or type-erased `Arc<dyn Any + Send + Sync>` (in-process) channels. The enum provides type erasure at the executor level while allowing type-safe extraction at the node level.

### Core Types

```rust
/// Type-erased channel item that can hold either Bytes (distributed) or Arc<T> (in-process)
#[derive(Clone)]
pub enum ChannelItem {
    /// Serialized bytes for distributed execution
    Bytes(bytes::Bytes),
    /// Type-erased Arc for zero-copy in-process execution
    Arc(std::sync::Arc<dyn std::any::Any + Send + Sync>),
}

/// Type-erased channel sender
pub type TypeErasedSender = mpsc::Sender<ChannelItem>;

/// Type-erased channel receiver
pub type TypeErasedReceiver = mpsc::Receiver<ChannelItem>;
```

### Channel Creation Strategy

1. **GraphExecutor Level**: Store type-erased channels
   ```rust
   channel_senders: HashMap<(String, usize), TypeErasedSender>,
   channel_receivers: HashMap<(String, usize), TypeErasedReceiver>,
   ```

2. **Node Level**: Extract typed channels based on ExecutionMode
   - For `ExecutionMode::InProcess`: Extract `Arc<T>` from `ChannelItem::Arc`
   - For `ExecutionMode::Distributed`: Extract `Bytes` from `ChannelItem::Bytes`

### Type-Safe Extraction

Nodes need to extract the correct type from `ChannelItem`. We provide helper methods:

```rust
impl ChannelItem {
    /// Extract Bytes if this is a Bytes variant
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            ChannelItem::Bytes(b) => Some(b),
            _ => None,
        }
    }
    
    /// Extract Arc<T> if this is an Arc variant, with type checking
    pub fn downcast_arc<T: 'static>(self) -> Result<Arc<T>, Self> {
        match self {
            ChannelItem::Arc(arc) => {
                Arc::downcast(arc).map_err(|e| ChannelItem::Arc(e))
            }
            other => Err(other),
        }
    }
}
```

### Node Execution Pattern

Each node type (Producer, Transformer, Consumer) will:

1. Check `ExecutionMode` to determine channel type
2. Create typed wrappers around `TypeErasedReceiver`/`TypeErasedSender`
3. Use typed channels for zero-copy in-process or serialized distributed

### Example: ProducerNode

```rust
impl<P, Outputs> NodeTrait for ProducerNode<P, Outputs> {
    fn spawn_execution_task(
        &self,
        input_channels: HashMap<usize, TypeErasedReceiver>,
        output_channels: HashMap<usize, TypeErasedSender>,
        pause_signal: Arc<RwLock<bool>>,
        execution_mode: ExecutionMode,
    ) -> Option<JoinHandle<Result<(), ExecutionError>>> {
        match execution_mode {
            ExecutionMode::InProcess { .. } => {
                // Create Arc<P::Output> channels
                let typed_senders: HashMap<usize, mpsc::Sender<Arc<P::Output>>> = 
                    output_channels.into_iter()
                        .map(|(port, sender)| {
                            // Wrap TypeErasedSender to send Arc<P::Output>
                            (port, wrap_arc_sender::<P::Output>(sender))
                        })
                        .collect();
                
                // Spawn task with typed channels
                spawn_producer_task_arc(self, typed_senders, pause_signal)
            }
            ExecutionMode::Distributed { .. } => {
                // Create Bytes channels
                let typed_senders: HashMap<usize, mpsc::Sender<Bytes>> = 
                    output_channels.into_iter()
                        .map(|(port, sender)| {
                            // Wrap TypeErasedSender to send Bytes
                            (port, wrap_bytes_sender(sender))
                        })
                        .collect();
                
                // Spawn task with Bytes channels
                spawn_producer_task_bytes(self, typed_senders, pause_signal)
            }
            _ => unreachable!(),
        }
    }
}
```

### Channel Wrapper Pattern

Create wrapper types that convert between `ChannelItem` and typed channels:

```rust
/// Wrapper that converts Arc<T> to ChannelItem::Arc
struct ArcChannelSender<T> {
    inner: mpsc::Sender<ChannelItem>,
    _phantom: PhantomData<T>,
}

impl<T: 'static + Send + Sync> ArcChannelSender<T> {
    async fn send(&self, item: Arc<T>) -> Result<(), mpsc::error::SendError<ChannelItem>> {
        self.inner.send(ChannelItem::Arc(item)).await
    }
}

/// Wrapper that converts Bytes to ChannelItem::Bytes
struct BytesChannelSender {
    inner: mpsc::Sender<ChannelItem>,
}

impl BytesChannelSender {
    async fn send(&self, item: Bytes) -> Result<(), mpsc::error::SendError<ChannelItem>> {
        self.inner.send(ChannelItem::Bytes(item)).await
    }
}
```

### Fan-Out Strategy

For fan-out (multiple output channels), use `Arc::clone` which is zero-copy:

```rust
// In in-process mode
let item_arc = Arc::new(item);
for sender in output_channels.values() {
    sender.send(Arc::clone(&item_arc)).await?; // Zero-copy clone
}
```

### Fan-In Strategy

For fan-in (multiple input channels), merge streams:

```rust
// In in-process mode
let streams: Vec<_> = input_channels.into_iter()
    .map(|(_, receiver)| {
        // Convert TypeErasedReceiver to stream of Arc<T>
        receiver_to_arc_stream::<T::Input>(receiver)
    })
    .collect();
let merged = futures::stream::select_all(streams);
```

## Implementation Steps

1. **Create ChannelItem enum** in `packages/graph/src/channels.rs`
2. **Update GraphExecutor** to use `TypeErasedSender`/`TypeErasedReceiver`
3. **Create channel wrapper types** for type-safe conversion
4. **Update NodeTrait** signature to accept type-erased channels
5. **Update each node type** to extract typed channels based on ExecutionMode
6. **Add helper functions** for channel conversion

## Benefits

1. **Type Erasure**: Works with `Box<dyn NodeTrait>` at executor level
2. **Type Safety**: Maintains type safety at node level
3. **Zero-Copy**: Eliminates serialization in in-process mode
4. **Backward Compatible**: Can still use Bytes for distributed mode
5. **Runtime Selection**: Choose channel type based on ExecutionMode

## Trade-offs

1. **Complexity**: Adds enum wrapper and conversion logic
2. **Type Checking**: Runtime type checking for Arc downcast (but only in in-process mode)
3. **Memory**: Arc overhead vs serialization overhead (Arc is typically cheaper)

## Alternative Approaches Considered

### Option 1: Generic Executor
- **Pros**: Full type safety, no runtime checks
- **Cons**: Cannot work with `Box<dyn NodeTrait>`, requires monomorphization

### Option 2: Trait Objects for Channels
- **Pros**: More flexible
- **Cons**: More complex, harder to work with, dynamic dispatch overhead

### Option 3: Separate Executor Types
- **Pros**: Clear separation, type-safe
- **Cons**: Code duplication, harder to maintain

## Conclusion

The enum-based approach provides the best balance of:
- Type erasure at executor level (works with trait objects)
- Type safety at node level (compile-time checks where possible)
- Zero-copy in-process execution
- Maintainability and clarity

