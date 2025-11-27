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
#![warn(missing_doc_code_examples)]

pub mod consumer;
pub mod consumers;
pub mod error;
pub mod input;
pub mod message;
pub mod metrics;
pub mod offset;
pub mod output;
pub mod pipeline;
pub mod producer;
pub mod producers;
pub mod stateful_transformer;
pub mod transaction;
pub mod transformer;
pub mod transformers;
pub mod visualization;
pub mod window;

#[cfg(not(target_arch = "wasm32"))]
pub mod distributed;

#[cfg(all(not(target_arch = "wasm32"), feature = "http-server"))]
pub mod http_server;

#[cfg(feature = "sql")]
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
