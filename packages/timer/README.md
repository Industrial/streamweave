# streamweave-timer

[![Crates.io](https://img.shields.io/crates/v/streamweave-timer.svg)](https://crates.io/crates/streamweave-timer)
[![Documentation](https://docs.rs/streamweave-timer/badge.svg)](https://docs.rs/streamweave-timer)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Timer and interval producer for StreamWeave**  
*Generate events at regular intervals for time-based processing.*

The `streamweave-timer` package provides timer and interval producers for StreamWeave. It enables generating events at regular intervals for time-based processing, scheduling, and periodic tasks.

## ✨ Key Features

- **IntervalProducer**: Generate events at regular intervals
- **Time-Based Processing**: Time-based event generation
- **Scheduling**: Schedule periodic tasks
- **Timestamp Events**: Emit timestamps at intervals
- **Flexible Intervals**: Configurable interval durations

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-timer = "0.3.0"
```

## 🚀 Quick Start

### Interval Producer

```rust
use streamweave_timer::IntervalProducer;
use streamweave_pipeline::PipelineBuilder;
use std::time::Duration;

let producer = IntervalProducer::new(Duration::from_secs(1));

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(|timestamp: std::time::SystemTime| {
        println!("Tick at {:?}", timestamp);
    });

pipeline.run().await?;
```

### Periodic Processing

```rust
use streamweave_timer::IntervalProducer;
use streamweave_pipeline::PipelineBuilder;
use std::time::Duration;

let producer = IntervalProducer::new(Duration::from_secs(5));

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|_timestamp: std::time::SystemTime| {
        // Perform periodic task
        "Periodic task executed".to_string()
    })
    .consumer(|msg: String| {
        println!("{}", msg);
    });

pipeline.run().await?;
```

## 📖 API Overview

### IntervalProducer

Generates events at regular intervals:

```rust
pub struct IntervalProducer {
    pub interval: Duration,
    pub config: ProducerConfig<std::time::SystemTime>,
}
```

**Key Methods:**
- `new(interval)` - Create producer with interval duration
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream of timestamps

## 📚 Usage Examples

### Periodic Tasks

Execute periodic tasks:

```rust
use streamweave_timer::IntervalProducer;
use streamweave_pipeline::PipelineBuilder;
use std::time::Duration;

let producer = IntervalProducer::new(Duration::from_secs(60));

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(|_timestamp: std::time::SystemTime| {
        // Execute periodic task every minute
        perform_periodic_task();
    });

pipeline.run().await?;
```

### Time-Based Processing

Process events at intervals:

```rust
use streamweave_timer::IntervalProducer;
use streamweave_pipeline::PipelineBuilder;
use std::time::Duration;

let producer = IntervalProducer::new(Duration::from_millis(100));

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|timestamp: std::time::SystemTime| {
        format!("Event at {:?}", timestamp)
    })
    .consumer(|msg: String| {
        println!("{}", msg);
    });

pipeline.run().await?;
```

### Error Handling

Configure error handling:

```rust
use streamweave_timer::IntervalProducer;
use streamweave_error::ErrorStrategy;
use std::time::Duration;

let producer = IntervalProducer::new(Duration::from_secs(1))
    .with_error_strategy(ErrorStrategy::Skip);
```

## 🏗️ Architecture

Timer processing flow:

```text
Timer ──> IntervalProducer ──> Stream<SystemTime> ──> Transformer ──> Stream<T> ──> Consumer
```

**Timer Flow:**
1. IntervalProducer generates timestamps at intervals
2. Timestamps flow through transformers
3. Consumer processes time-based events
4. Events generated continuously at intervals

## 🔧 Configuration

### Producer Configuration

- **Interval**: Duration between events
- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Timer errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = IntervalProducer::new(Duration::from_secs(1))
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Interval Accuracy**: Intervals are approximate
- **Resource Usage**: Continuous timers consume resources
- **Scheduling**: Use appropriate intervals for tasks

## 📝 Examples

For more examples, see:
- [Timer Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-timer` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Timer integration is used for:

1. **Periodic Tasks**: Execute tasks at regular intervals
2. **Scheduling**: Schedule periodic operations
3. **Time-Based Processing**: Process events based on time
4. **Monitoring**: Monitor systems at intervals

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-timer)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/timer)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

