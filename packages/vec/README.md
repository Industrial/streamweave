# streamweave-vec

[![Crates.io](https://img.shields.io/crates/v/streamweave-vec.svg)](https://crates.io/crates/streamweave-vec)
[![Documentation](https://docs.rs/streamweave-vec/badge.svg)](https://docs.rs/streamweave-vec)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Vector producer and consumer for StreamWeave**  
*Produce from and consume to dynamic vectors with flexible size handling.*

The `streamweave-vec` package provides vector-based producers and consumers for StreamWeave. It enables reading from vectors and writing to vectors with dynamic size handling.

## ✨ Key Features

- **VecProducer**: Produce items from vectors
- **VecConsumer**: Consume items into vectors
- **Dynamic Size**: Vector size determined at runtime
- **Capacity Management**: Pre-allocate capacity for performance
- **Order Preservation**: Items preserved in order

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-vec = "0.3.0"
```

## 🚀 Quick Start

### Produce from Vector

```rust
use streamweave_vec::VecProducer;
use streamweave_pipeline::PipelineBuilder;

let data = vec![1, 2, 3, 4, 5];
let producer = VecProducer::new(data);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(/* process items */);

pipeline.run().await?;
```

### Consume to Vector

```rust
use streamweave_vec::VecConsumer;
use streamweave_pipeline::PipelineBuilder;

let consumer = VecConsumer::<i32>::new();

let pipeline = PipelineBuilder::new()
    .producer(/* produce items */)
    .consumer(consumer);

pipeline.run().await?;

let vec = consumer.into_vec();
```

## 📖 API Overview

### VecProducer

Produces items from a vector:

```rust
pub struct VecProducer<T> {
    pub data: Vec<T>,
    pub config: ProducerConfig<T>,
}
```

**Key Methods:**
- `new(data)` - Create producer from vector
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream from vector

### VecConsumer

Consumes items into a vector:

```rust
pub struct VecConsumer<T> {
    pub vec: Vec<T>,
    pub config: ConsumerConfig<T>,
}
```

**Key Methods:**
- `new()` - Create consumer
- `with_capacity(capacity)` - Pre-allocate capacity
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Collect items into vector
- `into_vec()` - Get collected vector

## 📚 Usage Examples

### Vector Transformation Pipeline

Transform vector elements:

```rust
use streamweave_vec::{VecProducer, VecConsumer};
use streamweave_pipeline::PipelineBuilder;

let input = vec![1, 2, 3, 4, 5];
let producer = VecProducer::new(input);
let consumer = VecConsumer::<i32>::with_capacity(10);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|x: i32| x * 2)  // Double each element
    .consumer(consumer);

pipeline.run().await?;
```

### Pre-allocated Capacity

Pre-allocate vector capacity:

```rust
use streamweave_vec::VecConsumer;

let consumer = VecConsumer::<i32>::with_capacity(1000);  // Pre-allocate for 1000 items
```

## 🏗️ Architecture

Vector processing flow:

```
Vec<T> ──> VecProducer ──> Stream<T> ──> Transformer ──> Stream<T> ──> VecConsumer ──> Vec<T>
```

**Vector Flow:**
1. VecProducer iterates over vector elements
2. Items flow through transformers
3. VecConsumer collects items into vector
4. Vector size grows dynamically

## 🔧 Configuration

### Producer Configuration

- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

### Consumer Configuration

- **Capacity**: Pre-allocated capacity
- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Vector errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = VecProducer::new(data)
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Pre-allocation**: Use `with_capacity` for known sizes
- **Dynamic Growth**: Vector grows as needed
- **Memory Efficiency**: Efficient vector operations

## 📝 Examples

For more examples, see:
- [Vector Processing Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-vec` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Vector integration is used for:

1. **Dynamic Data**: Process data with unknown size
2. **Memory Efficiency**: Efficient memory usage
3. **Testing**: Test pipelines with dynamic data
4. **Data Collection**: Collect stream items into vectors

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-vec)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/vec)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-array](../array/README.md) - Array-based streaming
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

