//! # StreamWeave
//!
//! StreamWeave is a composable, async, stream-first computation framework for Rust.
//! It provides a fluent API for building data processing pipelines with producers,
//! transformers, and consumers.
//!
//! ## Core Concepts
//!
//! - **Producers**: Generate data streams
//! - **Transformers**: Process and transform stream items
//! - **Consumers**: Consume stream data
//! - **Pipelines**: Compose producers, transformers, and consumers together
//!
//! ## Example
//!
//! ```rust
//! use streamweave::prelude::*;
//!
//! let pipeline = Pipeline::new()
//!     .with_producer(ArrayProducer::new(vec![1, 2, 3, 4, 5]))
//!     .with_transformer(MapTransformer::new(|x| x * 2))
//!     .with_consumer(VecConsumer::new());
//!
//! let result = pipeline.run().await?;
//! ```

#![warn(missing_docs)]

/// Consumer trait and related types.
pub mod consumer;
/// Built-in consumer implementations.
pub mod consumers;
/// Error handling types and strategies.
pub mod error;
/// Input type definitions.
pub mod input;
/// Message types for stream items.
pub mod message;
/// Metrics collection and reporting.
pub mod metrics;
/// Offset tracking for stream processing.
pub mod offset;
/// Output type definitions.
pub mod output;
/// Pipeline builder and execution.
pub mod pipeline;
/// Producer trait and related types.
pub mod producer;
/// Built-in producer implementations.
pub mod producers;
/// Stateful transformer support.
pub mod stateful_transformer;
/// Transaction management for exactly-once processing.
pub mod transaction;
/// Transformer trait and related types.
pub mod transformer;
/// Built-in transformer implementations.
pub mod transformers;
/// Visualization utilities for stream monitoring.
pub mod visualization;
/// Window-based processing utilities.
pub mod window;

/// Graph-based API for Flow-Based Programming patterns.
pub mod graph;

#[cfg(all(not(target_arch = "wasm32"), feature = "native"))]
/// Distributed processing support.
pub mod distributed;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
/// HTTP server integration for stream processing.
pub mod http_server;

#[cfg(feature = "sql")]
/// SQL query support for stream processing.
pub mod sql;

// Re-export commonly used types
pub use consumer::Consumer;
pub use error::{ErrorAction, ErrorContext, ErrorStrategy, StreamError};
pub use input::Input;
pub use message::{Message, MessageId, MessageMetadata};
pub use output::Output;
pub use pipeline::{Complete, Empty, HasProducer, HasTransformer, Pipeline};
pub use producer::Producer;
pub use stateful_transformer::{
  InMemoryStateStore, StateStore, StateStoreExt, StatefulTransformer,
};
pub use transformer::Transformer;

// Graph API re-exports
pub use graph::{
  BroadcastRouter, ConsumerNode, Graph, GraphBuilder, GraphExecution, GraphExecutor,
  KeyBasedRouter, MergeRouter, ProducerNode, RoundRobinRouter, SubgraphNode, TransformerNode,
};

// Feature-gated re-exports
#[cfg(feature = "file-formats")]
pub use {
  consumers::csv::csv_consumer::CsvConsumer, consumers::jsonl::jsonl_consumer::JsonlConsumer,
  consumers::msgpack::msgpack_consumer::MsgPackConsumer,
  consumers::parquet::parquet_consumer::ParquetConsumer, producers::csv::csv_producer::CsvProducer,
  producers::jsonl::jsonl_producer::JsonlProducer,
  producers::msgpack::msgpack_producer::MsgPackProducer,
  producers::parquet::parquet_producer::ParquetProducer,
};
