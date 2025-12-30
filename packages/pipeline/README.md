# streamweave-pipeline

[![Crates.io](https://img.shields.io/crates/v/streamweave-pipeline.svg)](https://crates.io/crates/streamweave-pipeline)
[![Documentation](https://docs.rs/streamweave-pipeline/badge.svg)](https://docs.rs/streamweave-pipeline)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Pipeline API for StreamWeave**  
*Type-safe linear pipeline builder with compile-time validation.*

The `streamweave-pipeline` package provides a fluent, type-safe API for building linear data processing pipelines. It uses a type-state builder pattern to ensure pipelines are only built when all required components (producer, transformer, consumer) are present, preventing runtime errors.

## ✨ Key Features

- **Type-State Builder**: Compile-time validation of pipeline completeness
- **Fluent API**: Chainable builder methods for intuitive pipeline construction
- **Error Handling**: Integrated error handling strategies
- **Multiple Transformers**: Chain multiple transformers in sequence
- **Async Execution**: Full async/await support

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-pipeline = "0.3.0"
```

## 🚀 Quick Start

### Simple Pipeline

```rust
use streamweave_pipeline::PipelineBuilder;
use streamweave_array::ArrayProducer;
use streamweave_vec::VecConsumer;
// Assume MapTransformer exists
use streamweave_transformers::MapTransformer;

let pipeline = PipelineBuilder::new()
    .producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
    .transformer(MapTransformer::new(|x| x * 2))
    .consumer(VecConsumer::new());

let (_, consumer) = pipeline.run().await?;
let results = consumer.into_inner();
assert_eq!(results, vec![2, 4, 6, 8, 10]);
```

## 📖 API Overview

### PipelineBuilder

The `PipelineBuilder` uses a type-state pattern to ensure pipelines are built correctly:

```rust
pub struct PipelineBuilder<State> {
    // Internal state
}
```

**Builder States:**
- `Empty` - No components added yet
- `HasProducer<P>` - Producer added
- `HasTransformer<P, T>` - Producer and transformer added
- `Complete<P, T, C>` - All components added (ready to build)

### Pipeline

A complete pipeline ready for execution:

```rust
pub struct Pipeline<P, T, C>
where
    P: Producer,
    T: Transformer,
    C: Consumer,
{
    // Internal components
}
```

**Key Methods:**
- `run()` - Execute the pipeline asynchronously
- `with_error_strategy(strategy)` - Set error handling strategy

## 📚 Usage Examples

### Basic Pipeline

```rust
use streamweave_pipeline::PipelineBuilder;
use streamweave_array::ArrayProducer;
use streamweave_vec::VecConsumer;
use streamweave_transformers::MapTransformer;

let pipeline = PipelineBuilder::new()
    .producer(ArrayProducer::new(vec![1, 2, 3]))
    .transformer(MapTransformer::new(|x| x * 2))
    .consumer(VecConsumer::new());

let (_, consumer) = pipeline.run().await?;
let results = consumer.into_inner();
```

### Multiple Transformers

Chain multiple transformers in sequence:

```rust
let pipeline = PipelineBuilder::new()
    .producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
    .transformer(MapTransformer::new(|x| x * 2))      // First transformer
    .transformer(MapTransformer::new(|x| x + 1))      // Second transformer
    .transformer(FilterTransformer::new(|x| x > 5))   // Third transformer
    .consumer(VecConsumer::new());

let (_, consumer) = pipeline.run().await?;
```

### Error Handling Strategies

Configure error handling for the entire pipeline:

```rust
use streamweave_error::ErrorStrategy;

// Stop on first error (default)
let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

// Skip errors and continue
let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

// Retry up to 3 times
let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Retry(3))
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);
```

### Pipeline Configuration

Set error strategy after building:

```rust
let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer)
    .with_error_strategy(ErrorStrategy::Skip);

let (_, consumer) = pipeline.run().await?;
```

### Type Safety

The builder ensures type safety at compile time:

```rust
// ✅ Valid: All components present
let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

// ❌ Compile error: Missing consumer
let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer);
    // .consumer(consumer); // Missing!

// ❌ Compile error: Missing transformer
let pipeline = PipelineBuilder::new()
    .producer(producer);
    // .transformer(transformer); // Missing!
    // .consumer(consumer);
```

### Error Handling in Pipelines

Handle errors during pipeline execution:

```rust
use streamweave_error::{ErrorStrategy, PipelineError};

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Stop)
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

match pipeline.run().await {
    Ok((_, consumer)) => {
        // Pipeline completed successfully
        let results = consumer.into_inner();
    }
    Err(PipelineError { .. }) => {
        // Error occurred, pipeline stopped
        eprintln!("Pipeline execution failed");
    }
}
```

### Custom Error Handling

Use custom error handlers:

```rust
use streamweave_error::{ErrorStrategy, ErrorAction, StreamError};

let custom_strategy = ErrorStrategy::new_custom(|error: &StreamError<()>| {
    if error.retries < 3 {
        ErrorAction::Retry
    } else {
        ErrorAction::Skip
    }
});

let pipeline = PipelineBuilder::new()
    .with_error_strategy(custom_strategy)
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);
```

## 🏗️ Architecture

Pipelines follow a linear flow:

```text
┌──────────┐
│ Producer │───produces───> Stream<T>
└──────────┘
       │
       ▼
┌─────────────┐
│Transformer1 │───transforms───> Stream<U>
└─────────────┘
       │
       ▼
┌─────────────┐
│Transformer2 │───transforms───> Stream<V>
└─────────────┘
       │
       ▼
┌──────────┐
│ Consumer │───consumes───> (writes, stores, etc.)
└──────────┘
```

**Pipeline Execution:**
1. Producer generates items
2. Items flow through transformers (in sequence)
3. Consumer processes final items
4. Error handling applied at each stage

## 🔧 Configuration

### Error Strategies

**Stop (Default):**
- Stops pipeline on first error
- Ensures data integrity
- Best for critical processing

**Skip:**
- Skips items that cause errors
- Continues processing remaining items
- Best for data cleaning

**Retry:**
- Retries failed operations
- Configurable retry count
- Best for transient failures

**Custom:**
- User-defined error handling
- Maximum flexibility
- Best for domain-specific logic

## 🔍 Error Handling

Pipeline errors are returned as `PipelineError<()>`:

```rust
pub enum PipelineError<T> {
    // Wraps StreamError with pipeline context
}
```

Error handling is applied:
- At the pipeline level (via `with_error_strategy`)
- At the component level (via component configs)
- Component-level configs override pipeline-level strategy

## ⚡ Performance Considerations

- **Zero-Cost Abstractions**: Type-state pattern compiles to efficient code
- **Stream-Based**: All processing is stream-based for memory efficiency
- **Async**: Full async/await support for concurrent processing
- **Type Safety**: Compile-time validation prevents runtime errors

## 📝 Examples

For more examples, see:
- [Basic Pipeline Example](https://github.com/Industrial/streamweave/tree/main/examples/basic_pipeline)
- [Advanced Pipeline Example](https://github.com/Industrial/streamweave/tree/main/examples/advanced_pipeline)
- [Error Handling Example](https://github.com/Industrial/streamweave/tree/main/examples/error_handling)

## 🔗 Dependencies

`streamweave-pipeline` depends on:

- `streamweave` - Core traits (Producer, Transformer, Consumer)
- `streamweave-error` - Error handling system
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Pipelines are used for:

1. **Linear Data Processing**: Process data through a sequence of transformations
2. **ETL Pipelines**: Extract, transform, and load data
3. **Data Validation**: Validate and clean data streams
4. **Simple Workflows**: Straightforward data processing workflows
5. **Type-Safe Processing**: Compile-time validation of pipeline structure

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-pipeline)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/pipeline)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-graph](../graph/README.md) - Graph API for complex topologies
- [streamweave-error](../error/README.md) - Error handling system

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

