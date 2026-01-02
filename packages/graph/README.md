# streamweave-graph

[![Crates.io](https://img.shields.io/crates/v/streamweave-graph.svg)](https://crates.io/crates/streamweave-graph)
[![Documentation](https://docs.rs/streamweave-graph/badge.svg)](https://docs.rs/streamweave-graph)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Graph API for StreamWeave**  
*Build complex data processing topologies with fan-in, fan-out, routing, and subgraphs.*

The `streamweave-graph` package provides a powerful graph-based API for building complex data processing topologies. It supports multi-port connections, fan-in/fan-out patterns, routing strategies, subgraphs, and both compile-time and runtime graph construction.

## ✨ Key Features

- **Graph Builder**: Compile-time and runtime graph construction
- **Node Types**: ProducerNode, TransformerNode, ConsumerNode
- **Multi-Port Connections**: Connect nodes via multiple input/output ports
- **Routing Strategies**: Broadcast, Round-Robin, Key-Based, Merge
- **Control Flow**: If/Else routing, Pattern matching, Loops, Variables, Aggregations
- **Fan-In/Fan-Out**: Support for complex topologies
- **Subgraphs**: Nest graphs within graphs
- **Serialization**: Serialize and deserialize graphs
- **Stateful Processing**: Stateful transformers in graphs
- **Windowing**: Window operations in graphs
- **Zero-Copy Node Architecture**: Nodes use `Arc<Mutex<T>>` internally, eliminating `Clone` requirements

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-graph = "0.6.0"
```

## 🚀 Quick Start

### Simple Graph

```rust
use streamweave_graph::{GraphBuilder, node::{ProducerNode, TransformerNode, ConsumerNode}};
use streamweave_array::ArrayProducer;
use streamweave_vec::VecConsumer;
use streamweave_transformers::MapTransformer;

let mut builder = GraphBuilder::new();

// Add nodes
builder.add_node("source".to_string(), ProducerNode::new(
    "source".to_string(),
    ArrayProducer::new(vec![1, 2, 3]),
))?;

builder.add_node("mapper".to_string(), TransformerNode::new(
    "mapper".to_string(),
    MapTransformer::new(|x| x * 2),
))?;

builder.add_node("sink".to_string(), ConsumerNode::new(
    "sink".to_string(),
    VecConsumer::new(),
))?;

// Connect nodes
builder.connect("source", 0, "mapper", 0)?;
builder.connect("mapper", 0, "sink", 0)?;

// Build and execute
let graph = builder.build();
let executor = GraphExecutor::new(graph);
executor.execute().await?;
```

## 📖 API Overview

### GraphBuilder

The `GraphBuilder` constructs graphs with compile-time type validation:

```rust
pub struct GraphBuilder {
    // Internal state
}
```

**Key Methods:**
- `new()` - Create new builder
- `add_node(name, node)` - Add a node to the graph
- `connect(source, source_port, target, target_port)` - Connect nodes
- `build()` - Build the graph

### Node Types

**ProducerNode:**
- Wraps a Producer
- Has output ports
- Generates data streams

**TransformerNode:**
- Wraps a Transformer
- Has input and output ports
- Transforms data streams

**ConsumerNode:**
- Wraps a Consumer
- Has input ports
- Consumes data streams

### Routing Strategies

**Broadcast:**
- Sends each item to all connected outputs
- Useful for fan-out patterns

**Round-Robin:**
- Distributes items evenly across outputs
- Load balancing

**Key-Based:**
- Routes items based on a key function
- Consistent routing

**Merge:**
- Combines multiple inputs into one output
- Fan-in pattern

### Control Flow Constructs

StreamWeave Graph provides comprehensive control flow constructs for Flow-Based Programming patterns:

**Routers:**
- **If Router**: Conditional routing based on predicates (if/else)
- **Match Router**: Pattern-based routing (match/switch)
- **Error Branch Router**: Route `Result<T, E>` items to success/error ports

**Transformers:**
- **ForEach**: Iterate over collections and expand to individual items
- **While**: Conditional iteration (continue while condition is true)
- **Delay**: Add delays between items
- **Timeout**: Add timeout handling to stream processing
- **Aggregate**: Compute aggregations (Sum, Count, Min, Max)
- **GroupBy**: Group items by a key function
- **Join**: Join multiple streams with different strategies (Inner, Outer, Left, Right)
- **Synchronize**: Wait for multiple inputs before proceeding

**Variables:**
- **GraphVariables**: Shared state between nodes
- **ReadVariable**: Read variable values
- **WriteVariable**: Write variable values

For detailed examples, see the [Control Flow Examples](#-control-flow-examples) section below.

## 📚 Usage Examples

### Fan-Out Graph

Send data from one producer to multiple transformers:

```rust
let mut builder = GraphBuilder::new();

builder.add_node("source".to_string(), ProducerNode::new(
    "source".to_string(),
    ArrayProducer::new(vec![1, 2, 3]),
))?;

builder.add_node("transformer1".to_string(), TransformerNode::new(
    "transformer1".to_string(),
    MapTransformer::new(|x| x * 2),
))?;

builder.add_node("transformer2".to_string(), TransformerNode::new(
    "transformer2".to_string(),
    MapTransformer::new(|x| x + 1),
))?;

builder.add_node("consumer1".to_string(), ConsumerNode::new(
    "consumer1".to_string(),
    VecConsumer::new(),
))?;

builder.add_node("consumer2".to_string(), ConsumerNode::new(
    "consumer2".to_string(),
    VecConsumer::new(),
))?;

// Fan-out: source -> transformer1 -> consumer1
//          source -> transformer2 -> consumer2
builder.connect("source", 0, "transformer1", 0)?;
builder.connect("transformer1", 0, "consumer1", 0)?;
builder.connect("source", 0, "transformer2", 0)?;
builder.connect("transformer2", 0, "consumer2", 0)?;
```

### Fan-In Graph

Combine multiple producers into one consumer:

```rust
let mut builder = GraphBuilder::new();

builder.add_node("source1".to_string(), ProducerNode::new(
    "source1".to_string(),
    ArrayProducer::new(vec![1, 2, 3]),
))?;

builder.add_node("source2".to_string(), ProducerNode::new(
    "source2".to_string(),
    ArrayProducer::new(vec![4, 5, 6]),
))?;

builder.add_node("merger".to_string(), TransformerNode::new(
    "merger".to_string(),
    MergeTransformer::new(),
))?;

builder.add_node("sink".to_string(), ConsumerNode::new(
    "sink".to_string(),
    VecConsumer::new(),
))?;

// Fan-in: source1 -> merger -> sink
//         source2 -> merger -> sink
builder.connect("source1", 0, "merger", 0)?;
builder.connect("source2", 0, "merger", 1)?;
builder.connect("merger", 0, "sink", 0)?;
```

### Routing Strategies

Use different routing strategies for data distribution:

```rust
use streamweave_graph::{BroadcastRouter, RoundRobinRouter, KeyBasedRouter};

// Broadcast: Send to all outputs
let router = BroadcastRouter::new();
builder.set_output_router("source", 0, router)?;

// Round-Robin: Distribute evenly
let router = RoundRobinRouter::new();
builder.set_output_router("source", 0, router)?;

// Key-Based: Route by key
let router = KeyBasedRouter::new(|item: &i32| item % 2);
builder.set_output_router("source", 0, router)?;
```

### Multi-Port Connections

Connect nodes via multiple ports:

```rust
// Producer with multiple outputs
builder.add_node("source".to_string(), ProducerNode::new(
    "source".to_string(),
    MultiPortProducer::new(),
))?;

// Transformer with multiple inputs and outputs
builder.add_node("transformer".to_string(), TransformerNode::new(
    "transformer".to_string(),
    MultiPortTransformer::new(),
))?;

// Connect via specific ports
builder.connect("source", 0, "transformer", 0)?;
builder.connect("source", 1, "transformer", 1)?;
builder.connect("transformer", 0, "sink", 0)?;
```

### Subgraphs

Create nested graphs:

```rust
use streamweave_subgraph::Subgraph;

let mut subgraph_builder = GraphBuilder::new();
// Build subgraph...
let subgraph = subgraph_builder.build();

let mut main_builder = GraphBuilder::new();
main_builder.add_node("subgraph".to_string(), Subgraph::new(
    "subgraph".to_string(),
    subgraph,
))?;

// Connect to subgraph
builder.connect("source", 0, "subgraph", 0)?;
builder.connect("subgraph", 0, "sink", 0)?;
```

### Graph Execution

Execute graphs with the GraphExecutor:

```rust
use streamweave_graph::{GraphExecutor, ExecutionState};

let graph = builder.build();
let executor = GraphExecutor::new(graph);

// Execute graph
executor.execute().await?;

// Check execution state
match executor.state() {
    ExecutionState::Running => println!("Graph is running"),
    ExecutionState::Completed => println!("Graph completed"),
    ExecutionState::Error(e) => println!("Error: {}", e),
}
```

### Graph Serialization

Serialize and deserialize graphs:

```rust
use streamweave_graph::{serialize, deserialize};

// Serialize graph
let json = serialize(&graph)?;

// Deserialize graph
let graph: Graph = deserialize(&json)?;
```

## 🏗️ Architecture

Graphs support complex topologies:

```text
┌──────────┐
│ Producer │───port 0───>┌─────────────┐───port 0───>┌──────────┐
└──────────┘              │ Transformer │             │ Consumer │
                          └─────────────┘             └──────────┘
                                │
                                │ port 1
                                ▼
                          ┌──────────┐
                          │ Consumer │
                          └──────────┘
```

**Graph Components:**
- **Nodes**: Producer, Transformer, Consumer nodes
- **Connections**: Type-safe connections between ports
- **Routers**: Data routing strategies
- **Subgraphs**: Nested graph structures

## 🔧 Configuration

### Routing Configuration

Configure routing per output port:

```rust
// Broadcast routing
builder.set_output_router("node", 0, BroadcastRouter::new())?;

// Round-robin routing
builder.set_output_router("node", 0, RoundRobinRouter::new())?;

// Key-based routing
builder.set_output_router("node", 0, KeyBasedRouter::new(|item| item.key()))?;
```

### Error Handling

Configure error handling for graph execution:

```rust
use streamweave_error::ErrorStrategy;

let executor = GraphExecutor::new(graph)
    .with_error_strategy(ErrorStrategy::Skip);
```

## 🔍 Error Handling

Graph operations return `Result<T, GraphError>`:

```rust
pub enum GraphError {
    NodeNotFound { name: String },
    DuplicateNode { name: String },
    InvalidConnection { source: String, target: String, reason: String },
    PortNotFound { node: String, port: usize },
    TypeMismatch { expected: String, actual: String },
}
```

## ⚡ Performance Considerations

- **Compile-Time Validation**: Type-safe connections validated at compile time
- **Runtime Flexibility**: Runtime graph construction for dynamic topologies
- **Efficient Routing**: Optimized routing strategies for high throughput
- **Parallel Execution**: Nodes can execute in parallel when possible
- **Zero-Copy Node Sharing**: Nodes use `Arc<tokio::sync::Mutex<T>>` internally for zero-copy node sharing
- **No Clone Requirement**: Producers, transformers, and consumers do not need to implement `Clone`

## 🏗️ Architecture

### Zero-Copy Node Design

StreamWeave Graph uses a zero-copy node architecture where nodes store their components
(producers, transformers, consumers) as `Arc<tokio::sync::Mutex<T>>`. This design:

- **Eliminates Clone Requirements**: Components no longer need to implement `Clone`
- **Enables Control Flow**: Control flow transformers (Aggregate, Delay, While, etc.) can be used without Clone
- **Zero-Copy Sharing**: Nodes are shared via `Arc` cloning (atomic reference counting)
- **Thread-Safe Access**: Mutex ensures safe concurrent access when needed

### Accessing Node Components

To access a node's component, use the accessor methods which return `Arc<Mutex<T>>`:

```rust
let node = TransformerNode::new("mapper".to_string(), MapTransformer::new(|x| x * 2));
let transformer_arc = node.transformer();

// Lock and use the transformer
let mut transformer = transformer_arc.lock().await;
// Use transformer...
```

**Note**: The `*_mut()` methods have been removed. Use `lock().await` on the `Arc<Mutex<T>>` instead.

## 🔀 Control Flow Examples

### Conditional Routing (If Router)

Route items conditionally based on a predicate:

```rust
use streamweave_graph::control_flow::If;
use streamweave_graph::node::TransformerNode;
use streamweave_transformers::IdentityTransformer;

let if_router = If::new(|x: &i32| *x % 2 == 0);
let node = TransformerNode::new(
    "split".to_string(),
    IdentityTransformer::new(),
    if_router, // Routes even numbers to port 0, odd to port 1
);
```

### Error Branching

Route `Result<T, E>` items to separate success and error ports:

```rust
use streamweave_graph::control_flow::ErrorBranch;

let error_router = ErrorBranch::<i32, String>::new();
let node = TransformerNode::new(
    "split_errors".to_string(),
    IdentityTransformer::new(),
    error_router, // Routes Ok(items) to port 0, Err(errors) to port 1
);
```

### Pattern Matching (Match Router)

Route items based on pattern matching:

```rust
use streamweave_graph::control_flow::{Match, Pattern, RangePattern};

let patterns: Vec<Box<dyn Pattern<i32>>> = vec![
    Box::new(RangePattern::new(0..50, 0)),   // Low: port 0
    Box::new(RangePattern::new(50..100, 1)), // High: port 1
];
let router = Match::new(patterns, Some(2)); // Default: port 2
```

### Variables

Share state between nodes:

```rust
use streamweave_graph::control_flow::{GraphVariables, ReadVariable, WriteVariable};

let vars = GraphVariables::new();
vars.set("counter", 42i32);

// Read variable
let read_var = ReadVariable::new("counter".to_string(), vars.clone());
let read_node = TransformerNode::from_transformer("read".to_string(), read_var);

// Write variable
let write_var = WriteVariable::new("counter".to_string(), vars.clone());
let write_node = TransformerNode::from_transformer("write".to_string(), write_var);
```

### Aggregation

Compute aggregations over streams:

```rust
use streamweave_graph::control_flow::{Aggregate, SumAggregator};

let aggregate = Aggregate::new(SumAggregator, None); // Sum all items
let node = TransformerNode::from_transformer("sum".to_string(), aggregate);
```

### GroupBy

Group items by a key:

```rust
use streamweave_graph::control_flow::GroupBy;

let group_by = GroupBy::new(|item: &(String, i32)| item.0.clone());
let node = TransformerNode::from_transformer("group".to_string(), group_by);
```

For complete runnable examples of all control flow features, see the [examples directory](https://github.com/Industrial/streamweave/tree/main/packages/graph/examples).

## 📝 Examples

For more examples, see:
- [Graph Examples](https://github.com/Industrial/streamweave/tree/main/examples)
- [Control Flow Examples](https://github.com/Industrial/streamweave/tree/main/packages/graph/examples)
- [Complex Topologies](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-graph` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-pipeline` - Pipeline integration
- `streamweave-stateful` - Stateful processing
- `streamweave-window` - Windowing operations
- `streamweave-transformers` - Transformer implementations
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization

## 🎯 Use Cases

Graphs are used for:

1. **Complex Topologies**: Build complex data processing workflows
2. **Fan-In/Fan-Out**: Distribute and combine data streams
3. **Routing**: Route data based on content or load
4. **Subgraphs**: Modular graph composition
5. **Dynamic Topologies**: Runtime graph construction

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-graph)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/graph)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API
- [streamweave-stateful](../stateful/README.md) - Stateful processing
- [streamweave-window](../window/README.md) - Windowing operations

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

