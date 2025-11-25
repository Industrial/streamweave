# StreamWeave

[![Crates.io](https://img.shields.io/crates/v/streamweave.svg)](https://crates.io/crates/streamweave)
[![Documentation](https://docs.rs/streamweave/badge.svg)](https://docs.rs/streamweave)
[![CI](https://github.com/Industrial/streamweave/workflows/CI/badge.svg)](https://github.com/Industrial/streamweave/actions)
[![codecov](https://codecov.io/gh/Industrial/streamweave/graph/badge.svg)](https://codecov.io/gh/Industrial/streamweave)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Composable, async, stream-first computation in pure Rust**  
*Build fully composable, async data pipelines using a fluent API.*

StreamWeave is a general-purpose Rust framework built around the concept of
**streaming data**, with a focus on simplicity, composability, and performance.
It supports WASM targets for the browser or server and does not rely on
browser-specific stream primitives.

## ✨ Key Features

### ✅ Implemented

- Pure Rust API with zero-cost abstractions
- Full async/await compatibility via `futures::Stream`
- Fluent pipeline-style API with type-safe builder pattern
- Comprehensive error handling system with multiple strategies
- Code-as-configuration — no external DSLs
- Comprehensive test infrastructure
- File-based producers and consumers
- Common transformers (Map, Batch, RateLimit, CircuitBreaker, Retry, Filter)
- HTTP middleware support with Axum integration
- WebSocket support
- Server-Sent Events support

### 🚧 Planned

- Add stateful processing
- Implement exactly-once processing
- Support distributed processing
- Implement windowing operations
- Support more data formats
- Add more specialized transformers
- Fan-in/fan-out support
- WASM-specific optimizations and documentation
- Additional specialized transformers and utilities
- Reusable pipeline components
- More specialized producers and consumers
- Add machine learning integration
- Implement monitoring and metrics
- Add SQL-like querying
- Add visualization tools

## 📦 Core Concepts

StreamWeave breaks computation into **three primary building blocks**:

| Component       | Description                                |
| --------------- | ------------------------------------------ |
| **Producer**    | Starts a stream of data                    |
| **Transformer** | Transforms stream items (e.g., map/filter) |
| **Consumer**    | Consumes the stream, e.g. writing, logging |

All components can be chained together fluently.

## 🔄 Example Pipeline

### ✅ Currently Possible

```rust
use streamweave::{
    consumers::console::console_consumer::ConsoleConsumer,
    pipeline::PipelineBuilder,
    producers::range::range_producer::RangeProducer,
    transformers::map::map_transformer::MapTransformer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a pipeline that:
    // 1. Produces numbers from 1 to 5
    // 2. Doubles each number
    // 3. Prints the result to the console
    let pipeline = PipelineBuilder::new()
        .producer(RangeProducer::new(1, 6, 1))
        .transformer(MapTransformer::new(|x: i32| x * 2))
        .consumer(ConsoleConsumer::new());

    // Run the pipeline
    pipeline.run().await?;
    Ok(())
}
```

## 🧱 API Overview

### ✅ Implemented Pipeline Construction

```rust
PipelineBuilder::new()
    .producer(...)    // Add data source
    .transformer(...) // Add transformation
    .consumer(...)    // Add data sink
    .run()           // Execute pipeline
```

### ✅ Error Handling

StreamWeave provides two levels of error handling:

1. **Pipeline Level**
```rust
// Default behavior: Pipeline stops on first error
pipeline.run().await?;

// Configure pipeline-wide error handling
pipeline
    .with_error_strategy(ErrorStrategy::Stop)  // Default
    .with_error_strategy(ErrorStrategy::Skip)  // Skip errored items
    .with_error_strategy(ErrorStrategy::Retry(3))  // Retry 3 times
    .run()
    .await?;
```

2. **Component Level**
```rust
// Override error handling for specific components
MapTransformer::new(parse)
    .with_error_strategy(ErrorStrategy::Stop)      // Stop component and pipeline
    .with_error_strategy(ErrorStrategy::Skip)      // Skip errors, continue processing
    .with_error_strategy(ErrorStrategy::Retry(3))  // Retry operation 3 times
    .with_error_strategy(ErrorStrategy::Custom(|err| {
        // Custom error handling logic
        ErrorAction::Skip
    }));
```

## 🚀 Getting Started

### Installation

Add StreamWeave to your `Cargo.toml`:

```toml
[dependencies]
streamweave = "0.1.0"
```

### Basic Usage

```rust
use streamweave::{
    consumers::vec::vec_consumer::VecConsumer,
    pipeline::PipelineBuilder,
    producers::array::array_producer::ArrayProducer,
    transformers::map::map_transformer::MapTransformer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let producer = ArrayProducer::new(vec![1, 2, 3, 4, 5]);
    let transformer = MapTransformer::new(|x: i32| x * 2);
    let consumer = VecConsumer::new();

    let pipeline = PipelineBuilder::new()
        .producer(producer)
        .transformer(transformer)
        .consumer(consumer);

    let ((), result) = pipeline.run().await?;
    println!("Result: {:?}", result.collected);
    Ok(())
}
```

## 🧪 Testing Pipelines

### ✅ Implemented

The framework includes comprehensive test infrastructure for unit testing pipelines and components:

```rust
// Example from the test suite
let producer = NumberProducer { numbers: vec![1, 2, 3] };
let transformer = StringifyTransformer;
let consumer = CollectConsumer { collected: Vec::new() };

let (_, consumer) = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer)
    .run()
    .await
    .unwrap();

assert_eq!(consumer.collected, vec!["1", "2", "3"]);
```

## 📚 Documentation

- [API Documentation](https://docs.rs/streamweave)
- [Examples](https://github.com/yourusername/streamweave/tree/main/examples)
- [Contributing Guide](https://github.com/yourusername/streamweave/blob/main/CONTRIBUTING.md)

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/).

See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- HTTP support powered by [Axum](https://github.com/tokio-rs/axum)
- Inspired by reactive programming patterns and stream processing frameworks