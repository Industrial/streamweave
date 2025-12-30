# streamweave-parquet

[![Crates.io](https://img.shields.io/crates/v/streamweave-parquet.svg)](https://crates.io/crates/streamweave-parquet)
[![Documentation](https://docs.rs/streamweave-parquet/badge.svg)](https://docs.rs/streamweave-parquet)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Parquet format support for StreamWeave**  
*Read and write Parquet files with columnar data processing.*

The `streamweave-parquet` package provides Parquet producers and consumers for StreamWeave. Parquet is a columnar storage format optimized for analytics workloads, enabling efficient processing of large datasets.

## ✨ Key Features

- **ParquetProducer**: Read Parquet files row by row
- **ParquetConsumer**: Write items to Parquet files
- **Columnar Processing**: Efficient columnar data operations
- **Schema Support**: Handle Parquet schemas
- **Large File Support**: Process large Parquet files efficiently

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-parquet = "0.6.0"
```

## 🚀 Quick Start

### Read and Process Parquet

```rust
use streamweave_parquet::{ParquetProducer, ParquetConsumer, ParquetReadConfig, ParquetWriteConfig};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(ParquetProducer::new("data.parquet", ParquetReadConfig::default())?)
    .transformer(/* your transformer */)
    .consumer(ParquetConsumer::new("output.parquet", ParquetWriteConfig::default())?);

pipeline.run().await?;
```

### Read Parquet File

```rust
use streamweave_parquet::{ParquetProducer, ParquetReadConfig};

let producer = ParquetProducer::new("data.parquet", ParquetReadConfig::default())?;
// Reads Parquet file row by row, producing records
```

### Write Parquet File

```rust
use streamweave_parquet::{ParquetConsumer, ParquetWriteConfig};

let consumer = ParquetConsumer::new("output.parquet", ParquetWriteConfig::default())?;
// Writes records to Parquet file
```

## 📖 API Overview

### ParquetProducer

Reads Parquet files row by row:

```rust
pub struct ParquetProducer {
    // Internal state
}
```

**Key Methods:**
- `new(path, config)` - Create producer for Parquet file
- `produce()` - Generate stream from Parquet file

### ParquetConsumer

Writes items to Parquet files:

```rust
pub struct ParquetConsumer {
    // Internal state
}
```

**Key Methods:**
- `new(path, config)` - Create consumer for Parquet file
- `consume(stream)` - Write stream items to Parquet file

### ParquetReadConfig

Configuration for reading Parquet files:

```rust
pub struct ParquetReadConfig {
    pub batch_size: usize,
    // ... other options
}
```

### ParquetWriteConfig

Configuration for writing Parquet files:

```rust
pub struct ParquetWriteConfig {
    pub row_group_size: usize,
    // ... other options
}
```

## 📚 Usage Examples

### Simple Parquet Processing

Process Parquet file:

```rust
use streamweave_parquet::{ParquetProducer, ParquetConsumer, ParquetReadConfig, ParquetWriteConfig};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(ParquetProducer::new("input.parquet", ParquetReadConfig::default())?)
    .consumer(ParquetConsumer::new("output.parquet", ParquetWriteConfig::default())?);

pipeline.run().await?;
```

### Schema Handling

Work with Parquet schemas:

```rust
use streamweave_parquet::{ParquetProducer, ParquetReadConfig};

let producer = ParquetProducer::new("data.parquet", ParquetReadConfig::default())?;
// Schema is automatically inferred from Parquet file
```

### Columnar Operations

Process columnar data:

```rust
use streamweave_parquet::{ParquetProducer, ParquetReadConfig};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(ParquetProducer::new("data.parquet", ParquetReadConfig::default())?)
    .transformer(/* process columnar data */)
    .consumer(/* write results */);
```

### Large Parquet Processing

Process large Parquet files efficiently:

```rust
use streamweave_parquet::{ParquetProducer, ParquetReadConfig};
use streamweave_pipeline::PipelineBuilder;

// ParquetProducer reads in batches, memory efficient
let config = ParquetReadConfig {
    batch_size: 1000,  // Read 1000 rows at a time
    ..Default::default()
};

let pipeline = PipelineBuilder::new()
    .producer(ParquetProducer::new("large_file.parquet", config)?)
    .transformer(/* process each batch */)
    .consumer(/* write results */);
```

## 🏗️ Architecture

Parquet processing flow:

```text
┌──────────────┐
│ Parquet File │───> ParquetProducer ───> Stream<Record> ───> Transformer ───> Stream<Record> ───> ParquetConsumer ───> Parquet File
└──────────────┘                                                                                                        └──────────────┘
```

**Parquet Flow:**
1. ParquetProducer reads Parquet file in batches
2. Records flow through transformers
3. ParquetConsumer writes records to Parquet file
4. Processing is columnar and memory efficient

## 🔧 Configuration

### Read Configuration

Configure Parquet reading:

```rust
let config = ParquetReadConfig {
    batch_size: 1000,  // Batch size for reading
    // ... other options
};
```

### Write Configuration

Configure Parquet writing:

```rust
let config = ParquetWriteConfig {
    row_group_size: 10000,  // Row group size for writing
    // ... other options
};
```

## 🔍 Error Handling

Parquet errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(ParquetProducer::new("data.parquet", ParquetReadConfig::default())?)
    .consumer(ParquetConsumer::new("output.parquet", ParquetWriteConfig::default())?);
```

## ⚡ Performance Considerations

- **Columnar Format**: Parquet is columnar, enabling efficient column operations
- **Batch Processing**: Reads and writes in batches
- **Compression**: Parquet files are compressed
- **Large Files**: Can handle Parquet files larger than memory

## 📝 Examples

For more examples, see:
- [Parquet Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/file_formats)
- [Large Parquet Processing](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-parquet` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `parquet` - Parquet format library
- `arrow` - Apache Arrow for columnar data
- `tokio` - Async runtime
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

Parquet processing is used for:

1. **Analytics Workloads**: Process large analytical datasets
2. **Data Warehousing**: Store and process data in Parquet format
3. **Columnar Operations**: Efficient column-based operations
4. **Large File Processing**: Process large Parquet files
5. **Data Lake Integration**: Integrate with data lake architectures

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-parquet)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/parquet)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-file](../file/README.md) - File I/O
- [streamweave-csv](../csv/README.md) - CSV format
- [streamweave-jsonl](../jsonl/README.md) - JSON Lines format
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

