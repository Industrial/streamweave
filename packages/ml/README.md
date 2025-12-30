# StreamWeave ML Transformers

Machine learning inference transformers for StreamWeave pipelines.

This package provides transformers that integrate ML model inference into StreamWeave data processing pipelines. It includes support for ONNX Runtime and provides a trait-based abstraction for adding support for other ML frameworks.

## Features

- **Inference Transformer**: Single-item inference transformer
- **Batched Inference Transformer**: Efficient batch processing transformer
- **ONNX Runtime Backend**: Support for ONNX models
- **Hot-Swapping**: Zero-downtime model updates
- **Trait-Based Design**: Easy to add support for other ML frameworks

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
streamweave-ml = { path = "../packages/ml", features = ["onnx"] }
```

## Example

### Single-Item Inference

```rust
use streamweave::prelude::*;
use streamweave_ml_transformers::{OnnxBackend, InferenceTransformer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = OnnxBackend::new()?;
    backend.load_from_path("model.onnx").await?;

    let pipeline = Pipeline::new()
        .with_producer(ArrayProducer::new(vec![vec![1.0, 2.0], vec![3.0, 4.0]]))
        .with_transformer(InferenceTransformer::new(backend))
        .with_consumer(VecConsumer::new());

    let (_, consumer) = pipeline.run().await?;
    Ok(())
}
```

### Batched Inference

```rust
use streamweave::prelude::*;
use streamweave_ml_transformers::{BatchedInferenceTransformer, OnnxBackend};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = OnnxBackend::new()?;
    backend.load_from_path("model.onnx").await?;

    let transformer = BatchedInferenceTransformer::new(backend)
        .with_batch_size(32)
        .with_timeout(Duration::from_millis(100));

    // Use in pipeline...
    Ok(())
}
```

## Features

- `onnx`: Enable ONNX Runtime support (requires `ort` and `ndarray`)

## Backends

### ONNX Runtime

The `OnnxBackend` provides support for ONNX models:

```rust
use streamweave_ml_transformers::OnnxBackend;

let mut backend = OnnxBackend::new()?;
backend.load_from_path("model.onnx").await?;
```

### Custom Backends

Implement the `InferenceBackend` trait to add support for other ML frameworks:

```rust
use streamweave_ml_transformers::InferenceBackend;

struct MyBackend {
    // Your backend implementation
}

impl InferenceBackend for MyBackend {
    type Input = Vec<f32>;
    type Output = Vec<f32>;
    type Error = MyError;

    // Implement required methods...
}
```

## Hot-Swapping

Models can be hot-swapped at runtime without stopping the pipeline:

```rust
use streamweave_ml_transformers::{OnnxBackend, InferenceTransformer, swap_model};

let transformer = InferenceTransformer::new(backend);
let backend_ref = transformer.backend();

// Later, swap to a new model
let mut new_backend = OnnxBackend::new()?;
new_backend.load_from_path("new_model.onnx").await?;
swap_model(&backend_ref, new_backend).await?;
```

