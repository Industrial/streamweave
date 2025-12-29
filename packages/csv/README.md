# streamweave-csv

[![Crates.io](https://img.shields.io/crates/v/streamweave-csv.svg)](https://crates.io/crates/streamweave-csv)
[![Documentation](https://docs.rs/streamweave-csv/badge.svg)](https://docs.rs/streamweave-csv)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**CSV parsing and serialization for StreamWeave**  
*Read and write CSV files with streaming processing.*

The `streamweave-csv` package provides CSV producers and consumers for StreamWeave. It enables reading CSV files row by row and writing stream items to CSV files, with support for headers, custom delimiters, and streaming processing of large CSV files.

## ✨ Key Features

- **CsvProducer**: Read CSV files row by row
- **CsvConsumer**: Write items to CSV files
- **Header Support**: Handle CSV headers automatically
- **Custom Delimiters**: Configure delimiter characters
- **Streaming Processing**: Process large CSV files efficiently
- **Type-Safe**: Use serde for type-safe CSV parsing

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-csv = "0.3.0"
```

## 🚀 Quick Start

### Read and Process CSV

```rust
use streamweave_csv::{CsvProducer, CsvConsumer, CsvReadConfig, CsvWriteConfig};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(CsvProducer::new("data.csv", CsvReadConfig::default())?)
    .transformer(/* your transformer */)
    .consumer(CsvConsumer::new("output.csv", CsvWriteConfig::default())?);

pipeline.run().await?;
```

### Read CSV File

```rust
use streamweave_csv::{CsvProducer, CsvReadConfig};

let producer = CsvProducer::new("data.csv", CsvReadConfig::default())?;
// Reads CSV file row by row, producing records
```

### Write CSV File

```rust
use streamweave_csv::{CsvConsumer, CsvWriteConfig};

let consumer = CsvConsumer::new("output.csv", CsvWriteConfig::default())?;
// Writes records to CSV file
```

## 📖 API Overview

### CsvProducer

Reads CSV files row by row:

```rust
pub struct CsvProducer {
    // Internal state
}
```

**Key Methods:**
- `new(path, config)` - Create producer for CSV file
- `produce()` - Generate stream from CSV file

### CsvConsumer

Writes items to CSV files:

```rust
pub struct CsvConsumer {
    // Internal state
}
```

**Key Methods:**
- `new(path, config)` - Create consumer for CSV file
- `consume(stream)` - Write stream items to CSV file

### CsvReadConfig

Configuration for reading CSV files:

```rust
pub struct CsvReadConfig {
    pub has_headers: bool,
    pub delimiter: u8,
    pub flexible: bool,
    // ... other options
}
```

### CsvWriteConfig

Configuration for writing CSV files:

```rust
pub struct CsvWriteConfig {
    pub has_headers: bool,
    pub delimiter: u8,
    // ... other options
}
```

## 📚 Usage Examples

### Simple CSV Processing

Process CSV file:

```rust
use streamweave_csv::{CsvProducer, CsvConsumer, CsvReadConfig, CsvWriteConfig};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(CsvProducer::new("input.csv", CsvReadConfig::default())?)
    .consumer(CsvConsumer::new("output.csv", CsvWriteConfig::default())?);

pipeline.run().await?;
```

### CSV with Headers

Handle CSV files with headers:

```rust
use streamweave_csv::{CsvReadConfig, CsvWriteConfig};

let read_config = CsvReadConfig {
    has_headers: true,
    ..Default::default()
};

let write_config = CsvWriteConfig {
    has_headers: true,
    ..Default::default()
};

let producer = CsvProducer::new("data.csv", read_config)?;
let consumer = CsvConsumer::new("output.csv", write_config)?;
```

### Custom Delimiter

Use custom delimiter (e.g., tab-separated):

```rust
use streamweave_csv::{CsvReadConfig, CsvWriteConfig};

let read_config = CsvReadConfig {
    delimiter: b'\t',  // Tab delimiter
    has_headers: true,
    ..Default::default()
};

let producer = CsvProducer::new("data.tsv", read_config)?;
```

### Transform CSV Data

Transform CSV records:

```rust
use streamweave_csv::{CsvProducer, CsvConsumer, CsvReadConfig, CsvWriteConfig};
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;

let pipeline = PipelineBuilder::new()
    .producer(CsvProducer::new("input.csv", CsvReadConfig::default())?)
    .transformer(MapTransformer::new(|record: Vec<String>| {
        // Transform each CSV record
        record.into_iter().map(|field| field.to_uppercase()).collect()
    }))
    .consumer(CsvConsumer::new("output.csv", CsvWriteConfig::default())?);
```

### Large CSV Processing

Process large CSV files efficiently:

```rust
use streamweave_csv::{CsvProducer, CsvReadConfig};
use streamweave_pipeline::PipelineBuilder;

// CsvProducer reads row by row, memory efficient
let pipeline = PipelineBuilder::new()
    .producer(CsvProducer::new("large_file.csv", CsvReadConfig::default())?)
    .transformer(/* process each row */)
    .consumer(/* write results */);
```

## 🏗️ Architecture

CSV processing flow:

```
┌──────────┐
│ CSV File │───> CsvProducer ───> Stream<Record> ───> Transformer ───> Stream<Record> ───> CsvConsumer ───> CSV File
└──────────┘                                                                                                    └──────────┘
```

**CSV Flow:**
1. CsvProducer reads CSV file row by row
2. Records flow through transformers
3. CsvConsumer writes records to CSV file
4. Processing is streaming and memory efficient

## 🔧 Configuration

### Read Configuration

Configure CSV reading:

```rust
let config = CsvReadConfig {
    has_headers: true,
    delimiter: b',',
    flexible: false,
    // ... other options
};
```

### Write Configuration

Configure CSV writing:

```rust
let config = CsvWriteConfig {
    has_headers: true,
    delimiter: b',',
    // ... other options
};
```

## 🔍 Error Handling

CSV errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)  // Skip malformed rows
    .producer(CsvProducer::new("data.csv", CsvReadConfig::default())?)
    .consumer(CsvConsumer::new("output.csv", CsvWriteConfig::default())?);
```

## ⚡ Performance Considerations

- **Row-by-Row**: CSV files are read row by row for memory efficiency
- **Streaming**: Output is streamed, not buffered
- **Large Files**: Can handle CSV files larger than memory
- **Async I/O**: Non-blocking file operations

## 📝 Examples

For more examples, see:
- [CSV Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/file_formats)
- [Large CSV Processing](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-csv` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `csv` - CSV parsing library
- `serde` - Serialization support
- `tokio` - Async runtime
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

CSV processing is used for:

1. **Data Import/Export**: Import and export data in CSV format
2. **ETL Pipelines**: Extract, transform, load CSV data
3. **Data Transformation**: Transform CSV data
4. **Large File Processing**: Process large CSV files
5. **Data Analysis**: Process CSV data for analysis

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-csv)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/csv)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-file](../file/README.md) - File I/O
- [streamweave-jsonl](../jsonl/README.md) - JSON Lines format
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

