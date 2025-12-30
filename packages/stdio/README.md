# streamweave-stdio

[![Crates.io](https://img.shields.io/crates/v/streamweave-stdio.svg)](https://crates.io/crates/streamweave-stdio)
[![Documentation](https://docs.rs/streamweave-stdio/badge.svg)](https://docs.rs/streamweave-stdio)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Standard I/O integration for StreamWeave**  
*Read from stdin and write to stdout/stderr with StreamWeave pipelines.*

The `streamweave-stdio` package provides producers and consumers for integrating StreamWeave with POSIX standard streams (stdin, stdout, stderr). It enables building command-line tools and interactive pipelines that process data from standard input and output results to standard output or error streams.

## ✨ Key Features

- **StdinProducer**: Read lines from standard input
- **StdoutConsumer**: Write items to standard output
- **StderrConsumer**: Write items to standard error
- **Line-by-Line Processing**: Process input line by line
- **Interactive Pipelines**: Build interactive command-line tools

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-stdio = "0.3.0"
```

## 🚀 Quick Start

### Basic Pipeline

```rust
use streamweave_stdio::{StdinProducer, StdoutConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(StdinProducer::new())
    .transformer(/* your transformer */)
    .consumer(StdoutConsumer::new());

pipeline.run().await?;
```

### Reading from Stdin

```rust
use streamweave_stdio::StdinProducer;

let producer = StdinProducer::new();
// Reads lines from stdin and produces them as String items
```

### Writing to Stdout

```rust
use streamweave_stdio::StdoutConsumer;

let consumer = StdoutConsumer::new();
// Writes items to stdout
```

### Writing to Stderr

```rust
use streamweave_stdio::StderrConsumer;

let consumer = StderrConsumer::new();
// Writes items to stderr (for error output)
```

## 📖 API Overview

### StdinProducer

Reads lines from standard input:

```rust
pub struct StdinProducer {
    // Internal state
}
```

**Key Methods:**
- `new()` - Create new stdin producer
- `produce()` - Generate stream from stdin

### StdoutConsumer

Writes items to standard output:

```rust
pub struct StdoutConsumer {
    // Internal state
}
```

**Key Methods:**
- `new()` - Create new stdout consumer
- `consume(stream)` - Write stream items to stdout

### StderrConsumer

Writes items to standard error:

```rust
pub struct StderrConsumer {
    // Internal state
}
```

**Key Methods:**
- `new()` - Create new stderr consumer
- `consume(stream)` - Write stream items to stderr

## 📚 Usage Examples

### Simple Echo Pipeline

Echo input to output:

```rust
use streamweave_stdio::{StdinProducer, StdoutConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(StdinProducer::new())
    .consumer(StdoutConsumer::new());

pipeline.run().await?;
```

### Transform and Output

Transform input before outputting:

```rust
use streamweave_stdio::{StdinProducer, StdoutConsumer};
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;

let pipeline = PipelineBuilder::new()
    .producer(StdinProducer::new())
    .transformer(MapTransformer::new(|line: String| {
        line.to_uppercase()
    }))
    .consumer(StdoutConsumer::new());

pipeline.run().await?;
```

### Error Output

Write errors to stderr:

```rust
use streamweave_stdio::{StdinProducer, StderrConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(StdinProducer::new())
    .transformer(/* filter errors */)
    .consumer(StderrConsumer::new());

pipeline.run().await?;
```

### Interactive Processing

Process input interactively:

```rust
use streamweave_stdio::{StdinProducer, StdoutConsumer};
use streamweave_pipeline::PipelineBuilder;

// Process input as it arrives
let pipeline = PipelineBuilder::new()
    .producer(StdinProducer::new())
    .transformer(/* process each line */)
    .consumer(StdoutConsumer::new());

// Run interactively
pipeline.run().await?;
```

## 🏗️ Architecture

Standard I/O integration:

```text
┌──────────┐
│  stdin   │───> StdinProducer ───> Stream ───> Transformer ───> Stream ───> StdoutConsumer ───> stdout
└──────────┘                                                                                      └──────────┘
                                                                                                  ┌──────────┐
                                                                                                  │  stderr  │
                                                                                                  └──────────┘
```

**I/O Flow:**
1. StdinProducer reads lines from stdin
2. Lines flow through transformers
3. StdoutConsumer writes to stdout
4. StderrConsumer writes to stderr (for errors)

## 🔧 Configuration

### Producer Configuration

Configure stdin producer:

```rust
let producer = StdinProducer::new()
    .with_config(ProducerConfig::default()
        .with_name("stdin".to_string()));
```

### Consumer Configuration

Configure stdout/stderr consumers:

```rust
let consumer = StdoutConsumer::new()
    .with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Skip,
        name: "stdout".to_string(),
    });
```

## 🔍 Error Handling

Standard I/O errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(StdinProducer::new())
    .consumer(StdoutConsumer::new());
```

## ⚡ Performance Considerations

- **Line Buffering**: Input is read line by line
- **Streaming**: Output is streamed, not buffered
- **Interactive**: Suitable for interactive use
- **Memory Efficient**: Processes one line at a time

## 📝 Examples

For more examples, see:
- [Standard I/O Example](https://github.com/Industrial/streamweave/tree/main/examples)
- [Command-Line Tools](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-stdio` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

Standard I/O is used for:

1. **Command-Line Tools**: Build CLI tools that process stdin/stdout
2. **Interactive Pipelines**: Process data interactively
3. **Unix Pipelines**: Integrate with Unix pipe operations
4. **Text Processing**: Process text streams line by line
5. **Error Handling**: Separate normal output from error output

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-stdio)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/stdio)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-file](../file/README.md) - File I/O
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

