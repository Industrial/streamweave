# streamweave-fs

[![Crates.io](https://img.shields.io/crates/v/streamweave-fs.svg)](https://crates.io/crates/streamweave-fs)
[![Documentation](https://docs.rs/streamweave-fs/badge.svg)](https://docs.rs/streamweave-fs)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**File system operations for StreamWeave**  
*Process directories, traverse file systems, and handle file system events.*

The `streamweave-fs` package provides file system operations for StreamWeave. It enables directory traversal, file system monitoring, directory synchronization, and processing files in directories as streams.

## ✨ Key Features

- **DirectoryProducer**: Traverse directories and produce file paths
- **DirectoryConsumer**: Write items to directories
- **Recursive Traversal**: Process directories recursively
- **File System Events**: Monitor file system changes
- **Directory Operations**: List, filter, and process directories

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-fs = "0.3.0"
```

## 🚀 Quick Start

### Process Directory

```rust
use streamweave_fs::DirectoryProducer;
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(DirectoryProducer::new("data/")?)
    .transformer(/* process each file path */)
    .consumer(/* handle results */);

pipeline.run().await?;
```

### Recursive Directory Traversal

```rust
use streamweave_fs::DirectoryProducer;

let producer = DirectoryProducer::new("data/")?
    .recursive(true);
// Traverses directory recursively, producing file paths
```

## 📖 API Overview

### DirectoryProducer

Traverses directories and produces file paths:

```rust
pub struct DirectoryProducer {
    // Internal state
}
```

**Key Methods:**
- `new(path)` - Create producer for directory
- `recursive(enable)` - Enable recursive traversal
- `produce()` - Generate stream of file paths

### DirectoryConsumer

Writes items to directories:

```rust
pub struct DirectoryConsumer {
    // Internal state
}
```

**Key Methods:**
- `new(path)` - Create consumer for directory
- `consume(stream)` - Write stream items to directory

## 📚 Usage Examples

### List Directory Contents

List all files in a directory:

```rust
use streamweave_fs::DirectoryProducer;
use streamweave_pipeline::PipelineBuilder;
use streamweave_stdio::StdoutConsumer;

let pipeline = PipelineBuilder::new()
    .producer(DirectoryProducer::new("data/")?)
    .consumer(StdoutConsumer::new());

pipeline.run().await?;
```

### Recursive Directory Processing

Process all files recursively:

```rust
use streamweave_fs::DirectoryProducer;
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(DirectoryProducer::new("data/")?.recursive(true))
    .transformer(/* process each file */)
    .consumer(/* write results */);
```

### Filter Files by Extension

Filter files by extension:

```rust
use streamweave_fs::DirectoryProducer;
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::FilterTransformer;

let pipeline = PipelineBuilder::new()
    .producer(DirectoryProducer::new("data/")?.recursive(true))
    .transformer(FilterTransformer::new(|path: &PathBuf| {
        path.extension().and_then(|ext| ext.to_str()) == Some("txt")
    }))
    .consumer(/* process filtered files */);
```

### Directory Synchronization

Synchronize directories:

```rust
use streamweave_fs::{DirectoryProducer, DirectoryConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(DirectoryProducer::new("source/")?.recursive(true))
    .transformer(/* transform file paths */)
    .consumer(DirectoryConsumer::new("target/")?);

pipeline.run().await?;
```

## 🏗️ Architecture

File system operations:

```text
┌──────────────┐
│  Directory   │───> DirectoryProducer ───> Stream ───> Transformer ───> Stream ───> DirectoryConsumer ───> Directory
└──────────────┘                                                                                                └──────────────┘
```

**File System Flow:**
1. DirectoryProducer traverses directory
2. File paths flow through transformers
3. DirectoryConsumer writes results to directory
4. Recursive traversal processes subdirectories

## 🔧 Configuration

### Producer Configuration

Configure directory producer:

```rust
let producer = DirectoryProducer::new("data/")?
    .recursive(true)
    .with_config(ProducerConfig::default()
        .with_name("directory-reader".to_string()));
```

### Consumer Configuration

Configure directory consumer:

```rust
let consumer = DirectoryConsumer::new("output/")?
    .with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Skip,
        name: "directory-writer".to_string(),
    });
```

## 🔍 Error Handling

File system errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(DirectoryProducer::new("data/")?)
    .consumer(/* consumer */);
```

## ⚡ Performance Considerations

- **Streaming**: Directories are traversed as streams
- **Recursive**: Recursive traversal is efficient
- **Memory Efficient**: Processes one file at a time
- **Async I/O**: Non-blocking file system operations

## 📝 Examples

For more examples, see:
- [File System Example](https://github.com/Industrial/streamweave/tree/main/examples)
- [Directory Processing](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-fs` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime with file system support
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

File system operations are used for:

1. **Directory Processing**: Process all files in a directory
2. **File System Monitoring**: Monitor directory changes
3. **Directory Synchronization**: Sync directories
4. **Batch File Processing**: Process multiple files
5. **Recursive Operations**: Process directory trees

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-fs)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/fs)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-file](../file/README.md) - File I/O
- [streamweave-stdio](../stdio/README.md) - Standard I/O
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

