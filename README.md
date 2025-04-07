# StreamWeave

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
- Common transformers (Map, Batch, RateLimit, CircuitBreaker, Retry)

### 🚧 Planned

- Conditional logic and fan-in/fan-out support
- WASM-specific optimizations and documentation
- Additional specialized transformers and utilities
- Reusable pipeline components
- More specialized producers and consumers

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
// Advanced pipeline with error handling and multiple transformers
let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("input.txt"))
    .transformer(MapTransformer::new(|s: String| {
        s.parse::<i32>().unwrap_or_default()
    }))
    .transformer(BatchTransformer::new(3).unwrap())
    .transformer(RateLimitTransformer::new(1, Duration::from_secs(1)))
    .transformer(CircuitBreakerTransformer::new(3, Duration::from_secs(5)))
    .transformer(RetryTransformer::new(3, Duration::from_millis(100)))
    .consumer(FileConsumer::new("output.txt"))
    .run()
    .await?;
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

**Default Behaviors:**
- Pipeline Level: `ErrorStrategy::Stop` - Pipeline stops on first error
- Component Level: Inherits pipeline strategy unless overridden

**Error Actions:**
- `Stop`: Stop processing and propagate error (default)
- `Skip`: Skip errored item and continue
- `Retry(n)`: Retry operation n times before applying next strategy
- `Custom(handler)`: Custom error handling logic

**Error Context:**
```rust
struct StreamError<E> {
    source: E,                    // Original error
    context: ErrorContext,        // Error metadata
    retries: usize,              // Retry attempts if any
    component: ComponentInfo,     // Component where error occurred
}

struct ErrorContext {
    timestamp: DateTime<Utc>,
    item: Option<Box<dyn Any>>,  // Item being processed
    stage: PipelineStage,        // Stage where error occurred
}
```

### 🚧 Planned Features

#### Conditional Logic

```rust
if config.enable_filter {
    graph = graph.transform(FilterTransformer::new(...));
}

// Or using planned .transform_when():
graph.transform_when(
    || config.enable_filter,
    FilterTransformer::new(...)
)
```

#### Fan-Out (Broadcast)

```rust
graph
    .producer(SensorProducer::new())
    .tee()
    .branch(|b| {
        b.transform(LogTransformer::new()).consumer(LogConsumer::new());
        b.transform(MetricsTransformer::new()).consumer(MetricsConsumer::new());
    });
```

#### Fan-In (Merge)

```rust
graph
    .merge(vec![stream1, stream2])
    .transform(DeduplicateTransformer::new())
    .consumer(OutputConsumer::new());
```

#### Composable Subgraphs

```rust
fn clean_emails() -> impl Transformer<Row, String, Error> {
    FilterTransformer::new(|row| row["active"] == "true")
        .chain(MapTransformer::new(|row| row["email"].to_lowercase()))
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

## 🌐 WASM Support

🚧 **Planned**: StreamWeave is designed to compile cleanly to WebAssembly (WASM).
- Use it for streaming pipelines in the browser or server
- Works with `wasm-bindgen`, `wasm-pack`, or `wasmer`

## 📚 Philosophy

StreamWeave is built on the belief that:

- **Streams are the natural shape of computation**
- **Rust's type system is the configuration language**
- **The best DSL is no DSL**

## 🧠 Contributions Welcome

StreamWeave is actively developed. Contributions, feedback, and experimentation are
very welcome. Current focus areas:

1. Implementing fan-out/fan-in operations
2. Adding conditional logic support
3. Creating reusable pipeline components
4. Adding WASM examples and documentation
5. Implementing additional specialized transformers and consumers
