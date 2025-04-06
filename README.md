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
- Basic error propagation
- Code-as-configuration — no external DSLs
- Comprehensive test infrastructure

### 🚧 Planned

- Lightweight error handling & routing strategies
- Optional conditional logic and fan-in/fan-out support
- WASM compatibility with minimal footprint
- Common transformers and utilities
- Reusable pipeline components

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
// Basic pipeline with custom components
let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transform(transformer)
    .consumer(consumer);

pipeline.run().await?;
```

### 🚧 Planned Features

```rust
let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("input.csv"))
    .transform(CsvTransformer::new())
    .transform(FilterTransformer::new(|row| row["active"] == "true"))
    .transform(MapTransformer::new(|row| row["email"].to_lowercase()))
    .consumer(FileConsumer::new("output.csv"));

pipeline.run().await?;
```

## 🧱 API Overview

### ✅ Implemented Pipeline Construction

```rust
PipelineBuilder::new()
    .producer(...)    // Add data source
    .transform(...)   // Add transformation
    .consumer(...)    // Add data sink
    .run()           // Execute pipeline
```

### 🚧 Planned Features

#### Error Handling

```rust
MapTransformer::new(parse)
    .on_error(ErrorStrategy::Ignore)

MapTransformer::new(parse)
    .on_error(ErrorStrategy::ReplaceWith(Default::default()))

MapTransformer::new(parse)
    .on_error(ErrorStrategy::RedirectTo(error_sink()))
```

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

### 🚧 Planned Testing Features

```rust
let test_pipeline = PipelineBuilder::new()
    .producer(VecProducer::new(vec![1, 2, 3]))
    .transform(MapTransformer::new(|x| x * 10))
    .consumer(VecConsumer::new());

assert_eq!(test_pipeline.run().await?, vec![10, 20, 30]);
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

StreamWeave is early-stage. Contributions, feedback, and experimentation are
very welcome. Current focus areas:

1. Implementing error handling strategies
2. Adding common transformers (Map, Filter, FlatMap)
3. Implementing fan-out/fan-in operations
4. Adding WASM examples and documentation
5. Creating common producers/consumers (File, Vec, etc.)
