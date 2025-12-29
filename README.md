# StreamWeave

[![Crates.io](https://img.shields.io/crates/v/streamweave.svg)](https://crates.io/crates/streamweave)
[![Documentation](https://docs.rs/streamweave/badge.svg)](https://docs.rs/streamweave)
[![CI](https://github.com/Industrial/streamweave/workflows/CI/badge.svg)](https://github.com/Industrial/streamweave/actions)
[![codecov](https://codecov.io/gh/Industrial/streamweave/graph/badge.svg)](https://codecov.io/gh/Industrial/streamweave)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Composable, async, stream-first computation in pure Rust**  
*Build fully composable, async data pipelines using a fluent API.*

StreamWeave is a general-purpose Rust framework built around the concept of
**streaming data**, with a focus on simplicity, composability, and performance.

## ✨ Key Features

### ✅ Implemented

- Pure Rust API with zero-cost abstractions
- Full async/await compatibility via `futures::Stream`
- Fluent pipeline-style API with type-safe builder pattern
- **Graph-based API** for complex topologies with fan-in/fan-out patterns
- **Flow-Based Programming (FBP)** patterns with type-safe routing
- **Multiple routing strategies**: Broadcast, Round-Robin, Merge, Key-Based
- Comprehensive error handling system with multiple strategies (Stop, Skip, Retry, Custom)
- Code-as-configuration — no external DSLs
- Comprehensive test infrastructure
- **Integration Examples**: Kafka, Redis Streams, Database (PostgreSQL/MySQL/SQLite), HTTP Polling
- **File Format Support**: CSV, JSONL, Parquet with streaming parsing
- **Stateful Processing**: RunningSum, MovingAverage transformers
- **Exactly-Once Processing**: Message deduplication with configurable windows
- **Windowing Operations**: Tumbling, sliding, and count-based windows
- **Advanced Transformers**: CircuitBreaker, Retry, Batch, RateLimit
- **Common Transformers**: Map, Filter, Flatten, Reduce, and many more
- HTTP middleware support with Axum integration
- **HTTP Graph Server**: Long-lived graph-based HTTP servers with path-based routing
- WebSocket support
- Server-Sent Events support

### 🚧 Planned

- Support distributed processing
- Additional specialized transformers and utilities
- Reusable pipeline components
- Add machine learning integration
- Implement monitoring and metrics
- Add SQL-like querying
- Graph visualization tools

## 📦 Core Concepts

StreamWeave breaks computation into **three primary building blocks**:

| Component       | Description                                |
| --------------- | ------------------------------------------ |
| **Producer**    | Starts a stream of data                    |
| **Transformer** | Transforms stream items (e.g., map/filter) |
| **Consumer**    | Consumes the stream, e.g. writing, logging |

All components can be chained together fluently. These components can be used in both the **Pipeline API** (for simple linear flows) and the **Graph API** (for complex topologies with fan-in/fan-out patterns).

## 🔀 Pipeline vs Graph API

StreamWeave provides two APIs for building data processing workflows:

| Feature | Pipeline API | Graph API |
|---------|-------------|-----------|
| **Use Case** | Simple linear flows | Complex topologies |
| **Topology** | Single path: Producer → Transformer → Consumer | Multiple paths, fan-in/fan-out |
| **Routing** | Sequential processing | Configurable routing strategies |
| **Complexity** | Lower complexity, easier to use | Higher flexibility, more powerful |
| **Best For** | ETL pipelines, simple transformations | Complex workflows, parallel processing, data distribution |

**When to use Pipeline API:**
- Simple linear data flows
- Single transformation path
- Quick prototyping
- Straightforward ETL operations

**When to use Graph API:**
- Multiple data paths
- Fan-out (one source to many destinations)
- Fan-in (many sources to one destination)
- Complex routing requirements
- Parallel processing needs
- Flow-Based Programming (FBP) patterns

## 🔄 Example Pipeline

### ✅ Currently Possible

```rust
use streamweave::{
    consumers::console::console_consumer::ConsoleConsumer,
    pipeline::PipelineBuilder,
    producers::range::range_producer::RangeProducer,
    transformers::map::map_transformer::MapTransformer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a pipeline that:
    // 1. Produces numbers from 1 to 5
    // 2. Doubles each number
    // 3. Prints the result to the console
    let pipeline = PipelineBuilder::new()
        .producer(RangeProducer::new(1, 6, 1))
        .transformer(MapTransformer::new(|x: i32| x * 2))
        .consumer(ConsoleConsumer::new());

    // Run the pipeline
    pipeline.run().await?;
    Ok(())
}
```

## 🕸️ Example Graph

### ✅ Currently Possible

```rust
use streamweave::graph::{GraphBuilder, ProducerNode, TransformerNode, ConsumerNode};
use streamweave::producers::vec::VecProducer;
use streamweave::transformers::map::MapTransformer;
use streamweave::consumers::vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a graph that:
    // 1. Produces numbers from a vector
    // 2. Doubles each number
    // 3. Collects the results
    let graph = GraphBuilder::new()
        .add_node(
            "source".to_string(),
            ProducerNode::from_producer(
                "source".to_string(),
                VecProducer::new(vec![1, 2, 3, 4, 5]),
            ),
        )?
        .add_node(
            "transform".to_string(),
            TransformerNode::from_transformer(
                "transform".to_string(),
                MapTransformer::new(|x: i32| x * 2),
            ),
        )?
        .add_node(
            "sink".to_string(),
            ConsumerNode::from_consumer(
                "sink".to_string(),
                VecConsumer::new(),
            ),
        )?
        .connect_by_name("source", "transform")?
        .connect_by_name("transform", "sink")?
        .build();

    // Execute the graph
    let mut executor = graph.executor();
    executor.start().await?;
    // Graph runs asynchronously - use stop() when done
    executor.stop().await?;
    Ok(())
}
```

### Fan-Out Example

The Graph API excels at fan-out patterns where one source feeds multiple destinations:

```rust
use streamweave::graph::{GraphBuilder, ProducerNode, TransformerNode, ConsumerNode};
use streamweave::producers::vec::VecProducer;
use streamweave::transformers::map::MapTransformer;
use streamweave::consumers::vec::VecConsumer;

// Create a producer that feeds multiple transformers (fan-out)
let producer = ProducerNode::from_producer(
    "source".to_string(),
    VecProducer::new(vec![1, 2, 3]),
);

// Multiple transformers process the same data
let double = TransformerNode::from_transformer(
    "double".to_string(),
    MapTransformer::new(|x: i32| x * 2),
);
let triple = TransformerNode::from_transformer(
    "triple".to_string(),
    MapTransformer::new(|x: i32| x * 3),
);

// Connect producer to both transformers (fan-out)
let graph = GraphBuilder::new()
    .add_node("source".to_string(), producer)?
    .add_node("double".to_string(), double)?
    .add_node("triple".to_string(), triple)?
    .connect_by_name("source", "double")?
    .connect_by_name("source", "triple")?
    .build();
```

For advanced patterns including fan-in, custom routing strategies (Broadcast, Round-Robin, Merge, Key-Based), subgraphs, and stateful nodes, see [GRAPH.md](GRAPH.md).

### HTTP Graph Server Example

StreamWeave enables building HTTP servers where all traffic flows through a graph. Requests are routed to different handlers based on path patterns, and responses flow back through the graph:

```rust
use streamweave::http_server::{
    HttpGraphServer, HttpGraphServerConfig,
    LongLivedHttpRequestProducer, HttpRequestProducerConfig,
    HttpResponseCorrelationConsumer,
    transformers::{PathBasedRouterTransformer, PathRouterConfig, RoutePattern},
};
use streamweave::graph::{GraphBuilder, ProducerNode, TransformerNode, ConsumerNode};
use axum::{Router, routing::get};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channel for HTTP requests
    let (request_sender, request_receiver) = tokio::sync::mpsc::channel(100);
    
    // Create HTTP request producer (injects requests into graph)
    let http_producer = LongLivedHttpRequestProducer::new(
        request_receiver,
        HttpRequestProducerConfig::default(),
    );
    
    // Create path-based router (routes by URL path)
    let router = PathBasedRouterTransformer::new(PathRouterConfig {
        routes: vec![
            RoutePattern {
                pattern: "/api/rest/*".to_string(),
                port: 0, // Route to REST handler
            },
            RoutePattern {
                pattern: "/api/graphql".to_string(),
                port: 1, // Route to GraphQL handler
            },
        ],
        default_port: Some(2), // Default/404 handler
    });
    
    // Create response correlation consumer (matches responses to requests)
    let response_consumer = HttpResponseCorrelationConsumer::with_timeout(
        std::time::Duration::from_secs(30),
    );
    
    // Build the graph
    let graph = GraphBuilder::new()
        .add_node(
            "http_producer".to_string(),
            ProducerNode::from_producer("http_producer".to_string(), http_producer),
        )?
        .add_node(
            "router".to_string(),
            TransformerNode::from_transformer("router".to_string(), router),
        )?
        // Add your handler nodes here (REST, GraphQL, RPC, Static, etc.)
        .add_node(
            "response_consumer".to_string(),
            ConsumerNode::from_consumer("response_consumer".to_string(), response_consumer),
        )?
        .connect_by_name("http_producer", "router")?
        // Connect router outputs to your handlers
        // Connect handler outputs to response_consumer
        .build();
    
    // Create the HTTP Graph Server
    let (server, _) = HttpGraphServer::new(
        graph,
        HttpGraphServerConfig::default(),
    ).await?;
    
    // Start the graph executor
    server.start().await?;
    
    // Create Axum handler
    let handler = server.create_handler();
    
    // Build Axum router
    let app = Router::new()
        .route("/*path", get(handler));
    
    // Start Axum server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

The HTTP Graph Server:
- Maintains a long-lived graph executor
- Injects HTTP requests into the graph as `Message<HttpRequest>` items
- Routes requests by path using `PathBasedRouterTransformer`
- Correlates responses with requests using `request_id`
- Handles request timeouts and errors gracefully

## 🧱 API Overview

### ✅ Implemented Pipeline Construction

```rust
PipelineBuilder::new()
    .producer(...)    // Add data source
    .transformer(...) // Add transformation
    .consumer(...)    // Add data sink
    .run()           // Execute pipeline
```

### ✅ Implemented Graph Construction

```rust
GraphBuilder::new()
    .add_node("name", ProducerNode::from_producer(...))    // Add producer node
    .add_node("name", TransformerNode::from_transformer(...)) // Add transformer node
    .add_node("name", ConsumerNode::from_consumer(...))    // Add consumer node
    .connect_by_name("source", "target")?  // Connect nodes
    .build()                                // Build graph
    .executor()                             // Get executor
    .start().await?                         // Execute graph
```

### ✅ Error Handling

StreamWeave provides two levels of error handling:

1. **Pipeline Level**
```rust
// Default behavior: Pipeline stops on first error
pipeline.run().await?;

// Configure pipeline-wide error handling
pipeline
    .with_error_strategy(ErrorStrategy::Stop)  // Default
    .with_error_strategy(ErrorStrategy::Skip)  // Skip errored items
    .with_error_strategy(ErrorStrategy::Retry(3))  // Retry 3 times
    .run()
    .await?;
```

2. **Component Level**
```rust
// Override error handling for specific components
MapTransformer::new(parse)
    .with_error_strategy(ErrorStrategy::Stop)      // Stop component and pipeline
    .with_error_strategy(ErrorStrategy::Skip)      // Skip errors, continue processing
    .with_error_strategy(ErrorStrategy::Retry(3))  // Retry operation 3 times
    .with_error_strategy(ErrorStrategy::Custom(|err| {
        // Custom error handling logic
        ErrorAction::Skip
    }));
```

## 🚀 Getting Started

### Installation

Add StreamWeave to your `Cargo.toml`:

```toml
[dependencies]
streamweave = "0.2.2"
```

### Basic Usage (Pipeline API)

```rust
use streamweave::{
    consumers::vec::vec_consumer::VecConsumer,
    pipeline::PipelineBuilder,
    producers::array::array_producer::ArrayProducer,
    transformers::map::map_transformer::MapTransformer,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let producer = ArrayProducer::new(vec![1, 2, 3, 4, 5]);
    let transformer = MapTransformer::new(|x: i32| x * 2);
    let consumer = VecConsumer::new();

    let pipeline = PipelineBuilder::new()
        .producer(producer)
        .transformer(transformer)
        .consumer(consumer);

    let ((), result) = pipeline.run().await?;
    println!("Result: {:?}", result.collected);
    Ok(())
}
```

### Using the Graph API

```rust
use streamweave::graph::{GraphBuilder, ProducerNode, TransformerNode, ConsumerNode};
use streamweave::producers::array::ArrayProducer;
use streamweave::transformers::map::MapTransformer;
use streamweave::consumers::vec::VecConsumer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph = GraphBuilder::new()
        .add_node(
            "source".to_string(),
            ProducerNode::from_producer(
                "source".to_string(),
                ArrayProducer::new(vec![1, 2, 3, 4, 5]),
            ),
        )?
        .add_node(
            "transform".to_string(),
            TransformerNode::from_transformer(
                "transform".to_string(),
                MapTransformer::new(|x: i32| x * 2),
            ),
        )?
        .add_node(
            "sink".to_string(),
            ConsumerNode::from_consumer(
                "sink".to_string(),
                VecConsumer::new(),
            ),
        )?
        .connect_by_name("source", "transform")?
        .connect_by_name("transform", "sink")?
        .build();

    let mut executor = graph.executor();
    executor.start().await?;
    // Graph runs asynchronously - use stop() when done
    executor.stop().await?;
    Ok(())
}
```

## 🧪 Testing Pipelines

### ✅ Implemented

The framework includes comprehensive test infrastructure for unit testing pipelines and components:

```rust
// Example from the test suite
let producer = NumberProducer { numbers: vec![1, 2, 3] };
let transformer = StringifyTransformer;
let consumer = CollectConsumer { collected: Vec::new() };

let (_, consumer) = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer)
    .run()
    .await
    .unwrap();

assert_eq!(consumer.collected, vec!["1", "2", "3"]);
```

## 📚 Documentation

- [API Documentation](https://docs.rs/streamweave)
- [Local Documentation](target/doc/streamweave/index.html) - Generated with rustdoc (run `./bin/docs`)
- [Graph API Guide](GRAPH.md) - Advanced graph patterns, routing strategies, and Flow-Based Programming
- [Getting Started Guide](docs/getting_started.md)
- [Architecture Overview](docs/architecture.md)
- [Common Use Cases](docs/guides/common_use_cases.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Contributing Guide](https://github.com/yourusername/streamweave/blob/main/CONTRIBUTING.md)

## 📖 Examples

StreamWeave includes comprehensive examples demonstrating all major features:

### Integration Examples
- **[Kafka Integration](examples/kafka_integration/)** - Produce to and consume from Kafka topics
- **[Redis Streams Integration](examples/redis_integration/)** - XADD and XREAD operations with consumer groups
- **[Database Integration](examples/database_integration/)** - Query PostgreSQL, MySQL, and SQLite with streaming results
- **[HTTP Polling Integration](examples/http_poll_integration/)** - Poll HTTP endpoints with pagination, delta detection, and rate limiting
- **[HTTP Graph Server](examples/http_graph_server/)** - Long-lived graph-based HTTP servers with path-based routing

### File Format Examples
- **[File Formats](examples/file_formats/)** - CSV, JSONL, and Parquet read/write with streaming parsing

### Processing Examples
- **[Stateful Processing](examples/stateful_processing/)** - RunningSum and MovingAverage transformers
- **[Error Handling](examples/error_handling/)** - Stop, Skip, Retry, and Custom error strategies
- **[Advanced Transformers](examples/advanced_transformers/)** - CircuitBreaker, Retry, Batch, RateLimit
- **[Windowing Operations](examples/windowing/)** - Tumbling, sliding, and count-based windows
- **[Exactly-Once Processing](examples/exactly_once/)** - Message deduplication and checkpointing

### Basic Examples
- **[Basic Pipeline](examples/basic_pipeline/)** - Simple pipeline example
- **[Advanced Pipeline](examples/advanced_pipeline/)** - Complex pipeline patterns

### Visualization Examples

- **[Basic Visualization](examples/visualization_basic/)** - Console export with DOT and JSON formats
- **[HTML Visualization](examples/visualization_html/)** - Web visualization with browser auto-open
- **[Graph Visualization](examples/visualization_graph/)** - Graph API visualization with fan-out and fan-in patterns
- **[Graphviz Image Generation](examples/visualization_dot_image/)** - Generate PNG, SVG, PDF, and JPG images from DAGs
- **[Metrics Visualization](examples/visualization_metrics/)** - Real-time metrics collection and visualization
- **[Debug Visualization](examples/visualization_debug/)** - Debug mode with breakpoint support
- **[Visualization Server](examples/visualization_server/)** - HTTP server for serving visualizations (requires `http-server` feature)

### Graph API Examples
- **[Graph Architecture](GRAPH.md)** - Comprehensive guide to the Graph API with examples
- Graph API examples demonstrate fan-in/fan-out patterns, routing strategies, and complex topologies

Run any example with:
```bash
cargo run --example <example_name> --features <required_features>
```

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/).

See [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- HTTP support powered by [Axum](https://github.com/tokio-rs/axum)
- Inspired by reactive programming patterns and stream processing frameworks