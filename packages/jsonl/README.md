# streamweave-jsonl

[![Crates.io](https://img.shields.io/crates/v/streamweave-jsonl.svg)](https://crates.io/crates/streamweave-jsonl)
[![Documentation](https://docs.rs/streamweave-jsonl/badge.svg)](https://docs.rs/streamweave-jsonl)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**JSON Lines format support for StreamWeave**  
*Read and write JSONL files with streaming JSON processing.*

The `streamweave-jsonl` package provides JSON Lines (JSONL) producers and consumers for StreamWeave. JSONL is a format where each line is a valid JSON object, enabling streaming processing of large JSON datasets.

## ✨ Key Features

- **JsonlProducer**: Read JSONL files line by line
- **JsonlConsumer**: Write items to JSONL files
- **Streaming JSON**: Process JSON objects as streams
- **Type-Safe**: Use serde for type-safe JSON parsing
- **Large File Support**: Handle large JSONL files efficiently

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-jsonl = "0.6.0"
```

## 🚀 Quick Start

### Read and Process JSONL

```rust
use streamweave_jsonl::{JsonlProducer, JsonlConsumer, JsonlWriteConfig};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(JsonlProducer::new("data.jsonl")?)
    .transformer(/* your transformer */)
    .consumer(JsonlConsumer::new("output.jsonl", JsonlWriteConfig::default())?);

pipeline.run().await?;
```

### Read JSONL File

```rust
use streamweave_jsonl::JsonlProducer;

let producer = JsonlProducer::new("data.jsonl")?;
// Reads JSONL file line by line, producing JSON objects
```

### Write JSONL File

```rust
use streamweave_jsonl::{JsonlConsumer, JsonlWriteConfig};

let consumer = JsonlConsumer::new("output.jsonl", JsonlWriteConfig::default())?;
// Writes JSON objects to JSONL file
```

## 📖 API Overview

### JsonlProducer

Reads JSONL files line by line:

```rust
pub struct JsonlProducer {
    // Internal state
}
```

**Key Methods:**
- `new(path)` - Create producer for JSONL file
- `produce()` - Generate stream from JSONL file

### JsonlConsumer

Writes items to JSONL files:

```rust
pub struct JsonlConsumer {
    // Internal state
}
```

**Key Methods:**
- `new(path, config)` - Create consumer for JSONL file
- `consume(stream)` - Write stream items to JSONL file

### JsonlWriteConfig

Configuration for writing JSONL files:

```rust
pub struct JsonlWriteConfig {
    pub pretty: bool,  // Pretty-print JSON
    // ... other options
}
```

## 📚 Usage Examples

### Simple JSONL Processing

Process JSONL file:

```rust
use streamweave_jsonl::{JsonlProducer, JsonlConsumer, JsonlWriteConfig};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(JsonlProducer::new("input.jsonl")?)
    .consumer(JsonlConsumer::new("output.jsonl", JsonlWriteConfig::default())?);

pipeline.run().await?;
```

### Type-Safe JSON Parsing

Parse JSON to typed structs:

```rust
use serde::{Deserialize, Serialize};
use streamweave_jsonl::JsonlProducer;
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;

#[derive(Deserialize, Serialize)]
struct Record {
    id: u64,
    name: String,
    value: f64,
}

let pipeline = PipelineBuilder::new()
    .producer(JsonlProducer::new("data.jsonl")?)
    .transformer(MapTransformer::new(|json: serde_json::Value| {
        serde_json::from_value::<Record>(json).unwrap()
    }))
    .consumer(/* process typed records */);
```

### Transform JSON Objects

Transform JSON data:

```rust
use streamweave_jsonl::{JsonlProducer, JsonlConsumer, JsonlWriteConfig};
use streamweave_pipeline::PipelineBuilder;
use streamweave_transformers::MapTransformer;

let pipeline = PipelineBuilder::new()
    .producer(JsonlProducer::new("input.jsonl")?)
    .transformer(MapTransformer::new(|mut json: serde_json::Value| {
        // Transform JSON object
        if let Some(obj) = json.as_object_mut() {
            obj.insert("processed".to_string(), serde_json::Value::Bool(true));
        }
        json
    }))
    .consumer(JsonlConsumer::new("output.jsonl", JsonlWriteConfig::default())?);
```

### Pretty-Print JSONL

Write pretty-printed JSON:

```rust
use streamweave_jsonl::{JsonlConsumer, JsonlWriteConfig};

let config = JsonlWriteConfig {
    pretty: true,  // Pretty-print each JSON object
    ..Default::default()
};

let consumer = JsonlConsumer::new("output.jsonl", config)?;
```

### Large JSONL Processing

Process large JSONL files efficiently:

```rust
use streamweave_jsonl::JsonlProducer;
use streamweave_pipeline::PipelineBuilder;

// JsonlProducer reads line by line, memory efficient
let pipeline = PipelineBuilder::new()
    .producer(JsonlProducer::new("large_file.jsonl")?)
    .transformer(/* process each JSON object */)
    .consumer(/* write results */);
```

## 🏗️ Architecture

JSONL processing flow:

```text
┌────────────┐
│ JSONL File │───> JsonlProducer ───> Stream<JSON> ───> Transformer ───> Stream<JSON> ───> JsonlConsumer ───> JSONL File
└────────────┘                                                                                                    └────────────┘
```

**JSONL Flow:**
1. JsonlProducer reads JSONL file line by line
2. JSON objects flow through transformers
3. JsonlConsumer writes JSON objects to JSONL file
4. Processing is streaming and memory efficient

## 🔧 Configuration

### Write Configuration

Configure JSONL writing:

```rust
let config = JsonlWriteConfig {
    pretty: false,  // Compact JSON (default)
    // ... other options
};
```

## 🔍 Error Handling

JSONL errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)  // Skip malformed JSON lines
    .producer(JsonlProducer::new("data.jsonl")?)
    .consumer(JsonlConsumer::new("output.jsonl", JsonlWriteConfig::default())?);
```

## ⚡ Performance Considerations

- **Line-by-Line**: JSONL files are read line by line for memory efficiency
- **Streaming**: Output is streamed, not buffered
- **Large Files**: Can handle JSONL files larger than memory
- **Async I/O**: Non-blocking file operations

## 📝 Examples

For more examples, see:
- [JSONL Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/file_formats)
- [Large JSONL Processing](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-jsonl` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `serde` - Serialization support
- `serde_json` - JSON serialization
- `tokio` - Async runtime
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

JSONL processing is used for:

1. **Data Import/Export**: Import and export data in JSONL format
2. **ETL Pipelines**: Extract, transform, load JSONL data
3. **Data Transformation**: Transform JSON data
4. **Large File Processing**: Process large JSON datasets
5. **Streaming JSON**: Process JSON objects as streams

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-jsonl)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/jsonl)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-file](../file/README.md) - File I/O
- [streamweave-csv](../csv/README.md) - CSV format
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

