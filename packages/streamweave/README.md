# streamweave

[![Crates.io](https://img.shields.io/crates/v/streamweave.svg)](https://crates.io/crates/streamweave)
[![Documentation](https://docs.rs/streamweave/badge.svg)](https://docs.rs/streamweave)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Core traits and types for StreamWeave**  
*The foundational abstractions that power all StreamWeave data processing.*

The `streamweave` package provides the core traits and types that form the foundation of the StreamWeave framework. All other StreamWeave packages depend on these core abstractions to build producers, transformers, and consumers.

## 🎯 Universal Message Model

**All data in StreamWeave flows as `Message<T>`.** This universal message model ensures that every piece of data has:
- **MessageId**: Unique identifier for tracking and correlation
- **MessageMetadata**: Timestamps, source information, headers, and custom attributes
- **Payload**: Your actual data (`T`)

This design enables:
- **End-to-end traceability**: Track messages through complex pipelines
- **Metadata preservation**: Pass context through transformations
- **Error correlation**: Link errors to specific messages
- **Zero-copy sharing**: Efficient message sharing in fan-out scenarios

### Working with Messages

**Direct Message Usage (Advanced):**
```rust
use streamweave::message::{Message, MessageId, MessageMetadata, wrap_message};

// Create a message with automatic ID generation
let msg = wrap_message(42);

// Access message components
let payload = msg.payload();        // &i32
let id = msg.id();                  // &MessageId
let metadata = msg.metadata();      // &MessageMetadata

// Create message with custom metadata
let metadata = MessageMetadata::default()
    .source("my_source")
    .header("key", "value");
let msg = Message::with_metadata(42, MessageId::new_uuid(), metadata);
```

**Adapter-Based Usage (Simple):**
For simple cases where you just want to work with raw types, use adapters:

```rust,no_run
use streamweave::adapters::{MessageWrapper, PayloadExtractor, PayloadExtractorConsumer};

// Wrap a raw producer (produces raw types)
// let raw_producer = MyRawProducer::new();
// let producer = MessageWrapper::new(raw_producer);  // Now produces Message<T>

// Extract payloads in transformer (works with raw types internally)
// let raw_transformer = MyRawTransformer::new();
// let transformer = PayloadExtractor::new(raw_transformer);  // Input/Output: Message<T>

// Extract payloads in consumer (receives raw types)
// let raw_consumer = MyRawConsumer::new();
// let consumer = PayloadExtractorConsumer::new(raw_consumer);  // Input: Message<T>
```

See the [Adapters](#-adapters) section for more details.

## ✨ Key Features

- **Universal Message Model**: All data flows as `Message<T>` with IDs and metadata
- **Producer Trait**: Define components that generate data streams
- **Transformer Trait**: Define components that transform data streams
- **Consumer Trait**: Define components that consume data streams
- **Graph API**: Build complex data flow topologies with multiple nodes and connections
- **Adapter Patterns**: Work with raw types while system uses messages internally
- **Input/Output Traits**: Type-safe stream interfaces
- **Port System**: Type-safe multi-port connections for graph-based processing
- **Configuration System**: Unified configuration for error handling and component naming
- **Error Handling Integration**: Seamless integration with `streamweave-error`

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave = "0.6.0"
```

## 🚀 Quick Start

### Basic Message Usage

All data in StreamWeave flows as `Message<T>`. Here's a simple example:

```rust
use streamweave::message::{Message, MessageId, wrap_message};

// Create a message (automatic ID generation)
let msg = wrap_message(42);

// Access the payload
let value = msg.payload();  // &i32

// Access message ID and metadata
let id = msg.id();
let metadata = msg.metadata();
```

### Working with Producers, Transformers, and Consumers

All components work with `Message<T>`:

```rust
use streamweave::{Producer, Transformer, Consumer};
use streamweave::message::Message;

// Producers yield Message<T>
// Transformers receive Message<T> and produce Message<U>
// Consumers receive Message<T>
```

For complete working examples, see:
- [Pipeline Examples](../pipeline/README.md) - Linear pipeline execution
- [Graph API](#graph-api) - Complex graph topologies with Message<T>
- [Examples Directory](https://github.com/Industrial/streamweave/tree/main/examples) - Additional examples

## 📖 API Overview

### Producer Trait

The `Producer` trait defines components that generate data streams. Producers are the starting point of any StreamWeave pipeline.

```text
// Producer trait signature (simplified for documentation)
// 
// trait Producer: Output {
//     type OutputPorts: PortList;
//     fn produce(&mut self) -> Self::OutputStream;
//     fn with_config(&self, config: ProducerConfig<Self::Output>) -> Self;
//     fn with_name(self, name: String) -> Self;
//     fn handle_error(&self, error: &StreamError<Self::Output>) -> ErrorAction;
// }
```

**Key Methods:**
- `produce()` - Generates the output stream
- `with_config()` - Applies configuration (error strategy, name)
- `handle_error()` - Handles errors according to configured strategy

**Example Producer Implementation:**

```rust,no_run
use streamweave::{Producer, Output, ProducerConfig};
use streamweave::message::{Message, MessageId, wrap_message};
use streamweave_error::ErrorStrategy;
use futures::Stream;
use std::pin::Pin;

struct NumberProducer {
    numbers: Vec<i32>,
    config: ProducerConfig<Message<i32>>,
}

impl Output for NumberProducer {
    type Output = Message<i32>;
    type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Producer for NumberProducer {
    type OutputPorts = (Message<i32>,);
    
    fn produce(&mut self) -> Self::OutputStream {
        let numbers = self.numbers.clone();
        Box::pin(futures::stream::iter(
            numbers.into_iter().map(|n| wrap_message(n))
        ))
    }
    
    fn set_config_impl(&mut self, config: ProducerConfig<Self::Output>) {
        self.config = config;
    }
    
    fn get_config_impl(&self) -> &ProducerConfig<Self::Output> {
        &self.config
    }
    
    fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<Self::Output> {
        &mut self.config
    }
}
```

### Transformer Trait

The `Transformer` trait defines components that transform data streams. Transformers process items as they flow through the pipeline.

```text
// Transformer trait signature (simplified for documentation)
//
// trait Transformer: Input + Output {
//     type InputPorts: PortList;
//     type OutputPorts: PortList;
//     async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream;
//     fn with_config(&self, config: TransformerConfig<Self::Input>) -> Self;
//     fn with_name(self, name: String) -> Self;
//     fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction;
// }
```

**Key Methods:**
- `transform()` - Transforms the input stream into an output stream
- `with_config()` - Applies configuration (error strategy, name)
- `handle_error()` - Handles errors according to configured strategy

**Example Transformer Implementation:**

```rust,no_run
use streamweave::{Transformer, Input, Output, TransformerConfig};
use streamweave::message::{Message, MessageId};
use streamweave_error::ErrorStrategy;
use futures::StreamExt;
use std::pin::Pin;
use tokio_stream::Stream;

struct DoubleTransformer {
    config: TransformerConfig<Message<i32>>,
}

impl Input for DoubleTransformer {
    type Input = Message<i32>;
    type InputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

impl Output for DoubleTransformer {
    type Output = Message<i32>;
    type OutputStream = Pin<Box<dyn Stream<Item = Message<i32>> + Send>>;
}

#[async_trait::async_trait]
impl Transformer for DoubleTransformer {
    type InputPorts = (Message<i32>,);
    type OutputPorts = (Message<i32>,);
    
    async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
        Box::pin(input.map(|msg| {
            let payload = msg.payload().clone();
            let id = msg.id().clone();
            let metadata = msg.metadata().clone();
            Message::with_metadata(payload * 2, id, metadata)
        }))
    }
    
    fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
        self.config = config;
    }
    
    fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
        &self.config
    }
    
    fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
        &mut self.config
    }
}
```

### Consumer Trait

The `Consumer` trait defines components that consume data streams. Consumers are the end point of a pipeline.

```text
// Consumer trait signature (simplified for documentation)
//
// trait Consumer: Input {
//     type InputPorts: PortList;
//     async fn consume(&mut self, stream: Self::InputStream);
//     fn with_config(&self, config: ConsumerConfig<Self::Input>) -> Self;
//     fn with_name(self, name: String) -> Self;
//     fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction;
// }
```

**Key Methods:**
- `consume()` - Consumes the input stream (async)
- `with_config()` - Applies configuration (error strategy, name)
- `handle_error()` - Handles errors according to configured strategy

**Example Consumer Implementation:**

```rust,no_run
use streamweave::{Consumer, Input, ConsumerConfig};
use streamweave::message::Message;
use streamweave_error::ErrorStrategy;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::Stream;

struct VecConsumer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
    items: Arc<Mutex<Vec<Message<T>>>>,
    config: ConsumerConfig<Message<T>>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Input for VecConsumer<T> {
    type Input = Message<T>;
    type InputStream = Pin<Box<dyn Stream<Item = Message<T>> + Send>>;
}

#[async_trait::async_trait]
impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> Consumer for VecConsumer<T> {
    type InputPorts = (Message<T>,);
    
    async fn consume(&mut self, mut stream: Self::InputStream) {
        while let Some(item) = stream.next().await {
            self.items.lock().await.push(item);
        }
    }
    
    fn set_config_impl(&mut self, config: ConsumerConfig<Self::Input>) {
        self.config = config;
    }
    
    fn get_config_impl(&self) -> &ConsumerConfig<Self::Input> {
        &self.config
    }
    
    fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<Self::Input> {
        &mut self.config
    }
}
```

### Input and Output Traits

The `Input` and `Output` traits define the stream interfaces for components.

**Input Trait:**
```rust,no_run
use tokio_stream::Stream;

pub trait Input {
    type Input;
    type InputStream: Stream<Item = Self::Input> + Send;
}
```

**Output Trait:**
```rust,no_run
use tokio_stream::Stream;

pub trait Output {
    type Output;
    type OutputStream: Stream<Item = Self::Output> + Send;
}
```

These traits ensure type safety and enable components to be composed together in pipelines and graphs.

### Message Module

The `message` module provides the core message types and utilities:

- **`Message<T>`**: The universal message envelope containing payload, ID, and metadata
- **`MessageId`**: Unique identifier (UUID or sequence-based)
- **`MessageMetadata`**: Timestamps, source, headers, and custom attributes
- **`IdGenerator`**: Trait for generating message IDs
- **Helper Functions**: `wrap_message()`, `unwrap_message()`, etc.

**Key Message Operations:**

```rust
use streamweave::message::{Message, MessageId, MessageMetadata, wrap_message};

// Create messages
let msg1 = wrap_message(42);  // Auto-generates UUID
let msg2 = Message::new(100, MessageId::new_uuid());

// Access components
let payload = msg1.payload();      // &T
let id = msg1.id();                // &MessageId
let metadata = msg1.metadata();     // &MessageMetadata

// Transform payload while preserving ID and metadata
let doubled = msg1.map(|x| x * 2);

// Work with metadata
let metadata = MessageMetadata::default()
    .source("my_source")
    .header("key", "value")
    .timestamp(std::time::Duration::from_secs(1234567890));
```

See the [message module documentation](https://docs.rs/streamweave/latest/streamweave/message/index.html) for complete API details.

### Adapters

Adapters allow you to work with raw types while the system uses `Message<T>` internally:

- **`MessageWrapper`**: Wraps a `RawProducer` to produce `Message<T>`
- **`PayloadExtractor`**: Extracts payloads for `RawTransformer`, wraps output back into `Message<U>`
- **`PayloadExtractorConsumer`**: Extracts payloads for `RawConsumer`

**When to Use Adapters:**
- Simple transformations that don't need message metadata
- Migrating existing code that works with raw types
- Quick prototyping

**When to Use Messages Directly:**
- Need to track messages through the pipeline
- Want to preserve or modify metadata
- Building advanced routing or correlation logic
- Error handling that needs message context

See the [adapters module documentation](https://docs.rs/streamweave/latest/streamweave/adapters/index.html) for details.

### Port System

The port system enables type-safe multi-port connections in the Graph API. Ports are represented as tuples, allowing components to have multiple inputs or outputs.

```rust,no_run
use streamweave::port::{PortList, GetPort};

// Single port
type SinglePort = (i32,);

// Multiple ports
type MultiPort = (i32, String, bool);

// Extract port types at compile time
type FirstPort = <MultiPort as GetPort<0>>::Type;  // i32
type SecondPort = <MultiPort as GetPort<1>>::Type; // String
type ThirdPort = <MultiPort as GetPort<2>>::Type;  // bool
```

The port system supports up to 12 ports per component, with compile-time type checking.

### Graph API

The Graph API enables you to create complex data flow topologies with multiple producers, transformers, and consumers. All data flowing through graphs is automatically wrapped in `Message<T>`, providing consistent metadata and ID tracking throughout the graph.

**Key Features:**
- **Type-Safe Graph Construction**: Compile-time type checking ensures nodes can be safely connected
- **Message-Based Data Flow**: All data flows as `Message<T>` with automatic ID and metadata preservation
- **Flexible Topologies**: Support for fan-out, fan-in, and complex routing patterns
- **Execution Modes**: In-process (zero-copy) and distributed (serialized) execution modes
- **Control Flow Nodes**: Built-in support for conditionals, loops, aggregation, and more
- **Router Nodes**: Broadcast, round-robin, key-based, and merge routing strategies

#### Creating a Graph

Use `GraphBuilder` to create graphs with a fluent API:

```rust
use streamweave::graph::{GraphBuilder, GraphExecution};
use streamweave::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
use streamweave_array::ArrayProducer;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

// Create a simple linear graph: producer -> transformer -> consumer
let graph = GraphBuilder::new()
    .node(ProducerNode::from_producer(
        "source".to_string(),
        ArrayProducer::new([1, 2, 3, 4, 5]),
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
```

#### Adding Nodes

Nodes wrap your producers, transformers, and consumers to enable graph execution:

**ProducerNode:**
```rust
use streamweave::graph::nodes::ProducerNode;
use streamweave_array::ArrayProducer;

let producer_node = ProducerNode::from_producer(
    "source".to_string(),
    ArrayProducer::new([1, 2, 3]),
);
```

**TransformerNode:**
```rust
use streamweave::graph::nodes::TransformerNode;
use streamweave_transformers::MapTransformer;

let transformer_node = TransformerNode::from_transformer(
    "transform".to_string(),
    MapTransformer::new(|x: i32| x * 2),
);
```

**ConsumerNode:**
```rust
use streamweave::graph::nodes::ConsumerNode;
use streamweave_vec::VecConsumer;

let consumer_node = ConsumerNode::from_consumer(
    "sink".to_string(),
    VecConsumer::<i32>::new(),
);
```

#### Connecting Nodes

Connect nodes using `connect_by_name()`:

```rust
// Connect source to transformer
builder.connect_by_name("source", "transform")?;

// Connect transformer to sink
builder.connect_by_name("transform", "sink")?;
```

For multi-port nodes, specify port names:

```rust
// Connect specific ports
builder.connect_by_name("source:out", "transform:in")?;
```

#### Executing Graphs

Execute graphs using the `GraphExecutor`:

```rust
use streamweave::graph::GraphExecution;

// Create executor from graph
let mut executor = graph.executor();

// Start execution
executor.start().await?;

// Wait for processing
tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

// Stop execution
executor.stop().await?;

// Check for errors
let errors = executor.errors();
if !errors.is_empty() {
    eprintln!("Graph execution had {} errors", errors.len());
}
```

#### Message<T> in Graphs

All data flowing through graphs is automatically wrapped in `Message<T>`. Nodes handle message wrapping and unwrapping internally:

- **ProducerNode**: Wraps producer output in `Message<T>` before sending
- **TransformerNode**: Unwraps `Message<T>` for transformation, wraps output back in `Message<T>`
- **ConsumerNode**: Unwraps `Message<T>` before passing to consumer

Message IDs and metadata are automatically preserved through the graph, enabling:
- End-to-end message tracking
- Error correlation with specific messages
- Metadata propagation through transformations

#### Execution Modes

Graphs support different execution modes:

**In-Process Mode (Default):**
- Zero-copy message sharing using `Arc<Message<T>>`
- Highest performance for single-process execution
- Automatic fallback to serialization when needed

**Distributed Mode:**
- Serialized message transmission using JSON or other formats
- Enables multi-process execution
- Supports compression and batching

```rust
use streamweave::graph::execution::ExecutionMode;

let graph = GraphBuilder::new()
    .with_execution_mode(ExecutionMode::Distributed {
        serialization: streamweave::graph::serialization::SerializationFormat::Json,
        compression: None,
    })
    // ... add nodes and connections
    .build();
```

#### Complex Topologies

Graphs support complex topologies with fan-out, fan-in, and routing:

```rust
// Fan-out: One producer to multiple transformers
builder
    .node(producer_node)?
    .node(transformer1)?
    .node(transformer2)?
    .connect_by_name("source", "transform1")?
    .connect_by_name("source", "transform2")?;

// Fan-in: Multiple producers to one consumer
builder
    .node(producer1)?
    .node(producer2)?
    .node(consumer)?
    .connect_by_name("source1", "sink")?
    .connect_by_name("source2", "sink")?;
```

#### Router Nodes

Router nodes enable advanced routing patterns:

- **BroadcastRouter**: Broadcasts messages to all output ports
- **RoundRobinRouter**: Distributes messages in round-robin fashion
- **KeyBasedRouter**: Routes messages based on a key function
- **MergeRouter**: Merges multiple input streams

Router nodes automatically handle `Message<T>` wrapping and unwrapping, preserving message IDs and metadata.

#### Control Flow Nodes

Control flow nodes provide advanced data processing patterns:

- **If**: Conditional routing based on predicates
- **Match**: Pattern matching and routing
- **Aggregate**: Aggregate items using various aggregators
- **GroupBy**: Group items by a key
- **Join**: Join multiple input streams
- **Delay**: Delay items by a specified duration
- **Timeout**: Apply timeouts to operations
- **While**: Loop constructs
- **ForEach**: Process each item in a collection

All control flow nodes preserve `Message<T>` IDs and metadata.

#### Subgraphs

Subgraphs allow you to use graphs as nodes within other graphs, enabling hierarchical composition:

```rust
use streamweave::graph::nodes::SubgraphNode;

// Create an inner graph
let inner_graph = GraphBuilder::new()
    .node(inner_producer)?
    .node(inner_transformer)?
    .node(inner_consumer)?
    .connect_by_name("inner_source", "inner_transform")?
    .connect_by_name("inner_transform", "inner_sink")?
    .build();

// Use as a subgraph node
let subgraph_node = SubgraphNode::new(
    "subgraph".to_string(),
    inner_graph,
    vec!["in".to_string()],      // Input port names
    vec!["out".to_string()],     // Output port names
);

// Use in outer graph
let outer_graph = GraphBuilder::new()
    .node(outer_producer)?
    .node(subgraph_node)?
    .node(outer_consumer)?
    .connect_by_name("outer_source", "subgraph")?
    .connect_by_name("subgraph", "outer_sink")?
    .build();
```

Messages flow correctly through subgraph boundaries, preserving IDs and metadata.

#### Error Handling in Graphs

Graph execution errors include message context:

```rust
use streamweave::graph::execution::ExecutionError;

let errors = executor.errors();
for error in errors {
    match error {
        ExecutionError::NodeExecutionFailed { node, reason, message_id, .. } => {
            eprintln!("Node {} failed: {} (message_id: {:?})", node, reason, message_id);
        }
        ExecutionError::SerializationError { node, reason, message_id, .. } => {
            eprintln!("Serialization error in {}: {} (message_id: {:?})", node, reason, message_id);
        }
        // ... other error types
        _ => {}
    }
}
```

All error types include optional `message_id` fields for correlating errors with specific messages.

#### Graph Examples

For more examples, see:
- [Basic Graph Example](../../packages/graph/examples/basic_graph.rs)
- [Complex Topology Example](../../packages/graph/examples/complex_topology.rs)
- [Router Examples](../../packages/graph/examples/)
- [Control Flow Examples](../../packages/graph/examples/control_flow/)

### Configuration System

All components support configuration through `ProducerConfig`, `TransformerConfig`, and `ConsumerConfig`:

```rust,no_run
use streamweave::{ProducerConfig, Producer};
use streamweave_error::ErrorStrategy;
use streamweave::message::Message;

// Configure error handling
let config = ProducerConfig::<Message<i32>>::default()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("my_producer".to_string());

// let producer = producer.with_config(config);
```

**Configuration Options:**
- `error_strategy` - How to handle errors (Stop, Skip, Retry, Custom)
- `name` - Component name for logging and metrics

## 📚 Usage Examples

### Creating a Producer

```rust,no_run
use streamweave::{Producer, Output, ProducerConfig};
use streamweave::message::Message;
use streamweave_error::ErrorStrategy;

// Create a producer with error handling
// let producer = MyProducer::new()
//     .with_config(
//         ProducerConfig::<Message<i32>>::default()
//             .with_error_strategy(ErrorStrategy::Retry(3))
//             .with_name("data_source".to_string())
//     );
```

### Creating a Transformer

```rust,no_run
use streamweave::{Transformer, TransformerConfig};
use streamweave::message::Message;
use streamweave_error::ErrorStrategy;

// Create a transformer with error handling
// let transformer = MyTransformer::new()
//     .with_config(
//         TransformerConfig::<Message<i32>>::default()
//             .with_error_strategy(ErrorStrategy::Skip)
//             .with_name("data_processor".to_string())
//     );
```

### Creating a Consumer

```rust,no_run
use streamweave::{Consumer, ConsumerConfig};
use streamweave::message::Message;
use streamweave_error::ErrorStrategy;

// Create a consumer with error handling
// let mut consumer = MyConsumer::new();
// let mut config = ConsumerConfig::<Message<i32>>::default();
// config.error_strategy = ErrorStrategy::Stop;
// config.name = "data_sink".to_string();
// consumer.set_config(config);
```

### Error Handling Strategies

All components support multiple error handling strategies:

```rust,ignore
use streamweave_error::ErrorStrategy;
use streamweave::message::Message;

// Stop on first error (default)
let _stop = ErrorStrategy::<Message<i32>>::Stop;

// Skip errors and continue processing
let _skip = ErrorStrategy::<Message<i32>>::Skip;

// Retry up to N times
ErrorStrategy::Retry(3)

// Custom error handler
ErrorStrategy::new_custom(|error| {
    // Custom logic
    ErrorAction::Skip
})
```

## 🏗️ Architecture

The streamweave core package provides the foundational abstractions with a universal message model:

```text
┌─────────────┐
│   Producer  │───produces───> Stream<Message<T>>
└─────────────┘
      │
      │ Each item is Message<T> with:
      │ - Unique MessageId
      │ - MessageMetadata (timestamp, source, headers)
      │ - Payload (your data)
      ▼
┌─────────────┐
│ Transformer │───transforms───> Stream<Message<U>>
└─────────────┘
      │
      │ Messages preserve ID and metadata
      │ through transformations
      ▼
┌─────────────┐
│  Consumer   │───consumes───> Stream<Message<T>>
└─────────────┘
```

**Key Architectural Principles:**
- **Universal Messages**: All data flows as `Message<T>` - no exceptions
- **Metadata Preservation**: Message IDs and metadata are preserved through transformations
- **Adapter Support**: Adapters allow working with raw types while system uses messages
- **Type Safety**: Compile-time guarantees that components can be connected
- **Zero-Copy**: Efficient message sharing in fan-out scenarios using `Arc`

All components:
- Work with `Message<T>` types
- Implement `Input` and/or `Output` traits for type safety
- Support configuration for error handling and naming
- Integrate with the error handling system (error contexts include full `Message<T>`)
- Can be used in both Pipeline and Graph APIs

## 🔗 Dependencies

`streamweave` depends on:

- `tokio` - Async runtime
- `futures` - Stream abstractions
- `async-trait` - Async trait support
- `chrono` - Timestamp support
- `streamweave-error` - Error handling system

## 🎯 Use Cases

The core traits are used to:

1. **Build Custom Components**: Create producers, transformers, and consumers for specific use cases
2. **Type-Safe Composition**: Ensure components can be safely connected in pipelines
3. **Error Handling**: Provide consistent error handling across all components
4. **Graph API**: Enable multi-port connections in complex topologies
5. **Configuration**: Standardize component configuration and naming

## 🔍 Error Handling

All components integrate with the `streamweave-error` package for consistent error handling:

- **Error Strategies**: Stop, Skip, Retry, or Custom handlers
- **Error Context**: Automatic error context creation with timestamps and component info
- **Message Context**: Error contexts include the full `Message<T>` that caused the error, providing access to message ID and metadata
- **Component Info**: Automatic component identification for error reporting

When an error occurs, the `ErrorContext` contains the `Message<T>` that caused it, allowing you to:
- Track which message failed using its `MessageId`
- Access message metadata for debugging
- Correlate errors across the pipeline
- Implement custom error handling based on message content

## ⚡ Performance

StreamWeave is designed for high-throughput streaming workloads with a focus on zero-cost abstractions and efficient execution modes.

### Throughput Benchmarks

Based on our benchmark suite, StreamWeave achieves the following throughput for simple pipelines:

| Mode | Configuration | Throughput |
|------|--------------|------------|
| **In-Process** | Simple producer → consumer | **6+ million items/second** |
| **In-Process** | With transformation | **2.9-3.0 million items/second** |
| **Distributed** | With JSON serialization | **2.5-2.6 million items/second** |

**How many messages can StreamWeave process per second?**

**Short answer:** StreamWeave can process **2-6 million messages per second** depending on your configuration and workload.

**Detailed answer:**
- **In-process pipelines** (zero-copy): **3-6 million messages/second** for simple workloads
- **Distributed pipelines** (with serialization): **2-3 million messages/second** 
- **With transformations**: Throughput depends on transformation complexity, typically **2-3 million messages/second**
- **Fan-out scenarios**: Throughput scales linearly with fan-out degree using zero-copy sharing

**Important caveats:**
- These numbers are for simple integer operations and may vary significantly based on:
  - Data size and complexity
  - Transformation operations (CPU-bound transformations reduce throughput)
  - I/O operations (file/network operations become the bottleneck)
  - System resources (CPU, memory, I/O bandwidth)
- **In-process mode** uses zero-copy optimizations and is typically 2-3x faster than distributed mode
- **Distributed mode** includes serialization overhead (JSON by default), which reduces throughput but enables multi-process execution
- Real-world workloads (with I/O, complex transformations, etc.) will typically achieve **100K-1M messages/second** depending on your specific operations

### Performance Characteristics

- **Zero-Cost Abstractions**: Traits compile to efficient code with no runtime overhead
- **Stream-Based**: All processing is stream-based for memory efficiency
- **Async**: Full async/await support for concurrent processing
- **Type Safety**: Compile-time type checking prevents runtime errors
- **Zero-Copy Optimizations**: In-process mode uses `Arc` sharing for efficient fan-out scenarios
- **Memory Efficient**: Streaming architecture bounds memory usage regardless of data volume

For detailed performance analysis and optimization strategies, see the [zero-copy architecture documentation](../../docs/zero-copy-architecture.md).

## 📝 Examples

For more examples, see:
- [Pipeline Examples](https://github.com/Industrial/streamweave/tree/main/examples/basic_pipeline) - Linear pipeline execution
- [Graph API](#graph-api) - Graph API usage with Message<T>
- [Graph Example Files](../../packages/graph/examples/) - Complete graph examples
- [Package Implementations](../) - See specific packages for concrete implementations

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/streamweave)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave-error](../error/README.md) - Error handling system
- [streamweave-pipeline](../pipeline/README.md) - Pipeline builder and execution
- [Graph API Documentation](#graph-api) - Graph API for complex topologies (included in streamweave)

**Note**: The message module is now part of the core `streamweave` package. See the [message module documentation](https://docs.rs/streamweave/latest/streamweave/message/index.html) for details.

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

