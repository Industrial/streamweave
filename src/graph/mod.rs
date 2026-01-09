//! # Graph Module
//!
//! This module provides graph-based execution for StreamWeave pipelines.
//! Graphs allow you to create complex data flow topologies with multiple
//! producers, transformers, and consumers connected together.
//!
//! ## `Message<T>` Based Data Flow
//!
//! **All data flowing through graphs is automatically wrapped in `Message<T>`.** This ensures:
//! - **Message IDs**: Every item has a unique identifier for tracking
//! - **Metadata Preservation**: Message metadata (timestamps, source, headers) is preserved through transformations
//! - **Error Correlation**: Errors include message IDs for correlation
//! - **Zero-Copy Sharing**: In-process mode uses `Arc<Message<T>>` for efficient fan-out
//!
//! Nodes handle `Message<T>` wrapping and unwrapping internally:
//! - **ProducerNode**: Wraps producer output in `Message<T>` before sending
//! - **TransformerNode**: Unwraps `Message<T>` for transformation, wraps output back
//! - **ConsumerNode**: Unwraps `Message<T>` before passing to consumer
//!
//! ## Overview
//!
//! The graph module provides:
//! - Graph structure and builder for creating data flow graphs
//! - Node types for producers, transformers, and consumers
//! - Execution engine for running graphs
//! - Control flow nodes for complex data processing patterns
//! - Router nodes for distributing data across multiple paths
//!
//! ## Example
//!
//! ```rust,no_run
//! use crate::graph::{GraphBuilder, GraphExecution};
//! use crate::graph::nodes::{ProducerNode, TransformerNode, ConsumerNode};
//! use streamweave_array::ArrayProducer;
//! use crate::transformers::MapTransformer;
//! use crate::consumers::VecConsumer;
//!
//! // Create a graph - all data flows as Message<T>
//! let graph = GraphBuilder::new()
//!     .node(ProducerNode::from_producer(
//!         "source".to_string(),
//!         ArrayProducer::new([1, 2, 3, 4, 5]),
//!     ))?
//!     .node(TransformerNode::from_transformer(
//!         "double".to_string(),
//!         MapTransformer::new(|x: i32| x * 2),
//!     ))?
//!     .node(ConsumerNode::from_consumer(
//!         "sink".to_string(),
//!         VecConsumer::<i32>::new(),
//!     ))?
//!     .connect_by_name("source", "double")?
//!     .connect_by_name("double", "sink")?
//!     .build();
//!
//! // Execute the graph
//! let mut executor = graph.executor();
//! executor.start().await?;
//! tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//! executor.stop().await?;
//! ```

// Node types will be organized in the nodes submodule
pub mod channels;
pub mod connection;
pub mod execution;
#[allow(clippy::module_inception)]
pub mod graph;
pub mod http_server;
pub mod nodes;
pub mod router;
pub mod tcp_server;
pub mod traits;

// Supporting modules
pub mod batching;
pub mod compression;
pub mod serialization;
pub mod shared_memory_channel;
// Note: stateful and windowing modules are temporarily disabled due to circular dependencies
// pub mod stateful;
pub mod subgraph;
pub mod throughput;
// pub mod windowing;
pub mod zero_copy;

// Re-export commonly used types
pub use channels::*;
pub use connection::*;
pub use execution::*;
pub use graph::*;
pub use nodes::{ConsumerNode, ProducerNode, TransformerNode};
pub use router::*;
pub use traits::*;

pub use http_server::*;

// Re-export supporting modules
pub use batching::*;
pub use compression::*;
pub use serialization::*;
pub use shared_memory_channel::*;
// pub use stateful::*;
pub use subgraph::*;
pub use throughput::*;
// pub use windowing::*;
pub use zero_copy::{
  ArcPool, InPlaceTransform, MutateInPlace, ZeroCopyShare, ZeroCopyShareWeak, ZeroCopyTransformer,
};
