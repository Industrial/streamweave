# StreamWeave Architecture

## Overview

StreamWeave is built around the concept of composable data pipelines. The architecture is designed to be:
- **Type-safe**: Compile-time guarantees about data flow
- **Async-first**: Built on `futures::Stream` for efficient async processing
- **Composable**: Components can be easily combined
- **Zero-cost**: Minimal runtime overhead

## Core Components

### Pipeline Builder Pattern

The pipeline uses a state machine pattern with compile-time validation:

```rust
Pipeline<Empty>           // Empty pipeline
  -> Pipeline<HasProducer>  // Producer added
  -> Pipeline<HasTransformer>  // Transformer added
  -> Pipeline<Complete>  // Ready to run
```

This ensures that pipelines are only run when they have all required components.

### Error Handling

StreamWeave provides comprehensive error handling:

- **Error Strategies**: Stop, Skip, Retry, Custom
- **Error Context**: Rich error information with context
- **Component-level**: Override pipeline strategy per component

### Feature Flags

The library uses feature flags to enable optional functionality:

- `native`: Full native platform features
- `wasm`: WebAssembly compatibility
- `kafka`: Kafka integration
- `redis-streams`: Redis Streams integration
- `database`: Database query support
- `http-poll`: HTTP polling support
- `http-server`: HTTP server support
- `file-formats`: CSV, JSONL, Parquet, MsgPack support

## Module Structure

```
src/
├── lib.rs              # Main library entry point
├── pipeline.rs         # Pipeline builder and execution
├── producer.rs         # Producer trait
├── transformer.rs      # Transformer trait
├── consumer.rs         # Consumer trait
├── error.rs            # Error handling
├── message.rs          # Message types
├── producers/          # Producer implementations
├── transformers/       # Transformer implementations
└── consumers/          # Consumer implementations
```

## Data Flow

```
Producer → Transformer → Transformer → ... → Consumer
   ↓           ↓              ↓                    ↓
 Stream    Stream         Stream              Result
```

Each component processes items asynchronously, allowing for efficient streaming of large datasets.

## Extension Points

### Custom Producers

Implement the `Producer` trait:

```rust
pub trait Producer: Send + Sync {
    type Output: Send + Sync;
    fn produce(&self) -> impl Stream<Item = Result<Self::Output, StreamError>>;
}
```

### Custom Transformers

Implement the `Transformer` trait:

```rust
pub trait Transformer: Send + Sync {
    type Input: Send + Sync;
    type Output: Send + Sync;
    fn transform(&self, input: Self::Input) -> impl Future<Output = Result<Self::Output, StreamError>>;
}
```

### Custom Consumers

Implement the `Consumer` trait:

```rust
pub trait Consumer: Send + Sync {
    type Input: Send + Sync;
    type Output: Send + Sync;
    fn consume(&self, input: impl Stream<Item = Result<Self::Input, StreamError>>) -> impl Future<Output = Result<Self::Output, StreamError>>;
}
```

