# streamweave-signal

[![Crates.io](https://img.shields.io/crates/v/streamweave-signal.svg)](https://crates.io/crates/streamweave-signal)
[![Documentation](https://docs.rs/streamweave-signal/badge.svg)](https://docs.rs/streamweave-signal)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Unix signal handling for StreamWeave**  
*Handle Unix signals (SIGINT, SIGTERM) and trigger graceful shutdowns.*

The `streamweave-signal` package provides signal handling producers for StreamWeave. It enables handling Unix signals and triggering graceful shutdowns or other actions based on signal events.

## ✨ Key Features

- **SignalProducer**: Produce events when Unix signals are received
- **Signal Types**: Support for SIGINT and SIGTERM
- **Graceful Shutdown**: Trigger graceful shutdowns on signals
- **Event-Driven**: Event-driven signal handling
- **Unix Only**: Unix-specific signal handling

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-signal = "0.6.0"
```

## 🚀 Quick Start

### Handle Signals

```rust
use streamweave_signal::{SignalProducer, Signal};
use streamweave_pipeline::PipelineBuilder;

let producer = SignalProducer::new();

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(|signal: Signal| {
        match signal {
            Signal::Interrupt => println!("Received SIGINT"),
            Signal::Terminate => println!("Received SIGTERM"),
        }
    });

pipeline.run().await?;
```

### Graceful Shutdown

```rust
use streamweave_signal::{SignalProducer, Signal};
use streamweave_pipeline::PipelineBuilder;

let producer = SignalProducer::new();

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(|signal: Signal| {
        match signal {
            Signal::Interrupt | Signal::Terminate => {
                println!("Shutting down gracefully...");
                // Perform cleanup
            }
        }
    });

pipeline.run().await?;
```

## 📖 API Overview

### SignalProducer

Produces events when Unix signals are received:

```rust
pub struct SignalProducer {
    pub config: ProducerConfig<Signal>,
}
```

**Key Methods:**
- `new()` - Create signal producer
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream of signal events

### Signal

Represents a Unix signal:

```rust
pub enum Signal {
    Interrupt,   // SIGINT
    Terminate,   // SIGTERM
}
```

**Key Methods:**
- `from_number(sig)` - Convert from signal number
- `number()` - Get signal number

## 📚 Usage Examples

### Signal-Based Processing

Process signals through pipeline:

```rust
use streamweave_signal::{SignalProducer, Signal};
use streamweave_pipeline::PipelineBuilder;

let producer = SignalProducer::new();

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|signal: Signal| {
        format!("Received signal: {:?}", signal)
    })
    .consumer(|msg: String| {
        println!("{}", msg);
    });

pipeline.run().await?;
```

### Signal Handling

Handle different signals:

```rust
use streamweave_signal::{SignalProducer, Signal};
use streamweave_pipeline::PipelineBuilder;

let producer = SignalProducer::new();

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(|signal: Signal| {
        match signal {
            Signal::Interrupt => {
                println!("Interrupted by user");
            }
            Signal::Terminate => {
                println!("Termination requested");
            }
        }
    });

pipeline.run().await?;
```

### Error Handling

Configure error handling:

```rust
use streamweave_signal::SignalProducer;
use streamweave_error::ErrorStrategy;

let producer = SignalProducer::new()
    .with_error_strategy(ErrorStrategy::Skip);
```

## 🏗️ Architecture

Signal handling flow:

```text
Unix Signal ──> SignalProducer ──> Stream<Signal> ──> Transformer ──> Stream<T> ──> Consumer
```

**Signal Flow:**
1. Unix signal received by process
2. SignalProducer emits Signal event
3. Signal events flow through transformers
4. Consumer handles signal events

## 🔧 Configuration

### Producer Configuration

- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Signal errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = SignalProducer::new()
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Event-Driven**: Signal handling is event-driven
- **Non-Blocking**: Signal handling is non-blocking
- **Unix Only**: Only available on Unix systems

## 📝 Examples

For more examples, see:
- [Signal Handling Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-signal` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Signal integration is used for:

1. **Graceful Shutdown**: Handle shutdown signals gracefully
2. **Signal Processing**: Process Unix signals
3. **Event-Driven**: Build event-driven applications
4. **System Integration**: Integrate with system signals

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-signal)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/signal)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

