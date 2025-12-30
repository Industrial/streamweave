# streamweave

[![Crates.io](https://img.shields.io/crates/v/streamweave.svg)](https://crates.io/crates/streamweave)
[![Documentation](https://docs.rs/streamweave/badge.svg)](https://docs.rs/streamweave)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Core traits and types for StreamWeave**  
*The foundational abstractions that power all StreamWeave data processing.*

The `streamweave` package provides the core traits and types that form the foundation of the StreamWeave framework. All other StreamWeave packages depend on these core abstractions to build producers, transformers, and consumers.

## ✨ Key Features

- **Producer Trait**: Define components that generate data streams
- **Transformer Trait**: Define components that transform data streams
- **Consumer Trait**: Define components that consume data streams
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

### Basic Pipeline Example

```rust
use streamweave::{
    Producer, Transformer, Consumer,
    Input, Output,
};

// This example shows the core traits in action
// See specific package implementations for concrete examples
```

For a complete working example, see the [pipeline package](../pipeline/README.md) or check out the [examples directory](https://github.com/Industrial/streamweave/tree/main/examples).

## 📖 API Overview

### Producer Trait

The `Producer` trait defines components that generate data streams. Producers are the starting point of any StreamWeave pipeline.

```rust
use streamweave::Producer;

#[async_trait::async_trait]
trait Producer: Output {
    type OutputPorts: PortList;
    
    fn produce(&mut self) -> Self::OutputStream;
    
    // Configuration methods
    fn with_config(&self, config: ProducerConfig<Self::Output>) -> Self;
    fn with_name(self, name: String) -> Self;
    
    // Error handling
    fn handle_error(&self, error: &StreamError<Self::Output>) -> ErrorAction;
}
```

**Key Methods:**
- `produce()` - Generates the output stream
- `with_config()` - Applies configuration (error strategy, name)
- `handle_error()` - Handles errors according to configured strategy

**Example Producer Implementation:**

```rust
use streamweave::{Producer, Output, ProducerConfig};
use streamweave_error::ErrorStrategy;
use futures::Stream;
use std::pin::Pin;

struct NumberProducer {
    numbers: Vec<i32>,
    config: ProducerConfig<i32>,
}

impl Output for NumberProducer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

#[async_trait::async_trait]
impl Producer for NumberProducer {
    type OutputPorts = (i32,);
    
    fn produce(&mut self) -> Self::OutputStream {
        Box::pin(futures::stream::iter(self.numbers.clone()))
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

```rust
use streamweave::Transformer;

#[async_trait::async_trait]
trait Transformer: Input + Output {
    type InputPorts: PortList;
    type OutputPorts: PortList;
    
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream;
    
    // Configuration methods
    fn with_config(&self, config: TransformerConfig<Self::Input>) -> Self;
    fn with_name(self, name: String) -> Self;
    
    // Error handling
    fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction;
}
```

**Key Methods:**
- `transform()` - Transforms the input stream into an output stream
- `with_config()` - Applies configuration (error strategy, name)
- `handle_error()` - Handles errors according to configured strategy

**Example Transformer Implementation:**

```rust
use streamweave::{Transformer, Input, Output, TransformerConfig};
use streamweave_error::ErrorStrategy;
use futures::StreamExt;
use std::pin::Pin;

struct DoubleTransformer {
    config: TransformerConfig<i32>,
}

impl Input for DoubleTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

impl Output for DoubleTransformer {
    type Output = i32;
    type OutputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

#[async_trait::async_trait]
impl Transformer for DoubleTransformer {
    type InputPorts = (i32,);
    type OutputPorts = (i32,);
    
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
        Box::pin(input.map(|x| x * 2))
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

```rust
use streamweave::Consumer;

#[async_trait::async_trait]
trait Consumer: Input {
    type InputPorts: PortList;
    
    async fn consume(&mut self, stream: Self::InputStream);
    
    // Configuration methods
    fn with_config(&self, config: ConsumerConfig<Self::Input>) -> Self;
    fn with_name(self, name: String) -> Self;
    
    // Error handling
    fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction;
}
```

**Key Methods:**
- `consume()` - Consumes the input stream (async)
- `with_config()` - Applies configuration (error strategy, name)
- `handle_error()` - Handles errors according to configured strategy

**Example Consumer Implementation:**

```rust
use streamweave::{Consumer, Input, ConsumerConfig};
use streamweave_error::ErrorStrategy;
use futures::StreamExt;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

struct VecConsumer<T> {
    items: Arc<Mutex<Vec<T>>>,
    config: ConsumerConfig<T>,
}

impl<T: Send + Sync + 'static> Input for VecConsumer<T> {
    type Input = T;
    type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait::async_trait]
impl<T: Send + Sync + 'static> Consumer for VecConsumer<T> {
    type InputPorts = (T,);
    
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
```rust
pub trait Input {
    type Input;
    type InputStream: Stream<Item = Self::Input> + Send;
}
```

**Output Trait:**
```rust
pub trait Output {
    type Output;
    type OutputStream: Stream<Item = Self::Output> + Send;
}
```

These traits ensure type safety and enable components to be composed together in pipelines and graphs.

### Port System

The port system enables type-safe multi-port connections in the Graph API. Ports are represented as tuples, allowing components to have multiple inputs or outputs.

```rust
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

### Configuration System

All components support configuration through `ProducerConfig`, `TransformerConfig`, and `ConsumerConfig`:

```rust
use streamweave_error::ErrorStrategy;

// Configure error handling
let config = ProducerConfig::default()
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("my_producer".to_string());

let producer = producer.with_config(config);
```

**Configuration Options:**
- `error_strategy` - How to handle errors (Stop, Skip, Retry, Custom)
- `name` - Component name for logging and metrics

## 📚 Usage Examples

### Creating a Producer

```rust
use streamweave::{Producer, Output, ProducerConfig};
use streamweave_error::ErrorStrategy;

// Create a producer with error handling
let producer = MyProducer::new()
    .with_config(
        ProducerConfig::default()
            .with_error_strategy(ErrorStrategy::Retry(3))
            .with_name("data_source".to_string())
    );
```

### Creating a Transformer

```rust
use streamweave::{Transformer, TransformerConfig};
use streamweave_error::ErrorStrategy;

// Create a transformer with error handling
let transformer = MyTransformer::new()
    .with_config(
        TransformerConfig::default()
            .with_error_strategy(ErrorStrategy::Skip)
            .with_name("data_processor".to_string())
    );
```

### Creating a Consumer

```rust
use streamweave::{Consumer, ConsumerConfig};
use streamweave_error::ErrorStrategy;

// Create a consumer with error handling
let consumer = MyConsumer::new()
    .with_config(
        ConsumerConfig {
            error_strategy: ErrorStrategy::Stop,
            name: "data_sink".to_string(),
        }
    );
```

### Error Handling Strategies

All components support multiple error handling strategies:

```rust
use streamweave_error::ErrorStrategy;

// Stop on first error (default)
ErrorStrategy::Stop

// Skip errors and continue processing
ErrorStrategy::Skip

// Retry up to N times
ErrorStrategy::Retry(3)

// Custom error handler
ErrorStrategy::new_custom(|error| {
    // Custom logic
    ErrorAction::Skip
})
```

## 🏗️ Architecture

The streamweave core package provides the foundational abstractions:

```text
┌─────────────┐
│   Producer  │───produces───> Stream<T>
└─────────────┘
       │
       │ Stream flows through
       ▼
┌─────────────┐
│ Transformer │───transforms───> Stream<U>
└─────────────┘
       │
       │ Stream flows through
       ▼
┌─────────────┐
│  Consumer   │───consumes───> (writes, stores, etc.)
└─────────────┘
```

All components:
- Implement `Input` and/or `Output` traits for type safety
- Support configuration for error handling and naming
- Integrate with the error handling system
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
- **Component Info**: Automatic component identification for error reporting

## ⚡ Performance Considerations

- **Zero-Cost Abstractions**: Traits compile to efficient code with no runtime overhead
- **Stream-Based**: All processing is stream-based for memory efficiency
- **Async**: Full async/await support for concurrent processing
- **Type Safety**: Compile-time type checking prevents runtime errors

## 📝 Examples

For more examples, see:
- [Pipeline Examples](https://github.com/Industrial/streamweave/tree/main/examples/basic_pipeline)
- [Graph Examples](https://github.com/Industrial/streamweave/tree/main/examples)
- [Package Implementations](../) - See specific packages for concrete implementations

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/streamweave)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave-error](../error/README.md) - Error handling system
- [streamweave-pipeline](../pipeline/README.md) - Pipeline builder and execution
- [streamweave-graph](../graph/README.md) - Graph API for complex topologies
- [streamweave-message](../message/README.md) - Message envelope and metadata

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

