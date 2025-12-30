# streamweave-process

[![Crates.io](https://img.shields.io/crates/v/streamweave-process.svg)](https://crates.io/crates/streamweave-process)
[![Documentation](https://docs.rs/streamweave-process/badge.svg)](https://docs.rs/streamweave-process)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Process management for StreamWeave**  
*Manage external processes, stream their output, and pipe data to their input.*

The `streamweave-process` package provides process management producers and consumers for StreamWeave. It enables spawning external processes, streaming their output, and piping data to their input.

## ✨ Key Features

- **ProcessProducer**: Spawn processes and stream their stdout
- **ProcessConsumer**: Spawn processes and pipe data to their stdin
- **Process Lifecycle**: Manage process lifecycle
- **Async I/O**: Non-blocking process I/O
- **Error Handling**: Comprehensive error handling

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-process = "0.6.0"
```

## 🚀 Quick Start

### Stream Process Output

```rust
use streamweave_process::ProcessProducer;
use streamweave_pipeline::PipelineBuilder;

let producer = ProcessProducer::new("ls")
    .arg("-la");

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(/* process output lines */);

pipeline.run().await?;
```

### Pipe Data to Process

```rust
use streamweave_process::ProcessConsumer;
use streamweave_pipeline::PipelineBuilder;

let consumer = ProcessConsumer::new("grep")
    .arg("pattern");

let pipeline = PipelineBuilder::new()
    .producer(/* produce data */)
    .consumer(consumer);

pipeline.run().await?;
```

## 📖 API Overview

### ProcessProducer

Spawns processes and streams their output:

```rust
pub struct ProcessProducer {
    pub command: String,
    pub args: Vec<String>,
    pub config: ProducerConfig<String>,
}
```

**Key Methods:**
- `new(command)` - Create producer with command
- `arg(arg)` - Add command argument
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream from process stdout

### ProcessConsumer

Spawns processes and pipes data to their input:

```rust
pub struct ProcessConsumer {
    pub command: String,
    pub args: Vec<String>,
    pub config: ConsumerConfig<String>,
}
```

**Key Methods:**
- `new(command)` - Create consumer with command
- `arg(arg)` - Add command argument
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Pipe stream data to process stdin

## 📚 Usage Examples

### Process Output Processing

Process process output:

```rust
use streamweave_process::ProcessProducer;
use streamweave_pipeline::PipelineBuilder;

let producer = ProcessProducer::new("find")
    .arg(".")
    .arg("-name")
    .arg("*.rs");

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|line: String| line.to_uppercase())
    .consumer(/* process transformed lines */);

pipeline.run().await?;
```

### Process Input Piping

Pipe data to process:

```rust
use streamweave_process::ProcessConsumer;
use streamweave_pipeline::PipelineBuilder;

let consumer = ProcessConsumer::new("sort")
    .arg("-r");

let pipeline = PipelineBuilder::new()
    .producer(/* produce data */)
    .consumer(consumer);

pipeline.run().await?;
```

### Error Handling

Configure error handling:

```rust
use streamweave_process::ProcessProducer;
use streamweave_error::ErrorStrategy;

let producer = ProcessProducer::new("command")
    .with_error_strategy(ErrorStrategy::Skip);
```

## 🏗️ Architecture

Process management flow:

```text
Process ──> ProcessProducer ──> Stream<String> ──> Transformer ──> Stream<String> ──> ProcessConsumer ──> Process
```

**Process Flow:**
1. ProcessProducer spawns process and streams stdout
2. Output lines flow through transformers
3. ProcessConsumer spawns process and pipes stdin
4. Processes execute asynchronously

## 🔧 Configuration

### Producer Configuration

- **Command**: Command to execute
- **Arguments**: Command arguments
- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

### Consumer Configuration

- **Command**: Command to execute
- **Arguments**: Command arguments
- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Process errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = ProcessProducer::new("command")
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Async Execution**: Processes execute asynchronously
- **Line-by-Line**: Process output line by line
- **Streaming**: Stream data efficiently

## 📝 Examples

For more examples, see:
- [Process Management Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-process` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Process integration is used for:

1. **External Tools**: Integrate with external tools
2. **Process Pipelines**: Build process pipelines
3. **Data Processing**: Process data through external processes
4. **System Integration**: Integrate with system processes

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-process)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/process)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-command](../command/README.md) - Command execution
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

