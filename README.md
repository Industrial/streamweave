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

**High-Performance Streaming:** Process **2-6 million messages per second** with in-process zero-copy execution. Perfect for high-throughput data processing pipelines.

## ✨ Key Features

- Pure Rust API with zero-cost abstractions
- Full async/await compatibility via `futures::Stream`
- Fluent pipeline-style API with type-safe builder pattern
- **Graph-based API** for complex topologies with fan-in/fan-out patterns
- **Flow-Based Programming (FBP)** patterns with type-safe routing
- Comprehensive error handling system with multiple strategies
- Code-as-configuration — no external DSLs
- Extensive package ecosystem for I/O, transformations, and integrations

## 📦 Core Concepts

StreamWeave breaks computation into **three primary building blocks**:

| Component       | Description                                |
| --------------- | ------------------------------------------ |
| **Producer**    | Starts a stream of data                    |
| **Transformer** | Transforms stream items (e.g., map/filter) |
| **Consumer**    | Consumes the stream, e.g. writing, logging |

All components can be chained together fluently. These components can be used in both the **Pipeline API** (for simple linear flows) and the **Graph API** (for complex topologies with fan-in/fan-out patterns).

## 🔀 Pipeline vs Graph API

StreamWeave provides two APIs for building data processing workflows:

| Feature | Pipeline API | Graph API |
|---------|-------------|-----------|
| **Use Case** | Simple linear flows | Complex topologies |
| **Topology** | Single path: Producer → Transformer → Consumer | Multiple paths, fan-in/fan-out |
| **Routing** | Sequential processing | Configurable routing strategies |
| **Complexity** | Lower complexity, easier to use | Higher flexibility, more powerful |
| **Best For** | ETL pipelines, simple transformations | Complex workflows, parallel processing, data distribution |

## 🚀 Quick Start

### Installation

Add StreamWeave to your `Cargo.toml`:

```toml
[dependencies]
streamweave = "0.7.0"
```

### Basic Example

```rust
use streamweave::PipelineBuilder;
use streamweave_array::ArrayProducer;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = PipelineBuilder::new()
        .producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
        .transformer(MapTransformer::new(|x: i32| x * 2))
        .consumer(VecConsumer::new());

    let ((), result) = pipeline.run().await?;
    println!("Result: {:?}", result.collected);
    Ok(())
}
```

For more examples and detailed documentation, see the [package documentation](#-packages) below.

## 📦 Packages

StreamWeave is organized as a monorepo with 39 packages, each providing specific functionality. Each package has its own README with detailed documentation, examples, and API reference.

### Core Foundation Packages

These are the foundational packages that other packages depend on:

- **[streamweave](packages/streamweave/README.md)** - Core traits and types (Producer, Transformer, Consumer)
- **[error](packages/error/README.md)** - Error handling system with multiple strategies
- **[message](packages/message/README.md)** - Message envelope and metadata
- **[offset](packages/offset/README.md)** - Offset management for exactly-once processing
- **[transaction](packages/transaction/README.md)** - Transaction support and boundaries

### System Packages

Core system functionality:

- **[pipeline](packages/pipeline/README.md)** - Pipeline builder and execution
- **[graph](packages/graph/README.md)** - Graph API for complex topologies
- **[stateful](packages/stateful/README.md)** - Stateful processing and state management
- **[window](packages/window/README.md)** - Windowing operations (tumbling, sliding, session)

### I/O Packages

Standard I/O and file system operations:

- **[stdio](packages/stdio/README.md)** - Standard input/output streaming
- **[file](packages/file/README.md)** - File I/O operations
- **[fs](packages/fs/README.md)** - File system operations and directory traversal
- **[tempfile](packages/tempfile/README.md)** - Temporary file handling
- **[path](packages/path/README.md)** - Path manipulation and transformations

### Data Format Packages

Data format parsing and serialization:

- **[csv](packages/csv/README.md)** - CSV parsing and writing
- **[jsonl](packages/jsonl/README.md)** - JSON Lines format support
- **[parquet](packages/parquet/README.md)** - Parquet format support

### Database Packages

Database integration:

- **[database](packages/database/README.md)** - Generic database support
- **[database-mysql](packages/database-mysql/README.md)** - MySQL integration
- **[database-postgresql](packages/database-postgresql/README.md)** - PostgreSQL integration
- **[database-sqlite](packages/database-sqlite/README.md)** - SQLite integration

### Network Packages

Network protocol integration:

- **[kafka](packages/kafka/README.md)** - Apache Kafka producer and consumer
- **[redis](packages/redis/README.md)** - Redis Streams integration
- **[http-server](packages/http-server/README.md)** - HTTP graph server with Axum integration

### Producer/Consumer Packages

Various data source and sink implementations:

- **[array](packages/array/README.md)** - Array-based streaming
- **[vec](packages/vec/README.md)** - Vector-based streaming
- **[env](packages/env/README.md)** - Environment variable streaming
- **[command](packages/command/README.md)** - Command execution and output streaming
- **[process](packages/process/README.md)** - Process management and monitoring
- **[signal](packages/signal/README.md)** - Unix signal handling
- **[timer](packages/timer/README.md)** - Time-based and interval-based streaming
- **[tokio](packages/tokio/README.md)** - Tokio channel integration

### Transformers Package

Comprehensive transformer implementations:

- **[transformers](packages/transformers/README.md)** - All transformer types including:
  - Basic: Map, Filter, Reduce
  - Advanced: Batch, Retry, CircuitBreaker, RateLimit
  - Stateful: RunningSum, MovingAverage
  - Routing: Router, Partition, RoundRobin
  - Merging: Merge, OrderedMerge, Interleave
  - ML: Inference, BatchedInference
  - Utility: Sample, Skip, Take, Limit, Sort, Split, Zip, Timeout, MessageDedupe

### Integration and Utility Packages

Observability and integration capabilities:

- **[integrations/opentelemetry](packages/integrations/opentelemetry/README.md)** - OpenTelemetry integration
- **[integrations/sql](packages/integrations/sql/README.md)** - SQL query support
- **[metrics](packages/metrics/README.md)** - Metrics collection and Prometheus integration
- **[visualization](packages/visualization/README.md)** - Pipeline and graph visualization

## 📚 Documentation

- [API Documentation](https://docs.rs/streamweave) - Full API reference on docs.rs
- [Local Documentation](target/doc/streamweave/index.html) - Generated with rustdoc (run `./bin/docs`)
- [Graph API Guide](GRAPH.md) - Advanced graph patterns, routing strategies, and Flow-Based Programming
- [Getting Started Guide](docs/getting_started.md)
- [Architecture Overview](docs/architecture.md)
- [Common Use Cases](docs/guides/common_use_cases.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Contributing Guide](CONTRIBUTING.md)

## 📖 Examples

StreamWeave includes comprehensive examples demonstrating all major features. See the [examples directory](examples/) for:

- Integration examples (Kafka, Redis, Database, HTTP)
- File format examples (CSV, JSONL, Parquet)
- Processing examples (Stateful, Error Handling, Windowing)
- Visualization examples
- Graph API examples

Run any example with:
```bash
cargo run --example <example_name> --features <required_features>
```

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/).

See \[LICENSE\](LICENSE) for details.

## 🙏 Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- HTTP support powered by [Axum](https://github.com/tokio-rs/axum)
- Inspired by reactive programming patterns and stream processing frameworks
