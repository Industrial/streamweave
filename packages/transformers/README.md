# streamweave-transformers

[![Crates.io](https://img.shields.io/crates/v/streamweave-transformers.svg)](https://crates.io/crates/streamweave-transformers)
[![Documentation](https://docs.rs/streamweave-transformers/badge.svg)](https://docs.rs/streamweave-transformers)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Transformers for StreamWeave**  
*A comprehensive collection of transformers for building powerful streaming data pipelines.*

The `streamweave-transformers` package provides a rich set of transformers for StreamWeave pipelines and graphs. Transformers are organized into categories: basic operations, advanced processing, stateful operations, routing, merging, machine learning, and utility functions.

## ✨ Key Features

- **30+ Transformers**: Comprehensive collection of transformers
- **Categorized**: Organized into logical categories
- **Type-Safe**: Full type safety with Rust's type system
- **Composable**: Transformers can be chained together
- **Error Handling**: Built-in error handling strategies
- **ML Support**: Optional machine learning transformers

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-transformers = "0.6.0"

# ML transformers are in a separate package
streamweave-ml = { path = "../packages/ml", features = ["onnx"] }
```

## 🚀 Quick Start

### Basic Transformation

```rust
use streamweave_transformers::MapTransformer;
use streamweave_pipeline::PipelineBuilder;

let transformer = MapTransformer::new(|x: i32| x * 2);

let pipeline = PipelineBuilder::new()
    .producer(/* produce numbers */)
    .transformer(transformer)
    .consumer(/* consume doubled numbers */);

pipeline.run().await?;
```

### Filtering

```rust
use streamweave_transformers::FilterTransformer;
use streamweave_pipeline::PipelineBuilder;

let transformer = FilterTransformer::new(|x: i32| x > 10);

let pipeline = PipelineBuilder::new()
    .producer(/* produce numbers */)
    .transformer(transformer)
    .consumer(/* consume filtered numbers */);

pipeline.run().await?;
```

## 📖 Transformer Categories

### Basic Transformers

Fundamental stream operations:

- **MapTransformer**: Transform each item with a function
- **FilterTransformer**: Filter items based on a predicate
- **ReduceTransformer**: Reduce stream to accumulated value

### Advanced Transformers

Resilience and performance:

- **BatchTransformer**: Batch items for batch processing
- **RetryTransformer**: Retry failed operations
- **CircuitBreakerTransformer**: Circuit breaker pattern
- **RateLimitTransformer**: Rate limiting

### Stateful Transformers

Stateful operations:

- **RunningSumTransformer**: Running sum calculation
- **MovingAverageTransformer**: Moving average calculation

### Routing Transformers

Route items to different paths:

- **RouterTransformer**: Route based on conditions
- **PartitionTransformer**: Partition by key
- **RoundRobinTransformer**: Round-robin distribution

### Merging Transformers

Combine multiple streams:

- **MergeTransformer**: Merge multiple streams
- **OrderedMergeTransformer**: Merge maintaining order
- **InterleaveTransformer**: Interleave streams

### Machine Learning Transformers

ML inference (requires `ml` feature):

- **InferenceTransformer**: Single-item inference
- **BatchedInferenceTransformer**: Batch inference

### Utility Transformers

Common utility operations:

- **SampleTransformer**: Sample items randomly
- **SkipTransformer**: Skip N items
- **TakeTransformer**: Take N items
- **LimitTransformer**: Limit stream size
- **SortTransformer**: Sort items
- **SplitTransformer**: Split items
- **SplitAtTransformer**: Split at index
- **ZipTransformer**: Zip multiple streams
- **TimeoutTransformer**: Timeout operations
- **MessageDedupeTransformer**: Deduplicate messages
- **GroupByTransformer**: Group items by key

## 📚 Usage Examples

### Basic Transformers

#### Map Transformer

Transform each item:

```rust
use streamweave_transformers::MapTransformer;

let transformer = MapTransformer::new(|x: i32| x * 2);
```

#### Filter Transformer

Filter items:

```rust
use streamweave_transformers::FilterTransformer;

let transformer = FilterTransformer::new(|x: i32| x > 0);
```

#### Reduce Transformer

Reduce stream:

```rust
use streamweave_transformers::ReduceTransformer;

let transformer = ReduceTransformer::new(0, |acc: i32, x: i32| acc + x);
```

### Advanced Transformers

#### Batch Transformer

Batch items:

```rust
use streamweave_transformers::BatchTransformer;
use std::time::Duration;

let transformer = BatchTransformer::new(100, Duration::from_secs(1));
```

#### Retry Transformer

Retry failed operations:

```rust
use streamweave_transformers::RetryTransformer;

let transformer = RetryTransformer::new(3, Duration::from_secs(1));
```

#### Circuit Breaker Transformer

Circuit breaker pattern:

```rust
use streamweave_transformers::CircuitBreakerTransformer;

let transformer = CircuitBreakerTransformer::new(5, Duration::from_secs(10));
```

#### Rate Limit Transformer

Rate limiting:

```rust
use streamweave_transformers::RateLimitTransformer;
use std::time::Duration;

let transformer = RateLimitTransformer::new(100, Duration::from_secs(1));
```

### Stateful Transformers

#### Running Sum Transformer

Calculate running sum:

```rust
use streamweave_transformers::RunningSumTransformer;

let transformer = RunningSumTransformer::new(0);
```

#### Moving Average Transformer

Calculate moving average:

```rust
use streamweave_transformers::MovingAverageTransformer;

let transformer = MovingAverageTransformer::new(10);  // Window size 10
```

### Routing Transformers

#### Router Transformer

Route based on conditions:

```rust
use streamweave_transformers::RouterTransformer;

let transformer = RouterTransformer::new(|x: i32| {
    if x > 0 { "positive" } else { "negative" }
});
```

#### Partition Transformer

Partition by key:

```rust
use streamweave_transformers::PartitionTransformer;

let transformer = PartitionTransformer::new(|x: i32| x % 2);
```

#### Round Robin Transformer

Round-robin distribution:

```rust
use streamweave_transformers::RoundRobinTransformer;

let transformer = RoundRobinTransformer::new(3);  // 3 outputs
```

### Merging Transformers

#### Merge Transformer

Merge streams:

```rust
use streamweave_transformers::MergeTransformer;

let transformer = MergeTransformer::new();
```

#### Ordered Merge Transformer

Merge maintaining order:

```rust
use streamweave_transformers::OrderedMergeTransformer;

let transformer = OrderedMergeTransformer::new(|x: &i32, y: &i32| x.cmp(y));
```

#### Interleave Transformer

Interleave streams:

```rust
use streamweave_transformers::InterleaveTransformer;

let transformer = InterleaveTransformer::new();
```

### Machine Learning Transformers

#### ML Transformers

ML transformers have been moved to a separate package: `streamweave-ml`.

See the [ML Transformers package](../ml/README.md) for documentation.

### Utility Transformers

#### Sample Transformer

Random sampling:

```rust
use streamweave_transformers::SampleTransformer;

let transformer = SampleTransformer::new(0.1);  // 10% sample rate
```

#### Skip Transformer

Skip items:

```rust
use streamweave_transformers::SkipTransformer;

let transformer = SkipTransformer::new(10);  // Skip first 10 items
```

#### Take Transformer

Take items:

```rust
use streamweave_transformers::TakeTransformer;

let transformer = TakeTransformer::new(100);  // Take first 100 items
```

#### Limit Transformer

Limit stream:

```rust
use streamweave_transformers::LimitTransformer;

let transformer = LimitTransformer::new(1000);  // Limit to 1000 items
```

#### Sort Transformer

Sort items:

```rust
use streamweave_transformers::SortTransformer;

let transformer = SortTransformer::new(|x: &i32, y: &i32| x.cmp(y));
```

#### Split Transformer

Split items:

```rust
use streamweave_transformers::SplitTransformer;

let transformer = SplitTransformer::new(|x: &String| x.split_whitespace());
```

#### Split At Transformer

Split at index:

```rust
use streamweave_transformers::SplitAtTransformer;

let transformer = SplitAtTransformer::new(100);  // Split at index 100
```

#### Zip Transformer

Zip streams:

```rust
use streamweave_transformers::ZipTransformer;

let transformer = ZipTransformer::new();
```

#### Timeout Transformer

Timeout operations:

```rust
use streamweave_transformers::TimeoutTransformer;
use std::time::Duration;

let transformer = TimeoutTransformer::new(Duration::from_secs(5));
```

#### Message Dedupe Transformer

Deduplicate messages:

```rust
use streamweave_transformers::MessageDedupeTransformer;

let transformer = MessageDedupeTransformer::new();
```

#### Group By Transformer

Group by key:

```rust
use streamweave_transformers::GroupByTransformer;

let transformer = GroupByTransformer::new(|x: &i32| x % 10);
```

## 🏗️ Architecture

Transformer flow:

```text
Stream<T> ──> Transformer ──> Stream<U>
```

**Transformer Flow:**
1. Input stream flows into transformer
2. Transformer processes items
3. Output stream flows out
4. Transformers can be chained

## 🔧 Configuration

All transformers support:

- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Transformers support error handling strategies:

```rust
use streamweave_error::ErrorStrategy;

let transformer = MapTransformer::new(|x: i32| x * 2)
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Batching**: Use batch transformers for better throughput
- **Stateful**: Stateful transformers maintain state
- **ML**: ML transformers require model loading
- **Routing**: Routing transformers distribute load

## 📝 Examples

For more examples, see:
- [Transformer Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-transformers` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` - Message envelopes
- `streamweave-stateful` - Stateful operations
- `tokio` - Async runtime
- `futures` - Stream utilities
- `ort` (optional) - ONNX Runtime for ML
- `ndarray` (optional) - Array operations for ML

## 🎯 Use Cases

Transformers are used for:

1. **Data Transformation**: Transform data in pipelines
2. **Filtering**: Filter data based on conditions
3. **Aggregation**: Aggregate data (sum, average, etc.)
4. **Routing**: Route data to different paths
5. **Merging**: Merge multiple data streams
6. **ML Inference**: Run ML models on streams
7. **Utility Operations**: Common utility operations

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-transformers)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/transformers)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-stateful](../stateful/README.md) - Stateful operations
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

