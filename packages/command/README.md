# streamweave-command

[![Crates.io](https://img.shields.io/crates/v/streamweave-command.svg)](https://crates.io/crates/streamweave-command)
[![Documentation](https://docs.rs/streamweave-command/badge.svg)](https://docs.rs/streamweave-command)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Command execution producer and consumer for StreamWeave**  
*Execute shell commands and stream their output, or pipe stream data to commands.*

The `streamweave-command` package provides command execution producers and consumers for StreamWeave. It enables executing shell commands and streaming their output, or piping stream data to commands as input.

## ✨ Key Features

- **CommandProducer**: Execute commands and stream output line by line
- **CommandConsumer**: Execute commands with stream items as input
- **Async Execution**: Non-blocking command execution
- **Line-by-Line Processing**: Process command output line by line
- **Error Handling**: Comprehensive error handling

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-command = "0.3.0"
```

## 🚀 Quick Start

### Execute Command and Stream Output

```rust
use streamweave_command::CommandProducer;
use streamweave_pipeline::PipelineBuilder;

let producer = CommandProducer::new("ls", vec!["-la"]);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(/* process output lines */);

pipeline.run().await?;
```

### Pipe Data to Command

```rust
use streamweave_command::CommandConsumer;
use streamweave_pipeline::PipelineBuilder;

let consumer = CommandConsumer::new("grep", vec!["pattern".to_string()]);

let pipeline = PipelineBuilder::new()
    .producer(/* produce data */)
    .consumer(consumer);

pipeline.run().await?;
```

## 📖 API Overview

### CommandProducer

Executes commands and streams output:

```rust
pub struct CommandProducer {
    pub command: String,
    pub args: Vec<String>,
    pub config: ProducerConfig<String>,
}
```

**Key Methods:**
- `new(command, args)` - Create producer with command and arguments
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream from command output

### CommandConsumer

Executes commands with stream items as input:

```rust
pub struct CommandConsumer<T> {
    pub command: Option<Command>,
    pub config: ConsumerConfig<T>,
}
```

**Key Methods:**
- `new(command, args)` - Create consumer with command and arguments
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Execute command for each stream item

## 📚 Usage Examples

### Process Command Output

Process command output through pipeline:

```rust
use streamweave_command::CommandProducer;
use streamweave_pipeline::PipelineBuilder;

let producer = CommandProducer::new("find", vec![".", "-name", "*.rs"]);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|line: String| line.to_uppercase())
    .consumer(/* process transformed lines */);

pipeline.run().await?;
```

### Chain Commands

Chain multiple commands:

```rust
use streamweave_command::{CommandProducer, CommandConsumer};
use streamweave_pipeline::PipelineBuilder;

let producer = CommandProducer::new("cat", vec!["file.txt"]);
let consumer = CommandConsumer::new("wc", vec!["-l".to_string()]);

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(consumer);

pipeline.run().await?;
```

### Error Handling

Configure error handling:

```rust
use streamweave_command::CommandProducer;
use streamweave_error::ErrorStrategy;

let producer = CommandProducer::new("command", vec!["arg"])
    .with_error_strategy(ErrorStrategy::Skip);
```

## 🏗️ Architecture

Command execution flow:

```
Command ──> CommandProducer ──> Stream<String> ──> Transformer ──> Stream<T> ──> CommandConsumer ──> Command
```

**Command Flow:**
1. CommandProducer executes command and streams stdout
2. Output lines flow through transformers
3. CommandConsumer executes command with stream items
4. Commands execute asynchronously

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

Command errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = CommandProducer::new("command", vec!["arg"])
    .with_error_strategy(ErrorStrategy::Skip);
```

## ⚡ Performance Considerations

- **Async Execution**: Commands execute asynchronously
- **Line-by-Line**: Process output line by line
- **Streaming**: Stream data efficiently

## 📝 Examples

For more examples, see:
- [Command Execution Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-command` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Command integration is used for:

1. **Shell Integration**: Integrate with shell commands
2. **Process Pipelines**: Build process pipelines
3. **Data Processing**: Process data through external commands
4. **System Integration**: Integrate with system tools

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-command)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/command)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-process](../process/README.md) - Process management
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

