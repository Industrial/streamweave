# streamweave-env

[![Crates.io](https://img.shields.io/crates/v/streamweave-env.svg)](https://crates.io/crates/streamweave-env)
[![Documentation](https://docs.rs/streamweave-env/badge.svg)](https://docs.rs/streamweave-env)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Environment variable producer and consumer for StreamWeave**  
*Read from and write to environment variables with streaming processing.*

The `streamweave-env` package provides environment variable producers and consumers for StreamWeave. It enables reading environment variables and setting environment variables through streams.

## ✨ Key Features

- **EnvVarProducer**: Produce environment variables as key-value pairs
- **EnvVarConsumer**: Consume key-value pairs and set environment variables
- **Filtering**: Filter specific environment variables
- **Validation**: Validate environment variable names
- **Safe Operations**: Safe environment variable operations

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-env = "0.3.0"
```

## 🚀 Quick Start

### Read All Environment Variables

```rust
use streamweave_env::EnvVarProducer;
use streamweave_pipeline::PipelineBuilder;

let producer = EnvVarProducer::new();

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .consumer(/* process (key, value) pairs */);

pipeline.run().await?;
```

### Read Specific Environment Variables

```rust
use streamweave_env::EnvVarProducer;

let producer = EnvVarProducer::with_vars(vec![
    "PATH".to_string(),
    "HOME".to_string(),
    "USER".to_string(),
]);
```

### Set Environment Variables

```rust
use streamweave_env::EnvVarConsumer;
use streamweave_pipeline::PipelineBuilder;

let consumer = EnvVarConsumer::new();

let pipeline = PipelineBuilder::new()
    .producer(/* produce (key, value) pairs */)
    .consumer(consumer);

pipeline.run().await?;
```

## 📖 API Overview

### EnvVarProducer

Produces environment variables as key-value pairs:

```rust
pub struct EnvVarProducer {
    pub filter: Option<Vec<String>>,
    pub config: ProducerConfig<(String, String)>,
}
```

**Key Methods:**
- `new()` - Create producer for all environment variables
- `with_vars(vars)` - Create producer for specific variables
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream of (key, value) pairs

### EnvVarConsumer

Consumes key-value pairs and sets environment variables:

```rust
pub struct EnvVarConsumer {
    pub config: ConsumerConfig<(String, String)>,
}
```

**Key Methods:**
- `new()` - Create consumer
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Set environment variables from stream

## 📚 Usage Examples

### Filter Environment Variables

Filter specific environment variables:

```rust
use streamweave_env::EnvVarProducer;

let producer = EnvVarProducer::with_vars(vec![
    "PATH".to_string(),
    "HOME".to_string(),
]);
```

### Transform Environment Variables

Transform environment variables:

```rust
use streamweave_env::{EnvVarProducer, EnvVarConsumer};
use streamweave_pipeline::PipelineBuilder;

let producer = EnvVarProducer::new();
let consumer = EnvVarConsumer::new();

let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(|(key, value): (String, String)| {
        (key.to_uppercase(), value)  // Uppercase keys
    })
    .consumer(consumer);

pipeline.run().await?;
```

### Error Handling

Configure error handling:

```rust
use streamweave_env::EnvVarConsumer;
use streamweave_error::ErrorStrategy;

let consumer = EnvVarConsumer::new()
    .with_error_strategy(ErrorStrategy::Skip);  // Skip invalid variable names
```

## 🏗️ Architecture

Environment variable processing flow:

```text
Environment ──> EnvVarProducer ──> Stream<(String, String)> ──> Transformer ──> Stream<(String, String)> ──> EnvVarConsumer ──> Environment
```

**Environment Flow:**
1. EnvVarProducer reads environment variables
2. (key, value) pairs flow through transformers
3. EnvVarConsumer sets environment variables
4. Variable names are validated

## 🔧 Configuration

### Producer Configuration

- **Filter**: Optional list of variable names to include
- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

### Consumer Configuration

- **Error Strategy**: Error handling strategy
- **Name**: Component name for logging

## 🔍 Error Handling

Environment variable errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let consumer = EnvVarConsumer::new()
    .with_error_strategy(ErrorStrategy::Skip);  // Skip invalid names
```

## ⚡ Performance Considerations

- **Filtering**: Filter variables for better performance
- **Validation**: Validate variable names before setting
- **Safe Operations**: Use safe environment variable operations

## 📝 Examples

For more examples, see:
- [Environment Variable Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-env` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

Environment variable integration is used for:

1. **Configuration**: Read configuration from environment
2. **Runtime Setup**: Set environment variables at runtime
3. **Configuration Pipelines**: Process configuration through pipelines
4. **Environment Management**: Manage environment variables

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-env)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/env)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-error](../error/README.md) - Error handling

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

