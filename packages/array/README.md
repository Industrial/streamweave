# streamweave-array

[![Crates.io](https://img.shields.io/crates/v/streamweave-array.svg)](https://crates.io/crates/streamweave-array)
[![Documentation](https://docs.rs/streamweave-array/badge.svg)](https://docs.rs/streamweave-array)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Array producer and consumer for StreamWeave**  
*Produce from and consume to fixed-size arrays with compile-time size guarantees.*

The `streamweave-array` package provides array-based producers and consumers for StreamWeave. It enables reading from fixed-size arrays and writing to fixed-size arrays with compile-time size guarantees.

## ✨ Key Features

- **ArrayProducer**: Produce items from fixed-size arrays
- **ArrayConsumer**: Consume items into fixed-size arrays
- **Compile-Time Size**: Array size determined at compile time
- **Type Safety**: Full type safety with const generics
- **Zero-Copy**: Efficient array iteration

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-array = "0.3.0"
```

## 🚀 Quick Start

### Produce from Array

```rust
use streamweave_array::ArrayProducer;
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;
use streamweave_vec::VecConsumer;

let array = [1, 2, 3, 4, 5];
let producer = ArrayProducer::new(array);
let transformer = MapTransformer::new(|x: i32| x); // Identity transformation
let consumer = VecConsumer::<i32>::new();

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

let ((), consumer) = pipeline.run().await?;
let collected = consumer.into_vec();
```

### Consume to Array

```rust
use streamweave_array::{ArrayConsumer, ArrayProducer};
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;

let consumer = ArrayConsumer::<i32, 5>::new();
let array = [1, 2, 3, 4, 5];
let producer = ArrayProducer::new(array);
let transformer = MapTransformer::new(|x: i32| x); // Identity transformation

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

let ((), consumer) = pipeline.run().await?;

let array = consumer.into_array();
```

## 📖 API Overview

### ArrayProducer

Produces items from a fixed-size array:

```rust
pub struct ArrayProducer<T, const N: usize> {
    pub array: [T; N],
    pub config: ProducerConfig<T>,
}
```

**Key Methods:**
- `new(array)` - Create producer from array
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream from array

### ArrayConsumer

Consumes items into a fixed-size array:

```rust
pub struct ArrayConsumer<T, const N: usize> {
    pub array: [Option<T>; N],
    pub index: usize,
    pub config: ConsumerConfig<T>,
}
```

**Key Methods:**
- `new()` - Create consumer
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Collect items into array
- `into_array()` - Get collected array

## 📚 Usage Examples

### Array Transformation Pipeline

Transform array elements:

```rust
use streamweave_array::{ArrayProducer, ArrayConsumer};
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;

let input = [1, 2, 3, 4, 5];
let producer = ArrayProducer::new(input);
let consumer = ArrayConsumer::<i32, 5>::new();
let transformer = MapTransformer::new(|x: i32| x * 2); // Double each element

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .await
    .consumer(consumer);

let ((), consumer) = pipeline.run().await?;
let array = consumer.into_array();
```

### Error Handling

Configure error handling:

```rust
use streamweave_array::ArrayProducer;
use streamweave_error::ErrorStrategy;

let array = [1, 2, 3];
let producer = ArrayProducer::new(array)
    .with_error_strategy(ErrorStrategy::Skip)
    .with_name("my_array_producer");
```

## 🏗️ Architecture

Array processing flow:

```text
Array[T; N] ──> ArrayProducer ──> Stream<T> ──> Transformer ──> Stream<T> ──> ArrayConsumer ──> [Option<T>; N]
```

**Array Flow:**
1. ArrayProducer iterates over array elements
2. Items flow through transformers
3. ArrayConsumer collects items into array
4. Array size is guaranteed at compile time

## 🔧 Configuration

### Producer Configuration

- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

### Consumer Configuration

- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Array errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = ArrayProducer::new(array)
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Compile-Time Size**: Array size known at compile time
- **Zero-Copy**: Efficient array iteration
- **Type Safety**: Full type safety with const generics

## 📝 Examples

For more examples, see:
- [Array Processing Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-array` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Array integration is used for:

1. **Fixed-Size Data**: Process fixed-size data sets
2. **Type Safety**: Compile-time size guarantees
3. **Testing**: Test pipelines with known data
4. **Small Datasets**: Process small, known-size datasets

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-array)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/array)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-vec](../vec/README.md) - Vector-based streaming
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

