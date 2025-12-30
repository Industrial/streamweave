# streamweave-tokio

[![Crates.io](https://img.shields.io/crates/v/streamweave-tokio.svg)](https://crates.io/crates/streamweave-tokio)
[![Documentation](https://docs.rs/streamweave-tokio/badge.svg)](https://docs.rs/streamweave-tokio)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Tokio channel integration for StreamWeave**  
*Integrate StreamWeave with Tokio channels for async communication.*

The `streamweave-tokio` package provides Tokio channel producers and consumers for StreamWeave. It enables reading from and writing to Tokio channels, allowing StreamWeave to integrate with existing async code that uses channels.

## ✨ Key Features

- **ChannelProducer**: Read items from Tokio channels
- **ChannelConsumer**: Write items to Tokio channels
- **Async Integration**: Seamless integration with Tokio async code
- **Channel Communication**: Bridge StreamWeave and Tokio channels
- **Type Safety**: Full type safety with channel types

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-tokio = "0.3.0"
```

## 🚀 Quick Start

### Read from Channel

```rust
use streamweave_tokio::ChannelProducer;
use streamweave_pipeline::PipelineBuilder;
use tokio::sync::mpsc;

let (sender, receiver) = mpsc::channel(100);
let producer = ChannelProducer::new(receiver);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(/* process items */);

pipeline.run().await?;
```

### Write to Channel

```rust
use streamweave_tokio::ChannelConsumer;
use streamweave_pipeline::PipelineBuilder;
use tokio::sync::mpsc;

let (sender, receiver) = mpsc::channel(100);
let consumer = ChannelConsumer::new(sender);

let pipeline = PipelineBuilder::new()
    .producer(/* produce items */)
    .consumer(consumer);

pipeline.run().await?;
```

## 📖 API Overview

### ChannelProducer

Reads items from a Tokio channel:

```rust
pub struct ChannelProducer<T> {
    pub receiver: Option<Receiver<T>>,
    pub config: ProducerConfig<T>,
}
```

**Key Methods:**
- `new(receiver)` - Create producer with channel receiver
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream from channel

### ChannelConsumer

Writes items to a Tokio channel:

```rust
pub struct ChannelConsumer<T> {
    pub channel: Option<Sender<T>>,
    pub config: ConsumerConfig<T>,
}
```

**Key Methods:**
- `new(sender)` - Create consumer with channel sender
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Send stream items to channel

## 📚 Usage Examples

### Channel Bridge

Bridge between channels and StreamWeave:

```rust
use streamweave_tokio::{ChannelProducer, ChannelConsumer};
use streamweave_pipeline::PipelineBuilder;
use tokio::sync::mpsc;

let (input_sender, input_receiver) = mpsc::channel(100);
let (output_sender, output_receiver) = mpsc::channel(100);

let producer = ChannelProducer::new(input_receiver);
let consumer = ChannelConsumer::new(output_sender);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|x: i32| x * 2)
    .consumer(consumer);

pipeline.run().await?;
```

### Async Integration

Integrate with async code:

```rust
use streamweave_tokio::ChannelProducer;
use streamweave_pipeline::PipelineBuilder;
use tokio::sync::mpsc;

let (sender, receiver) = mpsc::channel(100);

// Spawn async task that sends to channel
tokio::spawn(async move {
    for i in 0..10 {
        sender.send(i).await.unwrap();
    }
});

let producer = ChannelProducer::new(receiver);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(/* process items */);

pipeline.run().await?;
```

### Error Handling

Configure error handling:

```rust
use streamweave_tokio::ChannelProducer;
use streamweave_error::ErrorStrategy;
use tokio::sync::mpsc;

let (_, receiver) = mpsc::channel(100);
let producer = ChannelProducer::new(receiver)
    .with_error_strategy(ErrorStrategy::Skip);
```

## 🏗️ Architecture

Channel integration flow:

```text
Tokio Channel ──> ChannelProducer ──> Stream<T> ──> Transformer ──> Stream<T> ──> ChannelConsumer ──> Tokio Channel
```

**Channel Flow:**
1. ChannelProducer reads from Tokio channel
2. Items flow through transformers
3. ChannelConsumer writes to Tokio channel
4. Channels bridge async code and StreamWeave

## 🔧 Configuration

### Producer Configuration

- **Receiver**: Tokio channel receiver
- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

### Consumer Configuration

- **Sender**: Tokio channel sender
- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Channel errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = ChannelProducer::new(receiver)
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Channel Buffer**: Configure channel buffer size
- **Async Performance**: Leverage Tokio async performance
- **Backpressure**: Handle channel backpressure

## 📝 Examples

For more examples, see:
- [Tokio Channel Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-tokio` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `tokio-stream` - Stream utilities
- `futures` - Stream utilities

## 🎯 Use Cases

Tokio channel integration is used for:

1. **Async Integration**: Integrate with async Tokio code
2. **Channel Communication**: Bridge channels and StreamWeave
3. **Concurrent Processing**: Process data concurrently
4. **System Integration**: Integrate with existing async systems

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-tokio)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/tokio)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

