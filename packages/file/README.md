# streamweave-file

[![Crates.io](https://img.shields.io/crates/v/streamweave-file.svg)](https://crates.io/crates/streamweave-file)
[![Documentation](https://docs.rs/streamweave-file/badge.svg)](https://docs.rs/streamweave-file)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**File I/O for StreamWeave**  
*Read from and write to files with StreamWeave pipelines.*

The `streamweave-file` package provides file-based producers and consumers for StreamWeave. It enables reading files line by line and writing stream items to files, supporting large file processing and efficient file I/O operations.

## ✨ Key Features

- **FileProducer**: Read files line by line
- **FileConsumer**: Write items to files
- **Line-by-Line Processing**: Process large files efficiently
- **Async File I/O**: Non-blocking file operations
- **Large File Support**: Handle files of any size

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-file = "0.3.0"
```

## 🚀 Quick Start

### Read File and Process

```rust
use streamweave_file::{FileProducer, FileConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("input.txt")?)
    .transformer(/* your transformer */)
    .consumer(FileConsumer::new("output.txt")?);

pipeline.run().await?;
```

### Read File

```rust
use streamweave_file::FileProducer;

let producer = FileProducer::new("data.txt")?;
// Reads file line by line, producing String items
```

### Write to File

```rust
use streamweave_file::FileConsumer;

let consumer = FileConsumer::new("output.txt")?;
// Writes items to file
```

## 📖 API Overview

### FileProducer

Reads files line by line:

```rust
pub struct FileProducer {
    // Internal state
}
```

**Key Methods:**
- `new(path)` - Create producer for file at path
- `produce()` - Generate stream from file

### FileConsumer

Writes items to files:

```rust
pub struct FileConsumer {
    // Internal state
}
```

**Key Methods:**
- `new(path)` - Create consumer for file at path
- `consume(stream)` - Write stream items to file

## 📚 Usage Examples

### Simple File Copy

Copy file line by line:

```rust
use streamweave_file::{FileProducer, FileConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("input.txt")?)
    .consumer(FileConsumer::new("output.txt")?);

pipeline.run().await?;
```

### Transform File Content

Transform file while copying:

```rust
use streamweave_file::{FileProducer, FileConsumer};
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;

let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("input.txt")?)
    .transformer(MapTransformer::new(|line: String| {
        line.trim().to_uppercase()
    }))
    .consumer(FileConsumer::new("output.txt")?);

pipeline.run().await?;
```

### Process Large Files

Handle large files efficiently:

```rust
use streamweave_file::FileProducer;
use streamweave_pipeline::PipelineBuilder;

// FileProducer reads line by line, memory efficient
let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("large_file.txt")?)
    .transformer(/* process each line */)
    .consumer(/* write results */);
```

### File Processing Pipeline

Complete file processing workflow:

```rust
use streamweave_file::{FileProducer, FileConsumer};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new("data.csv")?)
    .transformer(/* parse CSV */)
    .transformer(/* transform data */)
    .transformer(/* filter data */)
    .consumer(FileConsumer::new("results.txt")?);

pipeline.run().await?;
```

## 🏗️ Architecture

File I/O integration:

```
┌──────────┐
│  File    │───> FileProducer ───> Stream ───> Transformer ───> Stream ───> FileConsumer ───> File
└──────────┘                                                                                   └──────────┘
```

**File Flow:**
1. FileProducer reads file line by line
2. Lines flow through transformers
3. FileConsumer writes results to file
4. Processing is streaming and memory efficient

## 🔧 Configuration

### Producer Configuration

Configure file producer:

```rust
let producer = FileProducer::new("input.txt")?
    .with_config(ProducerConfig::default()
        .with_name("file-reader".to_string()));
```

### Consumer Configuration

Configure file consumer:

```rust
let consumer = FileConsumer::new("output.txt")?
    .with_config(ConsumerConfig {
        error_strategy: ErrorStrategy::Skip,
        name: "file-writer".to_string(),
    });
```

## 🔍 Error Handling

File I/O errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(FileProducer::new("input.txt")?)
    .consumer(FileConsumer::new("output.txt")?);
```

## ⚡ Performance Considerations

- **Line-by-Line**: Files are read line by line for memory efficiency
- **Streaming**: Output is streamed, not buffered
- **Large Files**: Can handle files larger than memory
- **Async I/O**: Non-blocking file operations

## 📝 Examples

For more examples, see:
- [File Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/file_formats)
- [Large File Processing](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-file` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime with file I/O support
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

File I/O is used for:

1. **File Processing**: Process text files line by line
2. **Data Transformation**: Transform file content
3. **ETL Pipelines**: Extract, transform, load from files
4. **Large File Handling**: Process files larger than memory
5. **Batch Processing**: Process files in batches

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-file)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/file)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-stdio](../stdio/README.md) - Standard I/O
- [streamweave-fs](../fs/README.md) - File system operations
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

