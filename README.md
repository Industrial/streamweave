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
- Comprehensive error handling system with multiple strategies (Stop, Skip, Retry, Custom)
- Code-as-configuration — no external DSLs
- Comprehensive test infrastructure
- **Integration Examples**: Kafka, Redis Streams, Database (PostgreSQL/MySQL/SQLite), HTTP Polling
- **File Format Support**: CSV, JSONL, Parquet with streaming parsing
- **Stateful Processing**: RunningSum, MovingAverage transformers
- **Exactly-Once Processing**: Message deduplication with configurable windows
- **Windowing Operations**: Tumbling, sliding, and count-based windows
- **Advanced Transformers**: CircuitBreaker, Retry, Batch, RateLimit
- **Common Transformers**: Map, Filter, Flatten, Reduce, and many more
- HTTP middleware support with Axum integration
- WebSocket support
- Server-Sent Events support

### 🚧 Planned

- Support distributed processing
- Fan-in/fan-out support
- WASM-specific optimizations and documentation
- Additional specialized transformers and utilities
- Reusable pipeline components
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
streamweave = "0.2.1"
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
- [Local Documentation](target/doc/streamweave/index.html) - Generated with Doxidize (run `./bin/docs`)
- [Getting Started Guide](docs/getting_started.md)
- [Architecture Overview](docs/architecture.md)
- [Common Use Cases](docs/guides/common_use_cases.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Contributing Guide](https://github.com/yourusername/streamweave/blob/main/CONTRIBUTING.md)

## 📖 Examples

StreamWeave includes comprehensive examples demonstrating all major features:

### Integration Examples
- **[Kafka Integration](examples/kafka_integration/)** - Produce to and consume from Kafka topics
- **[Redis Streams Integration](examples/redis_streams_integration/)** - XADD and XREAD operations with consumer groups
- **[Database Integration](examples/database_integration/)** - Query PostgreSQL, MySQL, and SQLite with streaming results
- **[HTTP Polling Integration](examples/http_poll_integration/)** - Poll HTTP endpoints with pagination, delta detection, and rate limiting

### File Format Examples
- **[File Formats](examples/file_formats/)** - CSV, JSONL, and Parquet read/write with streaming parsing

### Processing Examples
- **[Stateful Processing](examples/stateful_processing/)** - RunningSum and MovingAverage transformers
- **[Error Handling](examples/error_handling/)** - Stop, Skip, Retry, and Custom error strategies
- **[Advanced Transformers](examples/advanced_transformers/)** - CircuitBreaker, Retry, Batch, RateLimit
- **[Windowing Operations](examples/windowing/)** - Tumbling, sliding, and count-based windows
- **[Exactly-Once Processing](examples/exactly_once/)** - Message deduplication and checkpointing

### Basic Examples
- **[Basic Pipeline](examples/basic_pipeline/)** - Simple pipeline example
- **[Advanced Pipeline](examples/advanced_pipeline/)** - Complex pipeline patterns

Run any example with:
```bash
cargo run --example <example_name> --features <required_features>
```

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/).

See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- HTTP support powered by [Axum](https://github.com/tokio-rs/axum)
- Inspired by reactive programming patterns and stream processing frameworks