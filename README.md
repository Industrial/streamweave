# StreamWeave

**Composable, async, stream-first computation in pure Rust**  
*Build fully composable, async data pipelines using a fluent API.*

StreamWeave is a general-purpose Rust framework built around the concept of
**streaming data**, with a focus on simplicity, composability, and performance.
It supports WASM targets for the browser or server and does not rely on
browser-specific stream primitives.

## ✨ Key Features

- Pure Rust API — zero-cost abstractions
- Full async/await compatibility via `futures::Stream`
- Fluent pipeline-style API
- Lightweight error handling & routing
- Optional conditional logic and fan-in/fan-out support
- Code-as-configuration — no external DSLs
- WASM compatible with minimal footprint

## 📦 Core Concepts

StreamWeave breaks computation into **three primary building blocks**:

| Component       | Description                                |
| --------------- | ------------------------------------------ |
| **Producer**    | Starts a stream of data                    |
| **Transformer** | Transforms stream items (e.g., map/filter) |
| **Consumer**    | Consumes the stream, e.g. writing, logging |

All components can be chained together fluently.

## 🔄 Example Pipeline

```rust
let pipeline = Pipeline::new()
  .producer(FileProducer::new("input.csv"))
  .transform(CsvTransformer::new())
  .transform(FilterTransformer::new(|row| row["active"] == "true"))
  .transform(MapTransformer::new(|row| row["email"].to_lowercase()))
  .consumer(FileConsumer::new("output.csv"));

pipeline.run().await?;
```

## 🧱 API Overview

### ✅ Pipeline Construction

```rust
Pipeline::new()
  .producer(...)
  .transform(...)
  .transform(...)
  .consumer(...)
  .run()
```

- Each method adds a stage to the pipeline.
- `.run()` executes the pipeline and awaits completion.

### 🔁 Transformers

Transformers support common stream combinators:

```rust
MapTransformer::new(|x| x * 2)
FilterTransformer::new(|x| x > 10)
FlatMapTransformer::new(|x| vec![x, x + 1])
```

They can be chained via `.transform(...)`.

### ⚠️ Error Handling

Each stream stage uses `Result<T, E>` as its item type.

#### Strategies:
- **Propagate errors** by default.
- **Suppress or redirect errors** with `.on_error(...)`.

```rust
MapTransformer::new(parse)
  .on_error(ErrorStrategy::Ignore)

MapTransformer::new(parse)
  .on_error(ErrorStrategy::ReplaceWith(Default::default()))

MapTransformer::new(parse)
  .on_error(ErrorStrategy::RedirectTo(error_sink()))
```

### 🤔 Conditional Logic

Pipeline branches can be conditionally included at runtime:

```rust
if config.enable_filter {
  graph = graph.transform(FilterTransformer::new(...));
}
```

Or using `.transform_when()`:

```rust
graph.transform_when(
  || config.enable_filter,
  FilterTransformer::new(...)
)
```

### 🔀 Fan-Out (Broadcast)

Duplicate a stream to multiple branches:

```rust
graph
  .producer(SensorProducer::new())
  .tee()
  .branch(|b| {
    b.transform(LogTransformer::new()).consumer(LogConsumer::new());
    b.transform(MetricsTransformer::new()).consumer(MetricsConsumer::new());
  });
```

### 🔁 Fan-In (Merge)

Combine multiple streams into one:

```rust
graph
  .merge(vec![stream1, stream2])
  .transform(DeduplicateTransformer::new())
  .consumer(OutputConsumer::new());
```

## 🛠️ Composable Subgraphs

You can define reusable sub-pipelines as functions:

```rust
fn clean_emails() -> impl Transformer<Row, String, Error> {
  FilterTransformer::new(|row| row["active"] == "true")
    .chain(MapTransformer::new(|row| row["email"].to_lowercase()))
}
```

## 🧪 Testing Pipelines

You can unit test sub-pipelines just like functions:

```rust
let test_pipeline = Pipeline::new()
  .producer(VecProducer::new(vec![1, 2, 3]))
  .transform(MapTransformer::new(|x| x * 10))
  .consumer(VecConsumer::new());

assert_eq!(test_pipeline.run().await?, vec![10, 20, 30]);
```

## 🌐 WASM Support

- StreamWeave compiles cleanly to WebAssembly (WASM).
- Use it for streaming pipelines in the browser or server.
- Works with `wasm-bindgen`, `wasm-pack`, or `wasmer`.

## 📚 Philosophy

StreamWeave is built on the belief that:

- **Streams are the natural shape of computation**
- **Rust's type system is the configuration language**
- **The best DSL is no DSL**

## 🧠 Contributions Welcome

StreamWeave is early-stage. Contributions, feedback, and experimentation are
very welcome.
