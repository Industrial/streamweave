# Getting Started with StreamWeave

Welcome to StreamWeave! This guide will help you get started with building composable, async data pipelines in Rust.

## Installation

Add StreamWeave to your `Cargo.toml`:

```toml
[dependencies]
streamweave = "0.1.0"
```

## Basic Example

Here's a simple example that doubles numbers in a stream:

```rust
use streamweave::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pipeline = Pipeline::new()
        .with_producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
        .with_transformer(MapTransformer::new(|x| x * 2))
        .with_consumer(VecConsumer::new());
    
    let result = pipeline.run().await?;
    println!("Result: {:?}", result);
    Ok(())
}
```

## Core Concepts

### Producers
Producers generate data streams. Examples include:
- `ArrayProducer`: Produces items from a vector
- `FileProducer`: Reads data from files
- `KafkaProducer`: Consumes from Kafka topics
- `DatabaseProducer`: Queries database tables

### Transformers
Transformers process and transform stream items:
- `MapTransformer`: Transform each item
- `FilterTransformer`: Filter items based on conditions
- `BatchTransformer`: Group items into batches
- `RateLimitTransformer`: Control throughput

### Consumers
Consumers consume stream data:
- `VecConsumer`: Collects items into a vector
- `FileConsumer`: Writes data to files
- `KafkaConsumer`: Produces to Kafka topics
- `ConsoleConsumer`: Prints items to console

## Next Steps

- Check out the [Examples](../examples/) directory for more complex use cases
- Read the [Architecture Overview](architecture.md) to understand the design
- Explore the [API Reference](../target/doc/streamweave/index.html)

