# Graph API Migration Guide

This guide explains how to migrate code from the `streamweave-graph` package to the integrated `streamweave::graph` module.

## Overview

The `streamweave-graph` package has been integrated into the core `streamweave` package as the `graph` module. This consolidation provides:
- **Unified API**: Graph functionality is now part of the core package
- **Message<T> Integration**: All data flowing through graphs is automatically wrapped in `Message<T>`
- **Simplified Dependencies**: No need for a separate graph package
- **Better Integration**: Graph API works seamlessly with core StreamWeave components

## Key Changes

### 1. Import Path Changes

**Before (streamweave-graph):**
```rust
use streamweave_graph::{GraphBuilder, GraphExecution};
use streamweave_graph::node::{ProducerNode, TransformerNode, ConsumerNode};
use streamweave_graph::control_flow::{If, Match, Aggregate};
```

**After (streamweave::graph):**
```rust
use streamweave::graph::{GraphBuilder, GraphExecution};
use streamweave::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
use streamweave::graph::nodes::{If, Match, Aggregate};
```

### 2. Cargo.toml Changes

**Before:**
```toml
[dependencies]
streamweave = "0.7.0"
streamweave-graph = "0.6.0"
```

**After:**
```toml
[dependencies]
streamweave = "0.7.0"
# streamweave-graph is now included in streamweave
```

### 3. Node Creation Methods

Node creation methods have been updated for clarity:

**Before:**
```rust
let producer = ProducerNode::new(
    "source".to_string(),
    ArrayProducer::new([1, 2, 3]),
    vec!["out".to_string()],
);
```

**After:**
```rust
// Use from_producer, from_transformer, from_consumer for convenience
let producer = ProducerNode::from_producer(
    "source".to_string(),
    ArrayProducer::new([1, 2, 3]),
);
// Port names are automatically set based on the producer's port configuration
```

### 4. Message<T> Integration

**The most important change:** All data flowing through graphs is now automatically wrapped in `Message<T>`. You don't need to manually wrap/unwrap messages - nodes handle this internally.

**Before (if you were manually handling messages):**
```rust
// Manual message wrapping (no longer needed)
let msg = wrap_message(item);
```

**After (automatic):**
```rust
// Nodes automatically wrap/unwrap Message<T>
// ProducerNode wraps output in Message<T>
// TransformerNode unwraps Message<T>, transforms, wraps output
// ConsumerNode unwraps Message<T> before consuming
// You work with raw types, system handles Message<T>
```

### 5. Control Flow Node Imports

Control flow nodes are now in the `nodes` module:

**Before:**
```rust
use streamweave_graph::control_flow::{If, Match, Aggregate};
```

**After:**
```rust
use streamweave::graph::nodes::{If, Match, Aggregate};
```

### 6. Router Node Imports

Router nodes are also in the `nodes` module:

**Before:**
```rust
use streamweave_graph::{BroadcastRouter, RoundRobinRouter};
use streamweave_graph::router::{InputRouter, OutputRouter};
```

**After:**
```rust
use streamweave::graph::nodes::{BroadcastRouter, RoundRobinRouter};
use streamweave::graph::router::{InputRouter, OutputRouter};
```

## Migration Steps

### Step 1: Update Cargo.toml

Remove `streamweave-graph` from your dependencies:

```toml
[dependencies]
streamweave = "0.7.0"
# Remove: streamweave-graph = "0.6.0"
```

### Step 2: Update Imports

Replace all `streamweave_graph::` imports with `streamweave::graph::`:

```rust
// Find and replace:
// streamweave_graph:: → streamweave::graph::
// streamweave_graph::node:: → streamweave::graph::nodes::
// streamweave_graph::control_flow:: → streamweave::graph::nodes::
```

### Step 3: Update Node Creation

Update node creation to use the new convenience methods:

```rust
// Old
ProducerNode::new(name, producer, port_names)

// New
ProducerNode::from_producer(name, producer)
```

### Step 4: Remove Manual Message Handling

If you were manually wrapping/unwrapping messages, remove that code - nodes handle it automatically:

```rust
// Remove manual wrapping - nodes do this automatically
// let msg = wrap_message(item);  // No longer needed
```

### Step 5: Update Graph Execution

Graph execution API remains the same, but ensure you're using the correct imports:

```rust
use streamweave::graph::{GraphBuilder, GraphExecution};

let graph = GraphBuilder::new()
    .node(producer)?
    .node(transformer)?
    .node(consumer)?
    .connect_by_name("source", "transform")?
    .connect_by_name("transform", "sink")?
    .build();

let mut executor = graph.executor();
executor.start().await?;
// ... execution ...
executor.stop().await?;
```

## Complete Migration Example

### Before (streamweave-graph)

```rust
use streamweave_graph::{GraphBuilder, GraphExecution};
use streamweave_graph::node::{ProducerNode, TransformerNode, ConsumerNode};
use streamweave_graph::control_flow::If;
use streamweave_array::ArrayProducer;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = GraphBuilder::new();
    
    builder.add_node("source".to_string(), ProducerNode::new(
        "source".to_string(),
        ArrayProducer::new([1, 2, 3]),
        vec!["out".to_string()],
    ))?;
    
    builder.add_node("double".to_string(), TransformerNode::new(
        "double".to_string(),
        MapTransformer::new(|x: i32| x * 2),
        vec!["in".to_string()],
        vec!["out".to_string()],
    ))?;
    
    builder.add_node("sink".to_string(), ConsumerNode::new(
        "sink".to_string(),
        VecConsumer::<i32>::new(),
        vec!["in".to_string()],
    ))?;
    
    builder.connect_by_name("source", "double")?;
    builder.connect_by_name("double", "sink")?;
    
    let graph = builder.build();
    let mut executor = graph.executor();
    executor.start().await?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    executor.stop().await?;
    
    Ok(())
}
```

### After (streamweave::graph)

```rust
use streamweave::graph::{GraphBuilder, GraphExecution};
use streamweave::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
use streamweave::graph::nodes::If;
use streamweave_array::ArrayProducer;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use fluent API with convenience methods
    let graph = GraphBuilder::new()
        .node(ProducerNode::from_producer(
            "source".to_string(),
            ArrayProducer::new([1, 2, 3]),
        ))?
        .node(TransformerNode::from_transformer(
            "double".to_string(),
            MapTransformer::new(|x: i32| x * 2),
        ))?
        .node(ConsumerNode::from_consumer(
            "sink".to_string(),
            VecConsumer::<i32>::new(),
        ))?
        .connect_by_name("source", "double")?
        .connect_by_name("double", "sink")?
        .build();
    
    // Execution API is the same
    let mut executor = graph.executor();
    executor.start().await?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    executor.stop().await?;
    
    Ok(())
}
```

## Key Benefits of Migration

1. **Automatic Message<T> Handling**: No need to manually wrap/unwrap messages - nodes handle it automatically
2. **Message ID Tracking**: Every item gets a unique message ID for end-to-end traceability
3. **Metadata Preservation**: Message metadata (timestamps, source, headers) is preserved through transformations
4. **Error Correlation**: Errors include message IDs for correlating errors with specific messages
5. **Zero-Copy Optimizations**: In-process mode uses `Arc<Message<T>>` for efficient fan-out scenarios
6. **Simplified Dependencies**: One less package to manage

## Common Issues and Solutions

### Issue: "unresolved import `streamweave_graph`"

**Solution:** Update imports to use `streamweave::graph::` instead of `streamweave_graph::`

### Issue: "cannot find `node` in `streamweave::graph`"

**Solution:** Use `streamweave::graph::nodes::` instead of `streamweave::graph::node::`

### Issue: "cannot find `control_flow` in `streamweave::graph`"

**Solution:** Control flow nodes are now in `streamweave::graph::nodes::`

### Issue: "method `new` not found for `ProducerNode`"

**Solution:** Use `from_producer()`, `from_transformer()`, or `from_consumer()` convenience methods, or use `new()` with explicit port names if needed.

## Testing Your Migration

After migrating, verify your code:

1. **Compile Check**: Run `cargo check` to ensure all imports are correct
2. **Test Execution**: Run your tests to ensure graph execution works correctly
3. **Message Tracking**: Verify that message IDs and metadata are preserved (check error messages for message IDs)

## Additional Resources

- [Graph API Documentation](../packages/streamweave/README.md#graph-api) - Complete graph API reference
- [Message<T> Documentation](../packages/streamweave/README.md#-universal-message-model) - Message model details
- [Graph Examples](../../packages/graph/examples/) - Working examples

## Need Help?

If you encounter issues during migration:
1. Check that all imports use `streamweave::graph::` instead of `streamweave_graph::`
2. Verify node creation uses the new convenience methods
3. Ensure you've removed `streamweave-graph` from `Cargo.toml`
4. Review the examples in `packages/graph/examples/` for reference

