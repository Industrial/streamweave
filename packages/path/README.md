# streamweave-path

[![Crates.io](https://img.shields.io/crates/v/streamweave-path.svg)](https://crates.io/crates/streamweave-path)
[![Documentation](https://docs.rs/streamweave-path/badge.svg)](https://docs.rs/streamweave-path)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Path manipulation transformers for StreamWeave**  
*Transform file paths without filesystem operations.*

The `streamweave-path` package provides transformers for manipulating path strings. It uses Rust's standard library `Path` and `PathBuf` types for path manipulation without performing any filesystem operations.

## ✨ Key Features

- **Path Transformers**: Transform paths in streams
- **No Filesystem Operations**: Pure path string manipulation
- **Path Operations**: Extract filename, parent, normalize paths
- **Type-Safe**: Uses Rust's `PathBuf` type
- **Efficient**: No I/O overhead

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-path = "0.3.0"
```

## 🚀 Quick Start

### Extract Filenames

```rust
use streamweave_transformers::FileNameTransformer;
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(/* produce paths */)
    .transformer(FileNameTransformer::new())
    .consumer(/* consume filenames */);

pipeline.run().await?;
```

### Get Parent Directories

```rust
use streamweave_transformers::ParentTransformer;

let transformer = ParentTransformer::new();
// Transforms paths to their parent directories
```

### Normalize Paths

```rust
use streamweave_transformers::NormalizeTransformer;

let transformer = NormalizeTransformer::new();
// Normalizes paths (removes . and .. components)
```

## 📖 API Overview

### Path Transformers

**FileNameTransformer:**
- Extracts filename from path
- Input: `PathBuf`, Output: `String`

**ParentTransformer:**
- Gets parent directory of path
- Input: `PathBuf`, Output: `PathBuf`

**NormalizeTransformer:**
- Normalizes path (removes . and ..)
- Input: `PathBuf`, Output: `PathBuf`

## 📚 Usage Examples

### Extract Filenames from Paths

```rust
use streamweave_transformers::FileNameTransformer;
use streamweave_pipeline::PipelineBuilder;
use streamweave_fs::DirectoryProducer;

let pipeline = PipelineBuilder::new()
    .producer(DirectoryProducer::new("data/")?)
    .transformer(FileNameTransformer::new())
    .consumer(/* consume filenames */);
```

### Get Parent Directories

```rust
use streamweave_transformers::ParentTransformer;

let transformer = ParentTransformer::new();
// /path/to/file.txt -> /path/to
```

### Normalize Paths

```rust
use streamweave_transformers::NormalizeTransformer;

let transformer = NormalizeTransformer::new();
// /path/./to/../file.txt -> /path/file.txt
```

### Chain Path Transformations

```rust
use streamweave_path::{ParentTransformer, FileNameTransformer};

let pipeline = PipelineBuilder::new()
    .producer(/* produce paths */)
    .transformer(ParentTransformer::new())
    .transformer(FileNameTransformer::new())
    .consumer(/* consume results */);
```

## 🏗️ Architecture

Path transformation:

```text
┌──────────┐
│  Paths   │───> PathTransformer ───> Stream ───> Transformed Paths
└──────────┘
```

**Path Flow:**
1. Paths flow into transformer
2. Transformer manipulates paths
3. Transformed paths flow out
4. No filesystem operations performed

## 🔧 Configuration

### Transformer Configuration

Configure path transformers:

```rust
let transformer = FileNameTransformer::new()
    .with_config(TransformerConfig::default()
        .with_name("filename-extractor".to_string()));
```

## 🔍 Error Handling

Path transformation errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(/* producer */)
    .transformer(FileNameTransformer::new())
    .consumer(/* consumer */);
```

## ⚡ Performance Considerations

- **No I/O**: Pure path string manipulation
- **Efficient**: Minimal overhead
- **Type-Safe**: Uses Rust's path types
- **Streaming**: Processes paths as streams

## 📝 Examples

For more examples, see:
- [Path Transformation Example](https://github.com/Industrial/streamweave/tree/main/examples)
- [File System Operations](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-path` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `futures` - Stream utilities
- `async-stream` - Stream generation

## 🎯 Use Cases

Path manipulation is used for:

1. **Path Processing**: Extract components from paths
2. **Path Normalization**: Normalize path strings
3. **File Organization**: Organize files by path components
4. **Path Filtering**: Filter paths by components
5. **Path Transformation**: Transform paths in pipelines

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-path)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/path)
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

