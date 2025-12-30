# streamweave-tempfile

[![Crates.io](https://img.shields.io/crates/v/streamweave-tempfile.svg)](https://crates.io/crates/streamweave-tempfile)
[![Documentation](https://docs.rs/streamweave-tempfile/badge.svg)](https://docs.rs/streamweave-tempfile)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Temporary file handling for StreamWeave**  
*Create and process temporary files with automatic cleanup.*

The `streamweave-tempfile` package provides temporary file handling for StreamWeave. It enables creating temporary files, processing data in temporary files, and automatic cleanup when files are no longer needed.

## ✨ Key Features

- **TempfileProducer**: Read from temporary files
- **TempfileConsumer**: Write to temporary files
- **Automatic Cleanup**: Temporary files are automatically cleaned up
- **RAII Pattern**: Files are cleaned up when dropped
- **Configurable Cleanup**: Control when files are deleted

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-tempfile = "0.3.0"
```

## 🚀 Quick Start

### Create and Process Temp File

```rust
use streamweave_tempfile::{TempfileProducer, TempfileConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(TempfileProducer::new()?)
    .transformer(/* your transformer */)
    .consumer(TempfileConsumer::new()?);

pipeline.run().await?;
// Temporary files are automatically cleaned up
```

### Create Temp File

```rust
use streamweave_tempfile::TempfileProducer;

let producer = TempfileProducer::new()?;
// Creates temporary file, reads from it
// File is automatically deleted when producer is dropped
```

### Write to Temp File

```rust
use streamweave_tempfile::TempfileConsumer;

let consumer = TempfileConsumer::new()?;
// Writes to temporary file
// File is automatically deleted when consumer is dropped
```

## 📖 API Overview

### TempfileProducer

Reads from temporary files:

```rust
pub struct TempfileProducer {
    // Internal state
}
```

**Key Methods:**
- `new()` - Create producer with new temp file
- `produce()` - Generate stream from temp file
- `path()` - Get path to temp file

### TempfileConsumer

Writes to temporary files:

```rust
pub struct TempfileConsumer {
    // Internal state
}
```

**Key Methods:**
- `new()` - Create consumer with new temp file
- `consume(stream)` - Write stream items to temp file
- `path()` - Get path to temp file

## 📚 Usage Examples

### Process Temp Data

Process data in temporary file:

```rust
use streamweave_tempfile::{TempfileProducer, TempfileConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(TempfileProducer::new()?)
    .transformer(/* transform data */)
    .consumer(TempfileConsumer::new()?);

pipeline.run().await?;
// Both temp files are automatically cleaned up
```

### Keep Temp File

Keep temporary file after processing:

```rust
use streamweave_tempfile::TempfileConsumer;

let consumer = TempfileConsumer::new()?
    .keep_on_drop(true);

// Process...
pipeline.run().await?;

// File is kept, can access via consumer.path()
let path = consumer.path();
```

### Custom Temp Directory

Create temp files in specific directory:

```rust
use streamweave_tempfile::TempfileProducer;

let producer = TempfileProducer::new_in_dir("/tmp/my-temp")?;
// Creates temp file in specified directory
```

### Temp File Lifecycle

Control temp file lifecycle:

```rust
use streamweave_tempfile::TempfileConsumer;

let consumer = TempfileConsumer::new()?
    .keep_on_drop(false)  // Delete on drop (default)
    .prefix("my-prefix")  // Custom filename prefix
    .suffix(".tmp");      // Custom filename suffix

// Process...
// File is automatically deleted when consumer is dropped
```

## 🏗️ Architecture

Temporary file handling:

```text
┌──────────────┐
│  Temp File   │───> TempfileProducer ───> Stream ───> Transformer ───> Stream ───> TempfileConsumer ───> Temp File
└──────────────┘                                                                                              └──────────────┘
                                                                                                                                  │
                                                                                                                                  ▼
                                                                                                                          (Auto-cleanup)
```

**Temp File Flow:**
1. TempfileProducer/TempfileConsumer creates temp file
2. Data flows through pipeline
3. Temp file is automatically cleaned up when dropped
4. Optional: Keep file for later access

## 🔧 Configuration

### Producer Configuration

Configure temp file producer:

```rust
let producer = TempfileProducer::new()?
    .prefix("data")
    .suffix(".tmp")
    .with_config(ProducerConfig::default()
        .with_name("temp-reader".to_string()));
```

### Consumer Configuration

Configure temp file consumer:

```rust
let consumer = TempfileConsumer::new()?
    .keep_on_drop(false)
    .with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Skip,
        name: "temp-writer".to_string(),
    });
```

## 🔍 Error Handling

Temp file errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(TempfileProducer::new()?)
    .consumer(TempfileConsumer::new()?);
```

## ⚡ Performance Considerations

- **Automatic Cleanup**: Files are cleaned up automatically
- **RAII Pattern**: Cleanup happens on drop
- **Memory Efficient**: Temp files use disk, not memory
- **Configurable**: Control cleanup behavior

## 📝 Examples

For more examples, see:
- [Temp File Example](https://github.com/Industrial/streamweave/tree/main/examples)
- [File Processing](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-tempfile` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `tempfile` - Temporary file creation
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

Temporary files are used for:

1. **Intermediate Processing**: Store intermediate results
2. **Large Data**: Handle data too large for memory
3. **Batch Processing**: Process data in batches
4. **Testing**: Create test data files
5. **Data Transformation**: Transform data through temp files

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-tempfile)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/tempfile)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-file](../file/README.md) - File I/O
- [streamweave-fs](../fs/README.md) - File system operations
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

